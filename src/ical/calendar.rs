use chrono::{Date, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use log;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, one_of},
    combinator::{all_consuming, map_res, opt},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::convert::{From, TryFrom};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ical::parser::ical::IcalParser;
use ical::parser::ical::{component::IcalCalendar, component::IcalEvent};
use ical::parser::Component;
use ical::property::Property;

use uuid;

use super::{
    Error, ErrorKind, IcalResult, PropertyList, ISO8601_2004_LOCAL_FORMAT,
    ISO8601_2004_LOCAL_FORMAT_DATE,
};

#[derive(Default, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Duration {
    sign: i8,
    years: i64,
    months: i64,
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl Duration {
    fn parse_sign(input: &str) -> IResult<&str, Option<char>> {
        opt(one_of("+-"))(input)
    }

    fn parse_integer_value(input: &str) -> std::result::Result<i64, std::num::ParseIntError> {
        i64::from_str_radix(input, 10)
    }

    fn value_with_designator(designator: &str) -> impl Fn(&str) -> IResult<&str, i64> + '_ {
        move |input| {
            terminated(
                map_res(digit1, |s: &str| Self::parse_integer_value(s)),
                tag(designator),
            )(input)
        }
    }

    fn parse_week_format(input: &str) -> IResult<&str, Self> {
        let (input, weeks) = (Self::value_with_designator("W")(input))?;

        Ok((
            input,
            Self {
                sign: 1,
                years: 0,
                months: 0,
                weeks,
                days: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            },
        ))
    }

    fn parse_datetime_format(input: &str) -> IResult<&str, Self> {
        let (input, (years, months, days)) = tuple((
            opt(Self::value_with_designator("Y")),
            opt(Self::value_with_designator("M")),
            opt(Self::value_with_designator("D")),
        ))(input)?;

        let (input, time) = opt(preceded(
            char('T'),
            tuple((
                opt(Self::value_with_designator("H")),
                opt(Self::value_with_designator("M")),
                opt(Self::value_with_designator("S")),
            )),
        ))(input)?;

        let (hours, minutes, seconds) = time.unwrap_or_default();

        if years.is_none()
            && months.is_none()
            && days.is_none()
            && hours.is_none()
            && minutes.is_none()
            && seconds.is_none()
        {
            Err(nom::Err::Error(nom::error::ParseError::from_error_kind(
                input,
                nom::error::ErrorKind::Verify,
            )))
        } else {
            Ok((
                input,
                Self {
                    sign: 1,
                    years: years.unwrap_or_default(),
                    months: months.unwrap_or_default(),
                    weeks: 0,
                    days: days.unwrap_or_default(),
                    hours: hours.unwrap_or_default(),
                    minutes: minutes.unwrap_or_default(),
                    seconds: seconds.unwrap_or_default(),
                },
            ))
        }
    }

    fn as_chrono_duration(&self) -> chrono::Duration {
        chrono::Duration::seconds(
            self.sign as i64
                * ((self.years * 12 * 30 * 24 * 60 * 60)
                    + (self.months * 30 * 24 * 60 * 60)
                    + (self.weeks * 7 * 24 * 60 * 60)
                    + (self.hours * 60 * 60)
                    + (self.minutes * 60)
                    + (self.seconds)),
        )
    }
}

impl FromStr for Duration {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, sign) = Self::parse_sign(s)
            .or_else(|err| {
                return Err(Self::Err::new(ErrorKind::DurationParse).with_msg(&format!("{}", err)));
            })
            .unwrap();

        let (_, mut duration) = (all_consuming(preceded(
            char('P'),
            alt((Self::parse_week_format, Self::parse_datetime_format)),
        ))(rest))
        .or_else(|err| {
            return Err(Self::Err::new(ErrorKind::DurationParse).with_msg(&format!("{}", err)));
        })
        .unwrap();

        duration.sign = if let Some(sign) = sign {
            if sign == '-' {
                -1
            } else {
                1
            }
        } else {
            1
        };

        Ok(duration)
    }
}

impl TryFrom<&Property> for Duration {
    type Error = super::error::Error;

