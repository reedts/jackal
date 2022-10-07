use chrono::{Datelike, Duration, Month, NaiveDate, Utc};
use log;
use num_traits::FromPrimitive;

use crate::config::Config;
use crate::provider::*;

pub struct Agenda {
    collections: Vec<Box<dyn Collectionlike>>,
}

impl Agenda {
    pub fn from_config(config: &Config) -> Result<Self> {
        let collections: Vec<Box<dyn Collectionlike>> = config
            .collections
            .iter()
            .map(|collection_spec| {
                load_collection_with_calendars(
                    &collection_spec.provider,
                    &collection_spec.path,
                    collection_spec.calendars.as_slice(),
                )
            })
            .inspect(|c| {
                if let Err(e) = c {
                    log::warn!("{}", e)
                }
            })
            .filter_map(Result::ok)
            .map(|calendar| -> Box<dyn Collectionlike> { Box::new(calendar) })
            .collect();

        Ok(Agenda { collections })
    }

    pub fn _events_of_month<'a>(
        &'a self,
        month: Month,
        year: i32,
    ) -> impl Iterator<Item = &'a dyn Eventlike> + 'a {
        let begin = NaiveDate::from_ymd(year, month.number_from_month() as u32, 1).and_hms(0, 0, 0);
        let end = begin + Duration::days(_days_of_month(&month, year) as i64);

        self.collections
            .iter()
            .flat_map(|collection| collection.calendar_iter())
            .flat_map(move |calendar| {
                calendar.filter_events(EventFilter::default().datetime_range(begin..=end))
            })
    }

    pub fn _events_of_current_month(&self) -> impl Iterator<Item = &dyn Eventlike> {
        let today = Utc::today();
        let curr_month = Month::from_u32(today.month()).unwrap();
        let curr_year = today.year();

        self._events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day(&self, date: &NaiveDate) -> impl Iterator<Item = &dyn Eventlike> {
        let begin = date.and_hms(0, 0, 0);
        let end = begin + Duration::days(1);

        self.collections
            .iter()
            .flat_map(|collection| collection.calendar_iter())
            .flat_map(move |calendar| {
                calendar.filter_events(EventFilter::default().datetime_range(begin..=end))
            })
    }

    pub fn _events_of_current_day(&self) -> impl Iterator<Item = &dyn Eventlike> {
        let today = Utc::today();

        self.events_of_day(&today.naive_utc())
    }
}
