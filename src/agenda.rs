use chrono::{DateTime, Datelike, Duration, Month, NaiveDate, NaiveDateTime, TimeZone, Utc};
use elsa::FrozenBTreeMap;
use log;
use num_traits::FromPrimitive;
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;

use crate::config::Config;
use crate::provider::datetime::days_of_month;
use crate::provider::ical;
use crate::provider::tz::*;
use crate::provider::{Alarm, EventFilter, MutCalendarlike, Occurrence, ProviderCalendar, Result};

pub struct Agenda {
    calendars: BTreeMap<String, ProviderCalendar>,
}

impl Agenda {
    pub fn from_config(
        config: &Config,
        event_sink: &std::sync::mpsc::Sender<crate::events::Event>,
    ) -> Result<Self> {

        let calendars: BTreeMap<String, ProviderCalendar> = config
            .collections
            .iter()
            .filter_map(|collection_spec| {
                if collection_spec.provider == "ical" {
                    Some(ical::from_dir(
                        collection_spec.path.as_path(),
                        collection_spec.calendars.as_slice(),
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
        })
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
