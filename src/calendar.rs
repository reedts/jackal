use chrono::{Date, Datelike, TimeZone, Utc};
use chrono::naive::NaiveDate;
use std::cmp::Ordering;
use std::convert::{TryInto, TryFrom};
use std::fmt;
use std::fs;
use std::io;
use std::error::Error;
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

#[derive(Debug)]
pub struct NotAMonthError {}


pub struct Year<'a, Tz: TimeZone> {
    year: i32,
    begin: NaiveDate,
    end: NaiveDate,
    months: [Month<'a, Tz>; 12]
}

impl<'a> Calendar<'a> {
    pub fn new(path: &'a Path, year: i32) -> io::Result<Calendar<'a>> {
        Ok(Calendar {
            path: PathBuf::from(path),
            icals: fs::read_dir(path)?
                .map(|rd| rd.map_or_else(
                    |_| -> vobject::error::Result<ICalendar> {
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

    pub fn curr_month(&self) -> &Month<'a, Utc> {
        let date = Utc::now().date();

        &self.year.months[(date.naive_utc().month() - 1) as usize]
    }

    pub fn curr_month_mut(&mut self) -> &mut Month<'a, Utc> {
        let date = Utc::now().date();

        &mut self.year.months[date.naive_utc().month0() as usize]
    }

    pub fn curr_day(&self) -> &Day<'a, Utc> {
        let date = Utc::now().date();
        let naive_date = date.naive_utc();

        &self.year.months[naive_date.month0() as usize].days[naive_date.day0() as usize]
    }

    pub fn curr_day_mut(&mut self) -> &mut Day<'a, Utc> {
        let date = Utc::now().date();
        let naive_date = date.naive_utc();

        &mut self.year.months[naive_date.month0() as usize].days[naive_date.day0() as usize]
    }
}

impl<'a, Tz: TimeZone> Day<'a, Tz> {
    pub fn new(date: Date<Tz>, events: &[Event<'a>]) -> Day<'a, Tz> {
        Day { date, events: events.to_vec() }
    }
}

impl<'a, Tz: TimeZone> fmt::Display for Day<'a, Tz> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.date.naive_utc().day())
    }
}

impl<'a, Tz: TimeZone> Month<'a, Tz> {
    pub fn days(&self) -> &Vec<Day<'a, Tz>> {
        &self.days
    }
}

impl MonthValue {
    pub fn ord(&self) -> u8 {
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

    pub fn num(&self) -> u8 {
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

    pub fn name(&self) -> &'static str {
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

impl TryFrom<u32> for MonthValue {
    type Error = NotAMonthError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1  => Ok(MonthValue::January),
            2  => Ok(MonthValue::February),
            3  => Ok(MonthValue::March),
            4  => Ok(MonthValue::April),
            5  => Ok(MonthValue::May),
            6  => Ok(MonthValue::June),
            7  => Ok(MonthValue::July),
            8  => Ok(MonthValue::August),
            9  => Ok(MonthValue::September),
            10 => Ok(MonthValue::October),
            11 => Ok(MonthValue::November),
            12 => Ok(MonthValue::December) ,
            _ => Err(Self::Error {})
        }
    }
}

impl fmt::Display for NotAMonthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value could not be converted to a month")
    }
}

impl Error for NotAMonthError {}

impl<'a> Year<'a, Utc> {
    fn new(year: i32) -> Year<'a, Utc> {
        let mut y = Year {
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

        for month in y.months.iter_mut() {
            let days =
                if month.value.num() == 12 {
                    NaiveDate::from_ymd(year + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd(year, month.value.num() as u32 + 1, 1)
                }.signed_duration_since(NaiveDate::from_ymd(year, month.value.num() as u32, 1))
                .num_days();
            for d in 1..=days {
                let date = NaiveDate::from_ymd(year, month.value.num() as u32, d as u32);
                month.days.push(
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