    fn try_from(value: &Property) -> Result<Self, Self::Error> {
        let val = value
            .value
            .as_ref()
            .ok_or(Self::Error::from(ErrorKind::EventParse).with_msg("Empty duration property"))?;

        val.parse::<Self>()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeSpan {
    TimePoints(DateTime, DateTime),
    Duration(DateTime, Duration),
}

impl TimeSpan {
    pub fn from_start_and_end(begin: DateTime, end: DateTime) -> Self {
        TimeSpan::TimePoints(begin, end)
    }

    pub fn from_start_and_duration(begin: DateTime, end: Duration) -> Self {
        TimeSpan::Duration(begin, end)
    }

    pub fn begin(&self) -> DateTime {
        match &self {
            TimeSpan::TimePoints(begin, _) => begin.clone(),
            TimeSpan::Duration(begin, _) => begin.clone(),
        }
    }

    pub fn end(&self) -> DateTime {
        match &self {
            TimeSpan::TimePoints(_, end) => end.clone(),
            TimeSpan::Duration(begin, dur) => begin.clone().and_duration(dur.as_chrono_duration()),
        }
    }
}

#[derive(Clone)]
pub enum Occurrence {
    Allday(DateTime),
    Onetime(TimeSpan),
    Instant(DateTime),
}

impl Default for Occurrence {
    fn default() -> Self {
        Occurrence::Instant(DateTime::default())
    }
}

impl Occurrence {
    pub fn is_allday(&self) -> bool {
        use Occurrence::*;
        matches!(self, Allday(_))
    }

    pub fn is_onetime(&self) -> bool {
        use Occurrence::*;
        matches!(self, Onetime(_))
    }

    pub fn as_date<Tz: TimeZone>(&self, tz: &Tz) -> Date<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz),
            Onetime(timespan) => timespan.begin().as_date(tz),
            Instant(datetime) => datetime.as_date(tz),
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date
                .as_date(tz)
                .and_time(NaiveTime::from_hms(0, 0, 0))
                .unwrap(),
            Onetime(timespan) => timespan.begin().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }

    pub fn begin<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(0, 0, 0),
            Onetime(timespan) => timespan.begin().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }

    pub fn end<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(23, 59, 59),
            Onetime(timespan) => timespan.end().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }
}

impl TryFrom<&PropertyList> for Occurrence {
    type Error = super::error::Error;

    fn try_from(value: &PropertyList) -> Result<Self, Self::Error> {
        let dtstart = value
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(ErrorKind::EventMissingKey).with_msg("No DTSTART found"))?;

        let dtend = value.iter().find(|p| p.name == "DTEND");

        // Required (if METHOD not set)
        let dtstart_spec = DateTime::try_from(dtstart)?;

        // DTEND does not HAVE to be specified...
        if let Some(dt) = dtend {
            // ...but if set it must be parseable
            let dtend_spec = DateTime::try_from(dt)?;
            return Ok(Occurrence::Onetime(TimeSpan::from_start_and_end(
                dtstart_spec,
                dtend_spec,
            )));
        };

        // Check if DURATION is set
        let duration = value.iter().find(|p| p.name == "DURATION");

        if let Some(duration) = duration {
            let dur_spec = Duration::try_from(duration)?;
            return Ok(Occurrence::Onetime(TimeSpan::from_start_and_duration(
                dtstart_spec,
                dur_spec,
            )));
        };

        // If neither DTEND, nor DURATION is specified event duration depends solely
        // on DTSTART. RFC 5545 states, that if DTSTART is...
        //  ... a date spec, the event has to have the duration of a single day
        //  ... a datetime spec, the event has to have the dtstart also as dtend
        match dtstart_spec {
            date @ DateTime::Date(_) => Ok(Occurrence::Allday(date)),
            dt => Ok(Occurrence::Instant(dt)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateTime {
    Date(NaiveDate),
    Floating(NaiveDateTime),
    Utc(chrono::DateTime<Utc>),
    Local(chrono::DateTime<FixedOffset>, Tz),
}

impl FromStr for DateTime {
    type Err = super::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // First try to read DateTime, if that fails try date only
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT) {
            if s.ends_with("Z") {
                Ok(Self::Utc(chrono::DateTime::<Utc>::from_utc(dt, Utc)))
            } else {
                Ok(Self::Floating(dt))
            }
        } else {
            let date = NaiveDate::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT_DATE)?;
            Ok(Self::Date(date))
        }
    }
}

