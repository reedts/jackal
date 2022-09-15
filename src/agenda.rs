use chrono::{Date, DateTime, Duration, Month, NaiveDate, Utc, TimeZone};
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

impl TryFrom<&[&Path]> for Agenda {
    type Error = Error;

    fn try_from(value: &[&Path]) -> Result<Self> {
        let collections = value
            .iter()
            .map(|path| Box::new(ical::Collection::from_dir(path)))
            .inspect(|c| {
                if let Err(e) = c {
                    log::warn!("{}", e)
                }
            })
            .filter_map(|c| c.ok())
            .collect::<Vec<ical::Collection>>();

        if collections.is_empty() {
            return Err(Self::Error::from(std::io::ErrorKind::NotFound));
        }

        let mut events = EventMap::new();
        for (i, col) in collections.iter().enumerate() {
            for (j, cal) in col.calendars().iter().enumerate() {
                for (k, ev) in cal.events_iter().enumerate() {
                    events
                        .entry(ev.occurrence().as_datetime(&Utc {}))
                        .or_default()
                        .push(AgendaIndex(i, j, k))
                }
            }
        }

        Ok(Agenda {
            collections,
            events,
        })
    }
}

impl TryFrom<&Path> for Agenda {
    type Error = Error;
    fn try_from(path: &Path) -> Result<Self> {
        let dirs = path
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .collect::<Vec<PathBuf>>();

        if dirs.is_empty() {
            Self::try_from(&[path] as &[_])
        } else {
            Self::try_from(
                dirs.iter()
                    .map(|p| p.as_path())
                    .collect::<Vec<&Path>>()
                    .as_slice(),
            )
        }
    }
}

impl Agenda{
    pub fn from_config(config: &Config) -> Result<Self> {
        for collection_config in config.collections.iter() {

        }
    }

    fn lookup_event(&self, index: &AgendaIndex) -> &ical::Event {
        &self.collections[index.0].calendars()[index.1].events()[index.2]
    }

    pub fn events_of_month<'a>(
        &'a self,
        month: Month,
        year: i32,
    ) -> impl Iterator<Item = &'a ical::Event> + 'a {
        let b_date = DateTime::<Utc>::from_utc(
            NaiveDate::from_ymd(year, month.number_from_month() as u32, 1).and_hms(0, 0, 0),
            Utc {},
        );
        let e_date = b_date + Duration::days(days_of_month(&month, year) as i64);

        self.events
            .range((Included(b_date), Included(e_date)))
            .flat_map(|(_, indices)| indices.iter())
            .map(move |index| self.lookup_event(index))
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
