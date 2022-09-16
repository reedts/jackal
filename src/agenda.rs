use chrono::{Date, DateTime, Datelike, Duration, Local, Month, NaiveDate, TimeZone, Utc};
use log;
use num_traits::FromPrimitive;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ops::Bound::{Excluded, Included};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::provider::*;

pub type EventMap = BTreeMap<DateTime<Utc>, Vec<AgendaIndex>>;

#[derive(Debug, Clone)]
pub struct AgendaIndex(usize, usize, usize);

pub struct Agenda {
    collections: Vec<Box<dyn Collectionlike>>,
}

impl Agenda {
    pub fn from_config(config: &Config) -> Result<Self> {
        let collections: Vec<Box<dyn Collectionlike>> = config
            .collections
            .iter()
            .map(|collection_spec| {
                load_collection_with_calendars::<Local>(
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
            .flat_map(|collection| collection.event_iter())
    }

    pub fn events_of_current_month(&self) -> impl Iterator<Item = &ical::Event> {
        let today = Utc::today();
        let curr_month = Month::from_u32(today.month()).unwrap();
        let curr_year = today.year();

        self.events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day<Tz: TimeZone>(
        &self,
        date: &Date<Tz>,
    ) -> impl Iterator<Item = &ical::Event> {
        let begin = Utc.from_utc_date(&date.naive_utc()).and_hms(0, 0, 0);
        let end = begin + Duration::days(1);

        self.events
            .range((Included(begin), Excluded(end)))
            .flat_map(|(_, indices)| indices.iter())
            .map(move |index| self.lookup_event(index))
    }

    pub fn events_of_current_day(&self) -> impl Iterator<Item = &ical::Event> {
        let today = Utc::today();

        self.events_of_day(&today)
    }
}
