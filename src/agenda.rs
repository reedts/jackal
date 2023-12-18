use chrono::{DateTime, Datelike, Duration, Local, Month, NaiveDate, NaiveDateTime, TimeZone, Utc};
use log;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;
use std::ops::{Bound, RangeBounds};

use crate::config::Config;
use crate::provider::datetime::days_of_month;
use crate::provider::ical;
use crate::provider::tz::*;
use crate::provider::Uid;
use crate::provider::{
    Alarm, EventFilter, MutCalendarlike, Occurrence, ProviderCalendar, Result, TimeSpan,
};

#[derive(Default)]
struct OccurrenceCache {
    occurrences: BTreeMap<Uid, Vec<TimeSpan<Utc>>>,
    events: BTreeMap<NaiveDate, HashSet<Uid>>,
}

struct CacheLine<'cache>(&'cache Uid, &'cache [TimeSpan<Utc>]);

impl OccurrenceCache {
    pub fn add<'occ, I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Occurrence<'occ>>,
    {
        for occ in iter {
            let first_day = occ.begin().date_naive();
            let last_day = occ.end().date_naive();

            for day in first_day.iter_days().take_while(|dt| dt <= &last_day) {
                self.events
                    .entry(day)
                    .or_default()
                    .insert(occ.event.uid().to_string());
            }

            self.occurrences
                .entry(occ.event.uid().to_string())
                .or_default()
                .push(occ.span.with_tz(&Utc))
        }
    }

    pub fn contains(&self, date: &NaiveDate) -> bool {
        self.events.contains_key(date)
    }

    pub fn fetch_range<'cache>(
        &'cache self,
        range: impl RangeBounds<NaiveDate>,
    ) -> impl Iterator<Item = CacheLine<'cache>> + 'cache {
        let uids = self.events.range(range).flat_map(|(_, uids)| uids.iter());

        uids.into_iter()
            .map(move |uid| CacheLine(uid, &self.occurrences.get(uid).unwrap()))
    }

    pub fn fetch<'cache>(
        &'cache self,
        date: &NaiveDate,
    ) -> impl Iterator<Item = CacheLine<'cache>> + 'cache {
        self.events
            .get(&date)
            .unwrap()
            .iter()
            .map(move |uid| CacheLine(uid, self.occurrences.get(uid.as_str()).unwrap().as_slice()))
    }

    pub fn remove(&mut self, date: &NaiveDate) {
        let uids = self.events.get(date);

        match uids {
            Some(uids) => {
                for uid in uids {
                    self.occurrences.remove(uid);
                }
            }
            None => (),
        }

        self.events.remove(date);
    }
}

pub struct Agenda {
    calendars: BTreeMap<String, ProviderCalendar>,
    // By using RefCell we can mutate our cache even when
    // used with a shared reference
    occurrence_cache: RefCell<OccurrenceCache>,
    _tz_transition_cache: &'static TzTransitionCache,
}

impl Agenda {
    pub fn from_config(
        config: &Config,
        event_sink: &std::sync::mpsc::Sender<crate::events::Event>,
    ) -> Result<Self> {
        let _tz_transition_cache: &'static TzTransitionCache = Box::leak(Box::default());

        let calendars: BTreeMap<String, ProviderCalendar> = config
            .collections
            .iter()
            .filter_map(|collection_spec| {
                if collection_spec.provider == "ical" {
                    Some(ical::from_dir(
                        collection_spec.path.as_path(),
                        collection_spec.calendars.as_slice(),
                        _tz_transition_cache,
                        event_sink,
                    ))
                } else {
                    None
                }
            })
            .inspect(|c| {
                if let Err(e) = c {
                    log::error!("{}", e)
                }
            })
            .filter_map(Result::ok)
            .flat_map(|calendars| {
                calendars
                    .into_iter()
                    .map(|cal| (cal.name().to_owned(), cal))
            })
            .collect();

        Ok(Agenda {
            calendars,
            occurrence_cache: RefCell::default(),
            _tz_transition_cache,
        })
    }

    fn fetch_maybe_cached<'a>(
        &'a self,
        range: impl RangeBounds<NaiveDateTime> + 'a + Clone,
    ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        let start = match range.start_bound() {
            Bound::Included(t) | Bound::Excluded(t) => Some(t),
            Unbounded => None,
        };

