use chrono::{Date, DateTime, Datelike, Duration, Month, NaiveDate, TimeZone, Utc};
use log;
use num_traits::FromPrimitive;
use std::ops::Bound::Included;

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

    pub fn events_of_month<'a>(
        &'a self,
        month: Month,
        year: i32,
    ) -> impl Iterator<Item = &'a dyn Eventlike> + 'a {
        let b_date = DateTime::<Utc>::from_utc(
            NaiveDate::from_ymd(year, month.number_from_month() as u32, 1).and_hms(0, 0, 0),
            Utc {},
        );
        let e_date = b_date + Duration::days(days_of_month(&month, year) as i64);

        self.collections
            .iter()
            .flat_map(|collection| collection.calendar_iter())
            .flat_map(move |calendar| {
                calendar.filter_events(EventFilter::default().datetime_range((
                    Included(b_date.with_timezone(calendar.tz())),
                    Included(e_date.with_timezone(calendar.tz())),
                )))
            })
    }

    pub fn events_of_current_month(&self) -> impl Iterator<Item = &dyn Eventlike> {
        let today = Utc::today();
        let curr_month = Month::from_u32(today.month()).unwrap();
        let curr_year = today.year();

        self.events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day(&self, date: &NaiveDate) -> impl Iterator<Item = &dyn Eventlike> {
        let begin = Utc.from_utc_date(&date).and_hms(0, 0, 0);
        let end = begin + Duration::days(1);

        self.collections
            .iter()
            .flat_map(|collection| collection.calendar_iter())
            .flat_map(move |calendar| {
                calendar.filter_events(EventFilter::default().datetime_range((
                    Included(begin.with_timezone(calendar.tz())),
                    Included(end.with_timezone(calendar.tz())),
                )))
            })
    }

    pub fn events_of_current_day(&self) -> impl Iterator<Item = &dyn Eventlike> {
        let today = Utc::today();

        self.events_of_day(&today.naive_utc())
    }
}