impl TryFrom<&Property> for DateTime {
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

impl Default for DateTime {
    fn default() -> Self {
        DateTime::Floating(NaiveDateTime::from_timestamp(0, 0))
    }
}

impl DateTime {
    pub fn is_date(&self) -> bool {
        use DateTime::*;
        match *self {
            Date(_) => true,
            _ => false,
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        match *self {
            DateTime::Date(dt) => tz.from_utc_date(&dt).and_hms(0, 0, 0),
            DateTime::Floating(dt) => tz.from_utc_datetime(&dt),
            DateTime::Utc(dt) => dt.with_timezone(&tz),
            DateTime::Local(dt, old_tz) => dt.with_timezone(&old_tz).with_timezone(tz),
        }
    }

    pub fn as_date<Tz: TimeZone>(&self, tz: &Tz) -> Date<Tz> {
        match *self {
            DateTime::Date(dt) => tz.from_utc_date(&dt),
            DateTime::Floating(dt) => tz.from_utc_date(&dt.date()),
            DateTime::Utc(dt) => dt.with_timezone(tz).date(),
            DateTime::Local(dt, old_tz) => dt.with_timezone(&old_tz).with_timezone(tz).date(),
        }
    }

    pub fn with_tz(self, tz: &Tz) -> Self {
        match self {
            DateTime::Date(dt) => {
                let offset = tz.offset_from_utc_date(&dt).fix();
                DateTime::Local(offset.from_utc_date(&dt).and_hms(0, 0, 0), tz.clone())
            }
            DateTime::Floating(dt) => {
                let offset = tz.offset_from_utc_datetime(&dt).fix();
                DateTime::Local(offset.from_utc_datetime(&dt), tz.clone())
            }
            DateTime::Utc(dt) => DateTime::Local(
                dt.with_timezone(&tz.offset_from_utc_datetime(&dt.naive_utc()).fix()),
                tz.clone(),
            ),
            DateTime::Local(dt, _) => DateTime::Local(dt, tz.clone()),
        }
    }

    pub fn and_duration(self, duration: chrono::Duration) -> Self {
        match self {
            DateTime::Date(dt) => DateTime::Date(dt + duration),
            DateTime::Floating(dt) => DateTime::Floating(dt + duration),
            DateTime::Utc(dt) => DateTime::Utc(dt + duration),
            DateTime::Local(dt, tz) => DateTime::Local(dt + duration, tz),
        }
    }
}

#[derive(Clone)]
pub struct Event {
    occur: Occurrence,
    ical: IcalEvent,
}

impl TryFrom<IcalEvent> for Event {
    type Error = super::error::Error;

    fn try_from(ev: IcalEvent) -> Result<Self, Self::Error> {
        let occur = Occurrence::try_from(&ev.properties)?;

        Ok(Event { occur, ical: ev })
    }
}

impl Event {
    pub fn new(occurrence: Occurrence) -> Self {
        let mut ical_event = IcalEvent::new();
        ical_event.properties = vec![
            Property {
                name: "UID".to_owned(),
                params: None,
                value: Some(uuid::Uuid::new_v4().hyphenated().to_string()),
            },
            Property {
                name: "DTSTAMP".to_owned(),
                params: None,
                value: Some(super::generate_timestamp()),
            },
        ];

        Event {
            occur: occurrence,
            ical: ical_event,
        }
    }

    pub fn new_with_ical_properties(occurrence: Occurrence, properties: PropertyList) -> Self {
        let mut event = Self::new(occurrence);

        let new_properties: Vec<_> = properties
            .into_iter()
            .filter(|p| {
                event
                    .ical
                    .properties
                    .iter()
                    .find(|v| v.name == p.name)
                    .is_none()
            })
            .collect();

        event.ical.properties.extend(new_properties);

        event
    }

    fn get_property(&self, name: &str) -> Option<&str> {
        if let Some(prop) = self.ical.properties.iter().find(|prop| prop.name == name) {
            prop.value.as_deref()
        } else {
            None
        }
    }

    pub fn uuid(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(self.get_property("UID").unwrap()).unwrap()
    }

    pub fn summary(&self) -> &str {
        self.get_property("SUMMARY").unwrap()
    }