        let end = match range.start_bound() {
            Bound::Included(t) | Bound::Excluded(t) => Some(t),
            Unbounded => None,
        };

        if let (Some(start), Some(end)) = (start, end) {
            // Add to cache if not already cached
            let mut cache = self.occurrence_cache.borrow_mut();

            let begin_date = start.date();
            let end_date = end.date();

            for day in begin_date
                .iter_days()
                .take_while(|dt| dt <= &end_date)
                .filter(|dt| !cache.contains(dt))
            {
                self.add_to_cache(day);
            }

            let cached_events = cache.fetch_range(begin_date..=end_date);
        } else {
            self.fetch_no_cache(range.clone())
        }
    }

    fn fetch_no_cache<'a>(
        &'a self,
        range: impl RangeBounds<NaiveDateTime> + 'a + Clone,
    ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        self.calendars.values().flat_map(move |calendar| {
            calendar
                .as_calendar()
                .filter_events(EventFilter::default().datetime_range(range.clone()))
        })
    }

    fn add_to_cache(&self, date: NaiveDate) {
        let mut cache = self.occurrence_cache.borrow_mut();

        let begin = date.and_hms_opt(0, 0, 0).unwrap();
        let end = (date + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();

        if cache.contains(&date) {
            cache.remove(&date);
        }

        let occurrences = self.calendars.iter().flat_map(move |(name, calendar)| {
            calendar
                .as_calendar()
                .filter_events(EventFilter::default().datetime_range(begin..end))
                .into_iter()
        });

        cache.add(occurrences);
    }

    /// Note, even though events are sorted within one calendar, they are not sorted in the
    /// resulting iterator since multiple calendars are merged
    pub fn events_in<'a>(
        &'a self,
        range: impl RangeBounds<NaiveDateTime> + 'a + Clone,
    ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        self.calendars.values().flat_map(move |calendar| {
            calendar
                .as_calendar()
                .filter_events(EventFilter::default().datetime_range(range.clone()))
        })
    }

    pub fn events_of_month<'a>(
        &'a self,
        month: Month,
        year: i32,
    ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        let begin = NaiveDate::from_ymd_opt(year, month.number_from_month() as u32, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let end = begin + Duration::days(days_of_month(&month, year) as i64);

        self.events_in(begin..end)
    }

    pub fn _events_of_current_month<'a>(&'a self) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        let today = Utc::now().date_naive();
        let curr_month = Month::from_u32(today.month()).unwrap();
        let curr_year = today.year();

        self.events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day<'a>(
        &'a self,
        date: &NaiveDate,
    ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        let begin = date.and_hms_opt(0, 0, 0).unwrap();
        let end = begin + Duration::days(1);

        self.calendars.values().flat_map(move |calendar| {
            calendar
                .as_calendar()
                .filter_events(EventFilter::default().datetime_range(begin..end))
        })
    }

    pub fn _events_of_current_day<'a>(&'a self) -> impl Iterator<Item = Occurrence<'a>> + 'a {
        let today = Utc::now().date_naive();

        self.events_of_day(&today)
    }

    pub fn alarms_in<'a>(
        &'a self,
        range: impl std::ops::RangeBounds<NaiveDateTime> + 'a + Clone,
    ) -> impl Iterator<Item = Alarm<'a, Tz>> {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
            Bound::Excluded(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
            _ => Bound::Unbounded,
        };
        let end = match range.end_bound() {
            Bound::Included(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
            Bound::Excluded(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
            _ => Bound::Unbounded,
        };

        self.calendars
            .values()
            .flat_map(move |calendar| calendar.as_calendar().alarms_in(start, end))
    }

    pub fn calendar_by_name_mut(&mut self, name: &str) -> Option<&mut dyn MutCalendarlike> {
        self.calendars.get_mut(name).and_then(|cal| match cal {
            ProviderCalendar::Ical(c) => Some(c as &mut dyn MutCalendarlike),
        })
    }

    pub fn process_external_modifications(&mut self) {
        for (_, c) in &mut self.calendars {
            c.process_external_modifications();
        }
    }
}
