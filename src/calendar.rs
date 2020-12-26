use chrono::naive::NaiveDate;
use chrono::{Date, Datelike, Duration, FixedOffset, Offset, TimeZone, Utc};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::From;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::ops::Bound::Included;
use std::path::{Path, PathBuf};

use crate::ical;

pub type EventList = Vec<ical::Event<FixedOffset>>;
pub type EventMap = BTreeMap<Date<FixedOffset>, EventList>;

pub struct Calendar {
    path: PathBuf,
    name: String,
    icals: Vec<ical::Calendar<FixedOffset>>,
    events: EventMap,
}

pub struct Day<'a, Tz: TimeZone> {
    date: Date<Tz>,
    events: Vec<&'a ical::Event<Tz>>,
}

#[derive(Clone, Copy)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[derive(Debug)]
pub struct NotAMonthError;

impl Calendar {
    pub fn new(path: &Path) -> io::Result<Calendar> {
        // Load all valid .ics files from 'path'
        let mut icals: Vec<ical::Calendar<FixedOffset>> = fs::read_dir(path)?
            .map(|dir| {
                dir.map_or_else(
                    |_| -> std::io::Result<_> { Err(io::Error::from(io::ErrorKind::NotFound)) },
                    |file: std::fs::DirEntry| -> std::io::Result<ical::Calendar<FixedOffset>> {
                        ical::Calendar::from(file.path().as_path())
                    },
                )
            })
            .filter_map(Result::ok)
            .collect();

        let mut events: EventMap = EventMap::new();

        for event in icals.iter_mut().flat_map(|cal| cal.events_mut().iter_mut()) {
            events
                .entry(event.begin_date())
                .or_insert(EventList::new())
                .push(event.clone())
        }

        Ok(Calendar {
            path: PathBuf::from(path),
            name: path.file_name().unwrap().to_str().unwrap().to_owned(),
            icals,
            events,
        })
    }

    pub fn curr_month(&self) -> Month {
        let date = Utc::now().date();

        Month::from(date.naive_utc().month() - 1)
    }

    pub fn curr_year(&self) -> i32 {
        let date = Utc::now().date();

        date.naive_utc().year()
    }

    pub fn all_events(&self) -> Vec<&ical::Event<FixedOffset>> {
        self.icals
            .iter()
            .map(|cal| cal.events())
            .flatten()
            .collect()
    }

    pub fn events_of_month_and_year(
        &self,
        month: Month,
        year: i32,
    ) -> Vec<&ical::Event<FixedOffset>> {
        let b_date = Date::from_utc(
            NaiveDate::from_ymd(year, month.num() as u32, 1),
            chrono::offset::Utc.fix(),
        );
        let e_date = b_date + Duration::days(month.days(year) as i64);

        self.events
            .range((Included(&b_date), Included(&e_date)))
            .flat_map(|(k, v)| v.iter())
            .collect()
    }

    pub fn events_of_curr_month(&self) -> Vec<&ical::Event<FixedOffset>> {
        let curr_month = self.curr_month();
        let curr_year = self.curr_year();

        self.events_of_month_and_year(curr_month, curr_year)
    }

    pub fn events_of_day(&self, day: u32, month: Month, year: i32) -> Day<FixedOffset> {
        let date = Date::from_utc(
            NaiveDate::from_ymd(year, month.num() as u32, day),
            chrono::offset::Utc.fix(),
        );

        match self.events.get(&date) {
            Some(events) => Day::new(date, events),
            None => Day::new(date, &[]),
        }
    }
}

impl<'a> Day<'a, FixedOffset> {
    pub fn new(date: Date<FixedOffset>, events: &'a [ical::Event<FixedOffset>]) -> Self {
        Day {
            date,
            events: events.into_iter().collect(),
        }
    }

    pub fn date(&self) -> &Date<FixedOffset> {
        &self.date
    }

    pub fn events(&self) -> &Vec<&ical::Event<FixedOffset>> {
        &self.events
    }
}

impl Month {
    pub fn ord(&self) -> u32 {
        match *self {
            Month::January => 0,
            Month::February => 1,
            Month::March => 2,
            Month::April => 3,
            Month::May => 4,
            Month::June => 5,
            Month::July => 6,
            Month::August => 7,
            Month::September => 8,
            Month::October => 9,
            Month::November => 10,
            Month::December => 11,
        }
    }

    pub fn num(&self) -> u32 {
        match *self {
            Month::January => 1,
            Month::February => 2,
            Month::March => 3,
            Month::April => 4,
            Month::May => 5,
            Month::June => 6,
            Month::July => 7,
            Month::August => 8,
            Month::September => 9,
            Month::October => 10,
            Month::November => 11,
            Month::December => 12,
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        }
    }

    pub fn days(&self, year: i32) -> u64 {
        if self.num() == 12 {
            NaiveDate::from_ymd(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd(year, self.num() as u32 + 1, 1)
        }
        .signed_duration_since(NaiveDate::from_ymd(year, self.num() as u32, 1))
        .num_days() as u64
    }

    pub fn next(&self) -> Month {
        Month::from((self.num() % 12) + 1)
    }

    pub fn pred(&self) -> Month {
        let pred = self.num() - 1;
        Month::from(std::cmp::min(1, pred))
    }
}

impl PartialOrd for Month {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Month {
    fn eq(&self, other: &Self) -> bool {
        self.ord() == other.ord()
    }
}

impl Eq for Month {}

impl Ord for Month {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ord().cmp(&other.ord())
    }
}

impl From<u32> for Month {
    fn from(value: u32) -> Self {
        match value {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => Month::December,
        }
    }
}

impl fmt::Display for NotAMonthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value could not be converted to a month")
    }
}

impl Error for NotAMonthError {}
