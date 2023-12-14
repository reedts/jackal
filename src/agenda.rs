use chrono::{DateTime, Datelike, Duration, Local, Month, NaiveDate, NaiveDateTime, TimeZone, Utc};
use log;
use num_traits::FromPrimitive;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::iter::FromIterator;

use crate::config::Config;
use crate::provider::datetime::days_of_month;
use crate::provider::ical;
use crate::provider::tz::*;
use crate::provider::Uid;
use crate::provider::{
    Alarm, EventFilter, MutCalendarlike, Occurrence, ProviderCalendar, Result, TimeSpan,
};

struct CacheLine(TimeSpan<Local>, String, Uid);

type OccurrenceCache = BTreeMap<NaiveDate, Vec<CacheLine>>;

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

    // fn fetch_cached<'a>(
    //     &'a self,
    //     range: impl std::ops::RangeBounds<NaiveDateTime> + 'a + Clone,
    // ) -> impl Iterator<Item = Occurrence<'a>> + 'a {
    //     todo!();
    // }

    fn add_to_cache(&self, date: NaiveDate) {
        let mut cache = self.occurrence_cache.borrow_mut();

        let begin = date.and_hms_opt(0, 0, 0).unwrap();
        let end = (date + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();

        if cache.contains_key(&date) {
            cache.remove(&date);
        }

        let events = self.calendars.iter().flat_map(move |(name, calendar)| {
            calendar
                .as_calendar()
                .filter_events(EventFilter::default().datetime_range(begin..end))
                .into_iter()
                .zip(std::iter::repeat(name))
        });

        cache.insert(
            date,
            Vec::from_iter(events.map(|(occ, calendar): (Occurrence<'_>, &String)| {
                CacheLine(
                    occ.span.clone().with_tz(&Local),
                    calendar.to_string(),
                    occ.event.uid().to_string(),
                )
            })),
        );
    }

    /// Note, even though events are sorted within one calendar, they are not sorted in the
    /// resulting iterator since multiple calendars are merged
    pub fn events_in<'a>(
        &'a self,
        range: impl std::ops::RangeBounds<NaiveDateTime> + 'a + Clone,
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
