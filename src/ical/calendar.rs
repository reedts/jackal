use chrono::{
    Date, DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc,
};
use chrono_tz::Tz;
use ical::parser::ical::component::IcalCalendar;
use ical::parser::ical::IcalParser;
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ical::parser::ical::component::IcalEvent;
use ical::property::Property;

use crate::ical::{Error, ErrorKind};

use super::{ISO8601_2004_LOCAL_FORMAT, ISO8601_2004_LOCAL_FORMAT_DATE};

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TimeSpan<'a> {
    begin: &'a DateTimeSpec,
    end: &'a DateTimeSpec,
}

#[derive(Clone)]
pub enum OccurrenceSpec {
    Allday(DateTimeSpec),
    Onetime(DateTimeSpec, DateTimeSpec),
}

impl OccurrenceSpec {
    pub fn is_allday(&self) -> bool {
        use OccurrenceSpec::*;
        matches!(self, Allday(_))
    }

    pub fn is_onetime(&self) -> bool {
        use OccurrenceSpec::*;
        matches!(self, Onetime(_, _))
    }

    pub fn as_date<Tz: TimeZone>(&self, tz: &Tz) -> Date<Tz> {
        use OccurrenceSpec::*;
        match self {
            Allday(date) => date.as_date(tz),
            Onetime(datetime, _) => datetime.as_date(tz),
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> DateTime<Tz> {
        use OccurrenceSpec::*;
        match self {
            Allday(date) => date
                .as_date(tz)
                .and_time(NaiveTime::from_hms(0, 0, 0))
                .unwrap(),
            Onetime(datetime, _) => datetime.as_datetime(tz).clone(),
        }
    }

    pub fn begin<Tz: TimeZone>(&self, tz: &Tz) -> DateTime<Tz> {
        use OccurrenceSpec::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(0, 0, 0),
            Onetime(begin, _) => begin.as_datetime(tz).clone(),
        }
    }

    pub fn end<Tz: TimeZone>(&self, tz: &Tz) -> DateTime<Tz> {
        use OccurrenceSpec::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(23, 59, 59),
            Onetime(_, end) => end.as_datetime(tz).clone(),
        }
    }
}

impl TryFrom<&IcalEvent> for OccurrenceSpec {
    type Error = super::error::Error;

    fn try_from(value: &IcalEvent) -> Result<Self, Self::Error> {
        let dtstart = value
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(ErrorKind::EventMissingKey).with_msg("No DTSTART found"))?;

        let dtend = value
            .properties
            .iter()
            .find(|p| p.name == "DTEND")
            .ok_or(Error::new(ErrorKind::EventMissingKey).with_msg("No DTEND found"))?;

        let dtstart_spec = DateTimeSpec::try_from(dtstart)?;
        if let Some(dtend_spec) = DateTimeSpec::try_from(dtstart).ok() {
            Ok(OccurrenceSpec::Onetime(dtstart_spec, dtend_spec))
        } else {
            Ok(OccurrenceSpec::Allday(dtstart_spec))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DateTimeSpec {
    Date(NaiveDate),
    Floating(NaiveDateTime),
    Utc(DateTime<Utc>),
    Local(DateTime<FixedOffset>),
}

impl FromStr for DateTimeSpec {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // First try to read DateTime, if that fails try date only
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT) {
            if s.ends_with("Z") {
                Ok(Self::Utc(DateTime::<Utc>::from_utc(dt, Utc)))
            } else {
                Ok(Self::Floating(dt))
            }
        } else {
            let date = NaiveDate::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT_DATE)?;
            Ok(Self::Date(date))
        }
    }
}

impl TryFrom<&Property> for DateTimeSpec {
    type Error = super::error::Error;

    fn try_from(value: &Property) -> Result<Self, Self::Error> {
        let val = value
            .value
            .as_ref()
            .ok_or(Self::Error::from(ErrorKind::DateParse).with_msg("Missing datetime value"))?;

        let mut spec = val.parse::<Self>()?;

        // check for TZID in options
        if let Some(options) = &value.params {
            if let Some(option) = options.iter().find(|o| o.0 == "TZID") {
                let tz: Tz = option.1[0].parse().map_err(|err: String| {
                    Error::new(ErrorKind::DateParse).with_msg(err.as_str())
                })?;
                spec = spec.with_tz(&tz);
            }
        }

        Ok(spec)
    }
}

