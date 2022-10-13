use chrono::{DateTime, Datelike, Duration, Month, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use log;
use num_traits::FromPrimitive;

use crate::config::Config;
use crate::provider::datetime::days_of_month;
use crate::provider::ical;
use crate::provider::{CalendarMut, Calendarlike, EventFilter, Eventlike, Result};

pub struct Agenda {
    ical_collections: Vec<ical::Collection>,
}

impl Agenda {
    pub fn from_config(config: &Config) -> Result<Self> {
        let ical_collections: Vec<ical::Collection> = config
            .collections
            .iter()
            .filter_map(|collection_spec| {
                if collection_spec.provider == "ical" {
                    Some(ical::from_dir(
                        collection_spec.path.as_path(),
                        collection_spec,
                    ))
                } else {
                    None
                }
            })
            .inspect(|c| {
                if let Err(e) = c {
                    log::warn!("{}", e)
                }
            })
            .filter_map(Result::ok)
            .collect();

        Ok(Agenda { ical_collections })
    }

    /// Note, even though events are sorted within one calendar, they are not sorted in the
    /// resulting iterator since multiple calendars are merged
    pub fn events_in<'a>(
        &'a self,
        range: impl std::ops::RangeBounds<NaiveDateTime> + 'a + Clone,
    ) -> impl Iterator<Item = (&DateTime<Tz>, &'a dyn Eventlike)> + 'a {
        self.collections
            .iter()
            .flat_map(|collection| collection.calendar_iter())
            .flat_map(move |calendar| {
                calendar
                    .filter_events(EventFilter::default().datetime_range(range.clone()))
                    .map(|(k, v)| (k, v))
            })
    }

    pub fn _events_of_month(
        &self,
        month: Month,
        year: i32,
    ) -> impl Iterator<Item = (&DateTime<Tz>, &'a dyn Eventlike)> + 'a {
        let begin = NaiveDate::from_ymd(year, month.number_from_month() as u32, 1).and_hms(0, 0, 0);
        let end = begin + Duration::days(days_of_month(&month, year) as i64);

        self.events_in(begin..=end)
    }

    pub fn _events_of_current_month<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&DateTime<Tz>, &'a dyn Eventlike)> + 'a {
        let today = Utc::today();
        let curr_month = Month::from_u32(today.month()).unwrap();
        let curr_year = today.year();

        self._events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day<'a>(
        &'a self,
        date: &NaiveDate,
    ) -> impl Iterator<Item = (&DateTime<Tz>, &'a dyn Eventlike)> + 'a {
        let begin = date.and_hms(0, 0, 0);
        let end = begin + Duration::days(1);

        self.ical_collections
            .iter()
            .flat_map(|collection| collection.calendars())
            .flat_map(move |calendar| {
                calendar
                    .filter_events(EventFilter::default().datetime_range(begin..=end))
                    .map(|ev| ev as &dyn Eventlike)
            })
    }

    pub fn _events_of_current_day<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&DateTime<Tz>, &'a dyn Eventlike)> + 'a {
        let today = Utc::today();

        self.events_of_day(&today.naive_utc())
    }
}