    pub fn set_summary(&mut self, summary: &str) {
        let summary_prop = self
            .ical
            .properties
            .iter_mut()
            .find(|prop| prop.name == "SUMMARY");

        if let Some(prop) = summary_prop {
            prop.value = Some(summary.to_owned());
        } else {
            self.ical.add_property(Property {
                name: "SUMMARY".to_owned(),
                params: None,
                value: Some(summary.to_owned()),
            });
        }
    }

    pub fn occurrence(&self) -> &Occurrence {
        &self.occur
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical
    }

    pub fn is_allday(&self) -> bool {
        self.occur.is_allday()
    }
}

impl From<Event> for IcalEvent {
    fn from(event: Event) -> Self {
        event.ical
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

        Calendar::from_ical(path, ical).map_err(|err| err.into())
    }
}

impl Calendar {
    pub fn new(path: &Path) -> Self {
        let mut ical_calendar = IcalCalendar::new();
        ical_calendar.properties = vec![
            Property {
                name: "PRODID".to_owned(),
                params: None,
                value: Some(super::JACKAL_PRODID.to_owned()),
            },
            Property {
                name: "VERSION".to_owned(),
                params: None,
                value: Some(super::JACKAL_CALENDAR_VERSION.to_owned()),
            },
        ];

        Self {
            path: path.to_owned(),
            ical: ical_calendar,
            events: Vec::new(),
            tz: Tz::UTC,
        }
    }

    pub fn from_ical(path: &Path, mut ical: IcalCalendar) -> IcalResult<Self> {
        let events: Vec<Event> = std::mem::replace(&mut ical.events, Vec::new())
            .into_iter()
            .map(|ev| Event::try_from(ev))
            .inspect(|ev| {
                if let Err(e) = ev {
                    log::warn!("{} (in '{}')", e, path.display())
                }
            })
            .filter_map(Result::ok)
            .collect();

        // TODO: Actually parse timezone

        if events.is_empty() {
            Err(Error::from(ErrorKind::CalendarParse).with_msg("No event found"))
        } else {
            Ok(Calendar {
                path: path.into(),
                ical,
                events,
                tz: Tz::UTC,
            })
        }
    }

    pub fn from_event(path: &Path, mut event: Event) -> Self {
        let mut calendar = Self::new(path);

        calendar.events = vec![event];
        calendar
    }

    pub fn events_iter<'a>(&'a self) -> impl Iterator<Item = &'a Event> {
        self.events.iter()
    }

    pub fn events(&self) -> &Vec<Event> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
    }
}

impl From<Calendar> for IcalCalendar {
    fn from(calendar: Calendar) -> Self {
        let mut ical_calendar = calendar.ical;
        ical_calendar.events = calendar
            .events
            .into_iter()
            .map(|ev| ev.into())
            .collect::<Vec<IcalEvent>>();

        ical_calendar
    }
}

#[derive(Clone)]
pub struct Collection<'a> {
    path: PathBuf,
    friendly_name: Option<&'a str>,
    tz: Tz,
    calendars: Vec<Calendar>,
}

impl<'a> Collection<'a> {
    pub fn event_iter(&'a self) -> impl Iterator<Item = &'a Event> {
        self.calendars.iter().flat_map(|c| c.events_iter())
    }

    pub fn tz(&self) -> &Tz {
        &self.tz
    }

    pub fn calendars(&self) -> &Vec<Calendar> {
        &self.calendars
    }
}

impl TryFrom<&Path> for Collection<'_> {
    type Error = io::Error;
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::try_from(path.to_path_buf())
    }
}

impl TryFrom<PathBuf> for Collection<'_> {
    type Error = io::Error;
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        // Load all valid .ics files from 'path'
        let calendars: Vec<Calendar> = fs::read_dir(&path)?
            .map(|dir| {
                dir.map_or_else(
                    |_| -> io::Result<_> { Err(io::Error::from(io::ErrorKind::NotFound)) },
                    |file: fs::DirEntry| -> io::Result<Calendar> {
                        Calendar::try_from(file.path().as_path())
                    },
                )
            })
            .inspect(|res| {
                if let Err(err) = res {
                    log::warn!("{}", err)
                }
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