impl DateTimeSpec {
    pub fn is_date(&self) -> bool {
        use DateTimeSpec::*;
        match *self {
            Date(_) => true,
            _ => false,
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> DateTime<Tz> {
        match *self {
            DateTimeSpec::Date(dt) => tz.from_utc_date(&dt).and_hms(0, 0, 0),
            DateTimeSpec::Floating(dt) => tz.from_utc_datetime(&dt),
            DateTimeSpec::Utc(dt) => dt.with_timezone(&tz),
            DateTimeSpec::Local(dt) => dt.with_timezone(&tz),
        }
    }

    pub fn as_date<Tz: TimeZone>(&self, tz: &Tz) -> Date<Tz> {
        match *self {
            DateTimeSpec::Date(dt) => tz.from_utc_date(&dt),
            DateTimeSpec::Floating(dt) => tz.from_utc_date(&dt.date()),
            DateTimeSpec::Utc(dt) => dt.with_timezone(tz).date(),
            DateTimeSpec::Local(dt) => dt.with_timezone(tz).date(),
        }
    }

    pub fn with_tz<Tz: TimeZone>(self, tz: &Tz) -> Self {
        match self {
            DateTimeSpec::Date(dt) => {
                let offset = tz.offset_from_utc_date(&dt).fix();
                DateTimeSpec::Local(offset.from_utc_date(&dt).and_hms(0, 0, 0))
            }
            DateTimeSpec::Floating(dt) => {
                let offset = tz.offset_from_utc_datetime(&dt).fix();
                DateTimeSpec::Local(offset.from_utc_datetime(&dt))
            }
            DateTimeSpec::Utc(dt) => DateTimeSpec::Local(
                dt.with_timezone(&tz.offset_from_utc_datetime(&dt.naive_utc()).fix()),
            ),
            DateTimeSpec::Local(dt) => DateTimeSpec::Local(dt),
        }
    }
}

#[derive(Clone)]
pub struct Event {
    occur: OccurrenceSpec,
    ical_event: IcalEvent,
}

impl TryFrom<&IcalEvent> for Event {
    type Error = super::error::Error;

    fn try_from(ev: &IcalEvent) -> Result<Self, Self::Error> {
        let occur = OccurrenceSpec::try_from(ev)?;

        Ok(Event {
            occur,
            ical_event: ev.clone(),
        })
    }
}

impl Event {
    pub fn summary(&self) -> &str {
        self.ical_event
            .properties
            .iter()
            .find(|prop| prop.name == "SUMMARY")
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .as_str()
    }

    pub fn occurence(&self) -> &OccurrenceSpec {
        &self.occur
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical_event
    }

    pub fn is_allday(&self) -> bool {
        self.occur.is_allday()
    }
}

#[derive(Clone)]
pub struct Calendar {
    path: PathBuf,
    ical: IcalCalendar,
    events: Vec<Event>,
    tz: Tz,
}

impl TryFrom<&Path> for Calendar {
    type Error = io::Error;
    fn try_from(path: &Path) -> io::Result<Calendar> {
        let buf = io::BufReader::new(fs::File::open(path)?);

        let mut reader = IcalParser::new(buf);

        let ical: IcalCalendar = match reader.next() {
            Some(cal) => match cal {
                Ok(c) => c,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "No calendar could be read from '{p}': {e}",
                            p = path.display(),
                            e = e
                        ),
                    ))
                }
            },
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("No calendar found in '{}'", path.display()),
                ))
            }
        };

        let events: Vec<Event> = ical
            .events
            .iter()
            .map(|ev| Event::try_from(ev))
            .inspect(|ev| {
                if let Err(e) = ev {
                    println!("ERROR: {:?} (in '{}')", e, path.display())
                }
            })
            .filter_map(Result::ok)
            .collect();

        // TODO: Actually parse timezone

        Ok(Calendar {
            path: path.into(),
            ical,
            events,
            tz: Tz::UTC,
        })
    }
}

impl Calendar {
    pub fn events(&self) -> &Vec<Event> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
    }
}

#[derive(Clone)]
pub struct Collection<'a> {
    path: &'a Path,
    friendly_name: Option<&'a str>,
    tz: Tz,
    calendars: Vec<Calendar>,
}

impl<'a> TryFrom<&'a Path> for Collection<'a> {
    type Error = io::Error;
    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        // Load all valid .ics files from 'path'
        let mut calendars: Vec<Calendar> = fs::read_dir(path)?
            .map(|dir| {
                dir.map_or_else(
                    |_| -> io::Result<_> { Err(io::Error::from(io::ErrorKind::NotFound)) },
                    |file: fs::DirEntry| -> io::Result<Calendar> {
                        Calendar::try_from(file.path().as_path())
                    },
                )
            })
            .filter_map(Result::ok)
            .collect();

        Ok(Collection {
            path,
            friendly_name: None,
            tz: Tz::UTC,
            calendars,
        })
    }
}
