use chrono::naive::NaiveDate;
use chrono::{
    Date,
    Datelike,
    FixedOffset,
    TimeZone,
    Utc,
    Weekday
};
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ical;

pub struct Calendar {
    path: PathBuf,
    icals: Vec<ical::Calendar<Utc>>,
    year: Year<Utc>,
}

pub struct Day<Tz: TimeZone> {
    date: Date<Tz>,
    events: Vec<ical::Event<Tz>>,
}

pub struct Month<Tz: TimeZone> {
    name: MonthName,
    days: Vec<Day<Tz>>,
}

#[derive(Clone, Copy)]
pub enum MonthName {
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

pub struct Year<Tz: TimeZone> {
    year: i32,
    months: [Month<Tz>; 12],
}

impl Calendar {
    pub fn new(path: &Path, year: i32) -> io::Result<Calendar> {
        // Load all valid .ics files from 'path'
        let icals: Vec<ical::Calendar<Utc>> = fs::read_dir(path)?
            .map(|rd| {
                rd.map_or_else(
                    |_| -> std::io::Result<_> {
                        Err(io::Error::from(io::ErrorKind::NotFound))
                    },
                    |file: std::fs::DirEntry| -> std::io::Result<ical::Calendar<Utc>> {
                        ical::Calendar::from(file.path().as_path())
                    },
                )
            })
            .filter_map(|c| c.ok())
            .collect();

        Ok(Calendar {
            path: PathBuf::from(path),
            icals,
            year: Year::new(year),
        })
    }

    pub fn year(&self) -> &Year<Utc> {
        &self.year
    }

    pub fn curr_month(&self) -> &Month<Utc> {
        let date = Utc::now().date();

        &self.year.months[(date.naive_utc().month() - 1) as usize]
    }

    pub fn month_from_name(&self, name: MonthName) -> &Month<Utc> {
        &self.year.months[name.ord() as usize]
    }

    pub fn month_from_idx(&self, idx: u32) -> Option<&Month<Utc>> {
        self.year.months.get(idx as usize)
    }

    pub fn curr_month_mut(&mut self) -> &mut Month<Utc> {
        let date = Utc::now().date();

        &mut self.year.months[date.naive_utc().month0() as usize]
    }

    pub fn curr_day(&self) -> &Day<Utc> {
        let date = Utc::now().date();
        let naive_date = date.naive_utc();

        &self.year.months[naive_date.month0() as usize].days[naive_date.day0() as usize]
    }

    pub fn curr_day_mut(&mut self) -> &mut Day<Utc> {
        let date = Utc::now().date();
        let naive_date = date.naive_utc();

        &mut self.year.months[naive_date.month0() as usize].days[naive_date.day0() as usize]
    }

    pub fn all_events(&self) -> Vec<&ical::Event<Utc>> {
        self.icals.iter().
            map(|cal| cal.events())
            .flatten()
            .collect()
    }
}

impl<Tz: TimeZone> Day<Tz> {
    pub fn new(date: Date<Tz>, events: &[ical::Event<Tz>]) -> Day<Tz> {
        Day {
            date,
            events: events.to_vec(),
        }
    }

    pub fn date(&self) -> &Date<Tz> {
        &self.date
    }

    pub fn day_num(&self) -> u32 {
        self.date.naive_utc().day()
    }

    pub fn weekday(&self) -> Weekday {
        self.date.weekday()
    }

    pub fn events(&self) -> &Vec<ical::Event<Tz>> {
        &self.events
    }
}

impl<Tz: TimeZone> fmt::Display for Day<Tz> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.date.naive_utc().day())
    }
}

impl<Tz: TimeZone> Month<Tz> {
    pub fn days(&self) -> &Vec<Day<Tz>> {
        &self.days
    }

    pub fn day(&self, n: usize) -> &Day<Tz> {
        &self.days[n]
    }

    pub fn day_mut(&mut self, n: usize) -> &mut Day<Tz> {
        &mut self.days[n]
    }

    pub fn name(&self) -> MonthName {
        self.name
    }

    pub fn to_str(&self) -> &'static str {
        self.name.name()
    }

    pub fn num(&self) -> u32 {
        self.name.num() as u32
    }

    pub fn ord(&self) -> u32 {
        self.name.ord() as u32
    }
}

