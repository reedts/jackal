use chrono::prelude::*;
use chrono::Duration;
use num_traits::FromPrimitive;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::marker::PhantomPinned;
use std::ops::Bound::Included;
use std::path::Path;
use std::pin::Pin;

use crate::ical;

pub type EventMap<'a> = BTreeMap<DateTime<Local>, Vec<&'a ical::Event>>;

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

pub struct EventsOfDay<'a, Tz: TimeZone> {
    date: Date<Tz>,
    events: Vec<&'a ical::Event>,
}

impl<'a> EventsOfDay<'a, FixedOffset> {
    pub fn new<Iter: Iterator<Item = &'a ical::Event>>(
        date: Date<FixedOffset>,
        events_it: Iter,
    ) -> Self {
        EventsOfDay {
            date,
            events: events_it.collect(),
        }
    }

    pub fn date(&self) -> &Date<FixedOffset> {
        &self.date
    }

    pub fn events(&self) -> &Vec<&ical::Event> {
        &self.events
    }
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
            let mut_ref: Pin<&mut Agenda> = Pin::as_mut(&mut boxed);
            let mut event_map = &Pin::get_unchecked_mut(mut_ref).events;

            for (dt, ev) in boxed
                .collections
                .iter()
                .flat_map(|c| c.event_iter())
                .map(|ev| (ev.occurence().as_datetime(&Local{}), ev))
            {
                event_map.entry(dt).or_default().push(ev)
            }
        }

        Ok(boxed)
    }
}

impl Agenda<'_> {
    pub fn curr_month(&self) -> Month {
        let date = Utc::now().date();

        Month::from_u32(date.naive_utc().month()).unwrap()
    }

    pub fn curr_year(&self) -> i32 {
        let date = Utc::now().date();

        date.naive_utc().year()
    }

    pub fn events_of_month(&self, month: Month, year: i32) -> Vec<&ical::Event> {
        let b_date = Date::from_utc(
            NaiveDate::from_ymd(year, month.number_from_month() as u32, 1),
            chrono::offset::Utc.fix(),
        );
        let e_date = b_date + Duration::days(days_of_month(&month, year) as i64);

        self.events
            .range((Included(&b_date), Included(&e_date)))
            .flat_map(|(_, v)| v.values())
            .collect()
    }

    pub fn events_of_curr_month(&self) -> Vec<&ical::Event> {
        let curr_month = self.curr_month();
        let curr_year = self.curr_year();

        self.events_of_month(curr_month, curr_year)
    }

    pub fn events_of_day(&self, date: &Date<FixedOffset>) -> EventsOfDay<FixedOffset> {
        match self.events.get(&date) {
            Some(events) => EventsOfDay::new(*date, events.values()),
            None => EventsOfDay::new(*date, [].iter()),
        }
    }
}
