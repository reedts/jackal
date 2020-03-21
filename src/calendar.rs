use chrono::{Date, TimeZone, Utc};
use chrono::naive::NaiveDate;
use std::convert::TryInto;
use std::cmp::Ordering;
use std::io;
use std::fs;
use std::path::{Path, PathBuf};
use vobject::icalendar::{ICalendar, Event};

pub struct Calendar<'a> {
    path: PathBuf,
    icals: Vec<ICalendar>,
    year: Year<'a, Utc>
}


pub struct Day<'a, Tz: TimeZone> {
    date: Date<Tz>,
    events: Vec<Event<'a>>,
}

pub struct Month<'a, Tz: TimeZone> {
    value: MonthValue,   
    days: Vec<Day<'a, Tz>>
}

pub enum MonthValue {
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


pub struct Year<'a, Tz: TimeZone> {
    year: i32,
    begin: NaiveDate,
    end: NaiveDate,
    months: [Month<'a, Tz>; 12]
}

impl<'a> Calendar<'a> {
    fn new(path: &'a Path, year: i32) -> io::Result<Calendar<'a>> {
        Ok(Calendar {
            path: PathBuf::from(path),
            icals: fs::read_dir(path)?
                .map(|rd| rd.map_or_else(
                    |err| -> vobject::error::Result<ICalendar> {
                        Err(vobject::error::VObjectErrorKind::NotAVCard)
                    },
                    |file| -> vobject::error::Result<ICalendar> {
                        ICalendar::build(file.path().to_str().unwrap_or(""))
                    }
                ))
                .filter_map(|c| c.ok())
                .collect(),
            year: Year::new(year)
        })

    }
}

impl<'a, Tz: TimeZone> Day<'a, Tz> {
    fn new(date: Date<Tz>, events: &[Event<'a>]) -> Day<'a, Tz> {
        Day { date, events: events.to_vec() }
    }
}

impl MonthValue {
    fn ord(&self) -> u8 {
        match *self {
            MonthValue::January     => 0,
            MonthValue::February    => 1,
            MonthValue::March       => 2,
            MonthValue::April       => 3,
            MonthValue::May         => 4,
            MonthValue::June        => 5,
            MonthValue::July        => 6,
            MonthValue::August      => 7,
            MonthValue::September   => 8,
            MonthValue::October     => 9,
            MonthValue::November    => 10,
            MonthValue::December    => 11
        }
    }
    
    fn num(&self) -> u8 {
        match *self {
            MonthValue::January     => 1,
            MonthValue::February    => 2,
            MonthValue::March       => 3,
            MonthValue::April       => 4,
            MonthValue::May         => 5,
            MonthValue::June        => 6,
            MonthValue::July        => 7,
            MonthValue::August      => 8,
            MonthValue::September   => 9,
            MonthValue::October     => 10,
            MonthValue::November    => 11,
            MonthValue::December    => 12 
        }
    }

    fn name(&self) -> &'static str {
        match *self {
            MonthValue::January     => "January",
            MonthValue::February    => "February",
            MonthValue::March       => "March",
            MonthValue::April       => "April",
            MonthValue::May         => "May",
            MonthValue::June        => "June",
            MonthValue::July        => "July",
            MonthValue::August      => "August",
            MonthValue::September   => "September",
            MonthValue::October     => "October",
            MonthValue::November    => "November",
            MonthValue::December    => "December"
        }
    }
}

impl PartialOrd for MonthValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MonthValue {
    fn eq(&self, other: &Self) -> bool {
        self.ord() == other.ord()
    }
}

impl Eq for MonthValue {}

impl Ord for MonthValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ord().cmp(&other.ord())
    }
}

impl<'a> Year<'a, Utc> {
    fn new(year: i32) -> Year<'a, Utc> {
        let y = Year {
            year,
            begin:  NaiveDate::from_ymd(year.try_into().unwrap(), 1, 1),
            end:    NaiveDate::from_ymd(year.try_into().unwrap(), 12, 31),
            months: [
                Month { value: MonthValue::January,     days: Vec::with_capacity(31)},
                Month { value: MonthValue::February,    days: Vec::with_capacity(30)},
                Month { value: MonthValue::March,       days: Vec::with_capacity(31)},
                Month { value: MonthValue::April,       days: Vec::with_capacity(30)},
                Month { value: MonthValue::May,         days: Vec::with_capacity(31)},
                Month { value: MonthValue::June,        days: Vec::with_capacity(30)},
                Month { value: MonthValue::July,        days: Vec::with_capacity(31)},
                Month { value: MonthValue::August,      days: Vec::with_capacity(31)},
                Month { value: MonthValue::September,   days: Vec::with_capacity(30)},
                Month { value: MonthValue::October,     days: Vec::with_capacity(31)},
                Month { value: MonthValue::November,    days: Vec::with_capacity(30)},
                Month { value: MonthValue::December,    days: Vec::with_capacity(31)}
            ]
        };
        for (month, days) in y.months.into_iter().map(|m| {
            // Get number of days in month by calculating the
            // difference between the first of the month and the
            // first of the next month
            (
                m,
                if m.value.num() == 12 {
                    NaiveDate::from_ymd(year + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd(year, m.value.num() as u32 + 1, 1)
                }.signed_duration_since(NaiveDate::from_ymd(year, m.value.num() as u32, 1))
                .num_days()
            )
        }) {
            for d in 1..=days {
                let date = NaiveDate::from_ymd(year, month.value.num() as u32, d as u32);
                y.months[month.value.ord() as usize].days.insert(
                    0,
                    Day {
                        date: Utc.from_utc_date(&date),
                        events: Vec::new()
                    }
                );
            }
        }

        y
    }
}