impl MonthName {
    pub fn ord(&self) -> u8 {
        match *self {
            MonthName::January   => 0,
            MonthName::February  => 1,
            MonthName::March     => 2,
            MonthName::April     => 3,
            MonthName::May       => 4,
            MonthName::June      => 5,
            MonthName::July      => 6,
            MonthName::August    => 7,
            MonthName::September => 8,
            MonthName::October   => 9,
            MonthName::November  => 10,
            MonthName::December  => 11,
        }
    }

    pub fn num(&self) -> u8 {
        match *self {
            MonthName::January   => 1,
            MonthName::February  => 2,
            MonthName::March     => 3,
            MonthName::April     => 4,
            MonthName::May       => 5,
            MonthName::June      => 6,
            MonthName::July      => 7,
            MonthName::August    => 8,
            MonthName::September => 9,
            MonthName::October   => 10,
            MonthName::November  => 11,
            MonthName::December  => 12,
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            MonthName::January   => "January",
            MonthName::February  => "February",
            MonthName::March     => "March",
            MonthName::April     => "April",
            MonthName::May       => "May",
            MonthName::June      => "June",
            MonthName::July      => "July",
            MonthName::August    => "August",
            MonthName::September => "September",
            MonthName::October   => "October",
            MonthName::November  => "November",
            MonthName::December  => "December",
        }
    }
}

impl PartialOrd for MonthName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MonthName {
    fn eq(&self, other: &Self) -> bool {
        self.ord() == other.ord()
    }
}

impl Eq for MonthName {}

impl Ord for MonthName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ord().cmp(&other.ord())
    }
}

impl TryFrom<u32> for MonthName {
    type Error = NotAMonthError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1  => Ok(MonthName::January),
            2  => Ok(MonthName::February),
            3  => Ok(MonthName::March),
            4  => Ok(MonthName::April),
            5  => Ok(MonthName::May),
            6  => Ok(MonthName::June),
            7  => Ok(MonthName::July),
            8  => Ok(MonthName::August),
            9  => Ok(MonthName::September),
            10 => Ok(MonthName::October),
            11 => Ok(MonthName::November),
            12 => Ok(MonthName::December),
            _  => Err(Self::Error {}),
        }
    }
}

impl fmt::Display for NotAMonthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value could not be converted to a month")
    }
}

impl Error for NotAMonthError {}

impl Year<Utc> {
    fn new(year: i32) -> Year<Utc> {
        let mut y = Year {
            year,
            months: [
                Month {
                    name: MonthName::January,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::February,
                    days: Vec::with_capacity(30),
                },
                Month {
                    name: MonthName::March,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::April,
                    days: Vec::with_capacity(30),
                },
                Month {
                    name: MonthName::May,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::June,
                    days: Vec::with_capacity(30),
                },
                Month {
                    name: MonthName::July,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::August,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::September,
                    days: Vec::with_capacity(30),
                },
                Month {
                    name: MonthName::October,
                    days: Vec::with_capacity(31),
                },
                Month {
                    name: MonthName::November,
                    days: Vec::with_capacity(30),
                },
                Month {
                    name: MonthName::December,
                    days: Vec::with_capacity(31),
                },
            ],
        };

        for month in y.months.iter_mut() {
            let days = if month.name.num() == 12 {
                NaiveDate::from_ymd(year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd(year, month.name.num() as u32 + 1, 1)
            }
            .signed_duration_since(NaiveDate::from_ymd(year, month.name.num() as u32, 1))
            .num_days();
            for d in 1..=days {
                let date = NaiveDate::from_ymd(year, month.name.num() as u32, d as u32);
                month.days.push(Day {
                    date: Utc.from_utc_date(&date),
                    events: Vec::new(),
                });
            }
        }

        y
    }

    pub fn num(&self) -> i32 {
        self.year
    }

    pub fn month(&self, month: MonthName) -> &Month<Utc> {
        &self.months[month.ord() as usize]
    }

    pub fn month_mut(&mut self, month: MonthName) -> &mut Month<Utc> {
        &mut self.months[month.ord() as usize]
    }
}
