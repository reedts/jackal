use chrono::prelude::*;
use chrono::Duration;
use num_traits::FromPrimitive;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::marker::PhantomPinned;
use std::ops::Bound::{Excluded, Included};
use std::path::Path;
use std::pin::Pin;
use std::ptr;

use crate::ical;

pub type EventMap<'a> = BTreeMap<DateTime<Utc>, Vec<*const ical::Event>>;

fn days_of_month(month: &Month, year: i32) -> u64 {
    if month.number_from_month() == 12 {
        NaiveDate::from_ymd(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd(year, month.number_from_month() as u32 + 1, 1)
    }
    .signed_duration_since(NaiveDate::from_ymd(
        year,
        month.number_from_month() as u32,
        1,
    ))
    .num_days() as u64
}

#[derive(Clone)]
pub struct Agenda<'a> {
    path: &'a Path,
    collections: Vec<ical::Collection<'a>>,
    events: EventMap<'a>,
    // Allow self referencing
    _pin: PhantomPinned,
}

impl<'a> TryFrom<&'a Path> for Pin<Box<Agenda<'a>>> {
    type Error = std::io::Error;
    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        let collections = vec![ical::Collection::try_from(path)?];

        let res = Agenda {
            path,
            collections,
            events: EventMap::new(),
            _pin: PhantomPinned,
        };

        let mut boxed = Box::pin(res);

        // Fill up eventmap. This is safe, even if we cannot explain this
        // to the compiler...
        unsafe {
            let mut event_map = EventMap::new();

            {
                for (dt, ev) in boxed
                    .collections
                    .iter()
                    .flat_map(|c| c.event_iter())
                    .map(|ev| (ev.occurence().as_datetime(&Utc {}), ev))
                {
                    event_map.entry(dt).or_default().push(ev)
                }
            }

            let mut_ref: Pin<&mut Agenda> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).events = event_map;
        }

        Ok(boxed)
    }
}

impl Agenda<'_> {
    pub fn events_of_month(&self, month: Month, year: i32) -> impl Iterator<Item = &ical::Event> {
        let b_date = DateTime::<Utc>::from_utc(
            NaiveDate::from_ymd(year, month.number_from_month() as u32, 1).and_hms(0, 0, 0),
            Utc {},
        );
        let e_date = b_date + Duration::days(days_of_month(&month, year) as i64);

        self.events
            .range((Included(b_date), Included(e_date)))
            .flat_map(|(_, evts)| evts.iter())
            .map(|ev| unsafe { &*(*ev) })
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
            .flat_map(|(_, evts)| evts.iter())
            .map(|ev| unsafe { &*(*ev) })
    }

    pub fn events_of_current_day(&self) -> impl Iterator<Item = &ical::Event> {
        let today = Utc::today();

        self.events_of_day(&today)
    }
}
