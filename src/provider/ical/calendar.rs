use chrono::{Date, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz;
use log;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, one_of},
    combinator::{all_consuming, map_res, opt},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::collections::BTreeMap;
use std::convert::{From, TryFrom};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ::ical::parser::ical::IcalParser;
use ::ical::parser::ical::{component::IcalCalendar, component::IcalEvent};
use ::ical::parser::Component;
use ::ical::property::Property;

use uuid;

use crate::config::CalendarSpec;
use crate::provider::*;

use super::{
    Error, ErrorKind, PropertyList, Result, ICAL_FILE_EXT, ISO8601_2004_LOCAL_FORMAT,
    ISO8601_2004_LOCAL_FORMAT_DATE,
};

#[derive(Default, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct IcalDuration {
    sign: i8,
    years: i64,
    months: i64,
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl IcalDuration {
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

impl FromStr for IcalDuration {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (rest, sign) = Self::parse_sign(s)
            .or_else(|err| {
                return Err(Self::Err::new(
                    ErrorKind::DurationParse,
                    &format!("{}", err),
                ));
            })
            .unwrap();

        let (_, mut duration) = (all_consuming(preceded(
            char('P'),
            alt((Self::parse_week_format, Self::parse_datetime_format)),
        ))(rest))
        .or_else(|err| {
            return Err(Self::Err::new(
                ErrorKind::DurationParse,
                &format!("{}", err),
            ));
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

impl TryFrom<&Property> for IcalDuration {
    type Error = Error;

    fn try_from(value: &Property) -> Result<Self> {
        let val = value
            .value
            .as_ref()
            .ok_or(Error::new(ErrorKind::EventParse, "Empty duration property"))?;

        val.parse::<Self>()
    }
}

impl<Tz: TimeZone> TryFrom<&PropertyList> for Occurrence<Tz> {
    type Error = Error;

    fn try_from(value: &PropertyList) -> Result<Self> {
        let dtstart = value
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(ErrorKind::EventMissingKey, "No DTSTART found"))?;

        let dtend = value.iter().find(|p| p.name == "DTEND");

        // Required (if METHOD not set)
        let dtstart_spec = IcalDateTime::try_from(dtstart)?;

        // DTEND does not HAVE to be specified...
        if let Some(dt) = dtend {
            // ...but if set it must be parseable
            let dtend_spec = IcalDateTime::try_from(dt)?;
            return Ok(Occurrence::Onetime(TimeSpan::from_start_and_end(
                dtstart_spec,
                dtend_spec,
            )));
        };

        // Check if DURATION is set
        let duration = value.iter().find(|p| p.name == "DURATION");

        if let Some(duration) = duration {
            let dur_spec = IcalDuration::try_from(duration)?;
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
            date @ IcalDateTime::Date(_) => Ok(Occurrence::Allday(date)),
            dt => Ok(Occurrence::Instant(dt)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcalDateTime {
    Date(NaiveDate),
    Floating(NaiveDateTime),
    Utc(DateTime<Utc>),
    Local(DateTime<FixedOffset>),
}

impl FromStr for IcalDateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
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

impl TryFrom<&Property> for IcalDateTime {
    type Error = Error;

    fn try_from(value: &Property) -> Result<Self> {
        let val = value
            .value
            .as_ref()
            .ok_or(Self::Error::from(ErrorKind::DateParse).with_msg("Missing datetime value"))?;

        let mut spec = val.parse::<Self>()?;

        // check for TZID in options
        if let Some(options) = &value.params {
            if let Some(option) = options.iter().find(|o| o.0 == "TZID") {
                let tz: chrono_tz::Tz = option.1[0]
                    .parse()
                    .map_err(|err: String| Error::new(ErrorKind::DateParse, err.as_str()))?;
                spec = spec.with_tz(&tz);
            }
        }

        Ok(spec)
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for IcalDateTime {
    fn from(dt: DateTime<Tz>) -> Self {
        let fixed_offset = dt.offset().fix();
        let n_dt = dt.with_timezone(&fixed_offset);

        if fixed_offset.utc_minus_local() == 0 {
            IcalDateTime::Utc(dt.with_timezone(&Utc {}))
        } else {
            IcalDateTime::Local(n_dt)
        }
    }
}

impl Default for IcalDateTime {
    fn default() -> Self {
        IcalDateTime::Floating(NaiveDateTime::from_timestamp(0, 0))
    }
}

impl IcalDateTime {
    pub fn is_date(&self) -> bool {
        use IcalDateTime::*;
        match *self {
            Date(_) => true,
            _ => false,
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        match *self {
            IcalDateTime::Date(dt) => tz.from_utc_date(&dt).and_hms(0, 0, 0),
            IcalDateTime::Floating(dt) => tz.from_utc_datetime(&dt),
            IcalDateTime::Utc(dt) => dt.with_timezone(&tz),
            IcalDateTime::Local(dt) => dt.with_timezone(tz),
        }
    }

    pub fn as_date<Tz: TimeZone>(&self, tz: &Tz) -> Date<Tz> {
        match *self {
            IcalDateTime::Date(dt) => tz.from_utc_date(&dt),
            IcalDateTime::Floating(dt) => tz.from_utc_date(&dt.date()),
            IcalDateTime::Utc(dt) => dt.with_timezone(tz).date(),
            IcalDateTime::Local(dt) => dt.with_timezone(tz).date(),
        }
    }

    pub fn with_tz(self, tz: &chrono_tz::Tz) -> Self {
        match self {
            IcalDateTime::Date(dt) => {
                let offset = tz.offset_from_utc_date(&dt).fix();
                IcalDateTime::Local(offset.from_utc_date(&dt).and_hms(0, 0, 0))
            }
            IcalDateTime::Floating(dt) => {
                let offset = tz.offset_from_utc_datetime(&dt).fix();
                IcalDateTime::Local(offset.from_utc_datetime(&dt))
            }
            IcalDateTime::Utc(dt) => IcalDateTime::Local(
                dt.with_timezone(&tz.offset_from_utc_datetime(&dt.naive_utc()).fix()),
            ),
            IcalDateTime::Local(dt) => IcalDateTime::Local(dt.with_timezone(&tz.offset_from_utc_datetime(&dt.naive_utc()).fix())),
        }
    }

    pub fn and_duration(self, duration: chrono::Duration) -> Self {
        match self {
            IcalDateTime::Date(dt) => IcalDateTime::Date(dt + duration),
            IcalDateTime::Floating(dt) => IcalDateTime::Floating(dt + duration),
            IcalDateTime::Utc(dt) => IcalDateTime::Utc(dt + duration),
            IcalDateTime::Local(dt) => IcalDateTime::Local(dt + duration),
        }
    }
}

#[derive(Clone)]
pub struct Event<Tz: TimeZone = Local> {
    path: PathBuf,
    occurrence: Occurrence<Tz>,
    ical: IcalCalendar,
}

impl TryFrom<IcalCalendar> for Event {
    type Error = Error;

    fn try_from(ev: IcalCalendar) -> Result<Self> {
        let occur = Occurrence::try_from(&ev.properties)?;

        Ok(Event {
            path: PathBuf::new(),
            occurrence: occur,
            ical: ev,
        })
    }
}

impl<Tz: TimeZone> Event<Tz> {
    pub fn new(path: &Path, occurrence: Occurrence<Tz>) -> Result<Self> {
        if path.is_file() && path.exists() {
            return Err(Error::new(
                ErrorKind::EventParse,
                &format!("File '{}' already exists", path.display()),
            ));
        }

        let uid = if path.is_file() {
            // TODO: Error handling
            uuid::Uuid::parse_str(&path.file_stem().unwrap().to_string_lossy().to_string()).unwrap()
        } else {
            uuid::Uuid::new_v4()
        };

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

        let mut ical_event = IcalEvent::new();
        ical_event.properties = vec![
            Property {
                name: "UID".to_owned(),
                params: None,
                value: Some(uid.to_string()),
            },
            Property {
                name: "DTSTAMP".to_owned(),
                params: None,
                value: Some(super::generate_timestamp()),
            },
        ];
        ical_calendar.events.push(ical_event);

        Ok(Event {
            path: if path.is_file() {
                path.to_owned()
            } else {
                path.join(&uid.to_string()).with_extension(ICAL_FILE_EXT)
            },
            occurrence,
            ical: ical_calendar,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let buf = io::BufReader::new(fs::File::open(path)?);

        let mut reader = IcalParser::new(buf);

        let ical: IcalCalendar = match reader.next() {
            Some(cal) => match cal {
                Ok(c) => c,
                Err(e) => {
                    return Err(Error::from(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "No calendar could be read from '{p}': {e}",
                            p = path.display(),
                            e = e
                        ),
                    )))
                }
            },
            None => {
                return Err(Error::from(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("No calendar found in '{}'", path.display()),
                )))
            }
        };

        Self::from_ical(path, ical)
    }

    pub fn from_ical(path: &Path, mut ical: IcalCalendar) -> Result<Self> {
        if ical.events.len() >= 1 {
            return Err(Error::from(ErrorKind::CalendarParse).with_msg(&format!(
                "Calendar '{}' has more than one event entry",
                path.display()
            )));
        }

        if ical.events.is_empty() {
            return Err(Error::from(ErrorKind::CalendarParse)
                .with_msg(&format!("Calendar '{}' has no event entry", path.display())));
        }

        let event = &ical.events.first().unwrap();

        let occurrence = Occurrence::try_from(&event.properties)?;

        // TODO: Parse timezone

        Ok(Event {
            path: path.into(),
            occurrence,
            ical,
        })
    }

    pub fn new_with_ical_properties(
        path: &Path,
        occurrence: Occurrence<Tz>,
        properties: PropertyList,
    ) -> Result<Self> {
        let mut event = Self::new(path, occurrence)?;

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

        event.ical.events[0].properties.extend(new_properties);

        Ok(event)
    }

    fn get_property_value(&self, name: &str) -> Option<&str> {
        if let Some(prop) = self.ical.properties.iter().find(|prop| prop.name == name) {
            prop.value.as_deref()
        } else {
            None
        }
    }

    fn get_property_mut(&self, name: &str) -> Option<&mut Property> {
        self.ical
            .properties
            .iter_mut()
            .find(|prop| prop.name == name)
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

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical.events[0]
    }
}

impl<Tz: TimeZone> Eventlike<Tz> for Event<Tz> {
    fn title(&self) -> &str {
        self.get_property_value("SUMMARY").unwrap()
    }

    fn set_title(&mut self, title: &str) {
        if let Some(property) = self.get_property_mut("SUMMARY") {
            property.value = Some(title.to_owned());
        } else {
            self.ical.events[0].add_property(Property {
                name: "SUMMARY".to_owned(),
                params: None,
                value: Some(title.to_owned()),
            });
        };
    }

    fn uuid(&self) -> Uuid {
        uuid::Uuid::parse_str(self.get_property_value("UID").unwrap()).unwrap()
    }

    fn summary(&self) -> &str {
        self.title()
    }

    fn set_summary(&mut self, summary: &str) {
        self.set_title(summary);
    }

    fn occurrence(&self) -> &Occurrence<Tz> {
        &self.occurrence
    }

    fn set_occurrence(&mut self, occurrence: Occurrence<Tz>) {
        // TODO: implement
    }

    fn begin(&self) -> DateTime<Tz> {
        self.occurrence.begin()
    }

    fn end(&self) -> DateTime<Tz> {
        self.occurrence.end()
    }

    fn duration(&self) -> Duration {
        self.occurrence.duration().into()
    }
}

impl From<Event> for IcalEvent {
    fn from(event: Event) -> Self {
        event.ical.events[0]
    }
}

impl From<Event> for IcalCalendar {
    fn from(event: Event) -> Self {
        event.ical
    }
}

pub struct Calendar<Tz: TimeZone = Local> {
    path: PathBuf,
    identifier: String,
    friendly_name: String,
    events: BTreeMap<DateTime<Tz>, Vec<Event<Tz>>>,
}

impl<Tz: TimeZone> Calendar<Tz> {
    pub fn new(path: &Path) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();
        let friendly_name = identifier.clone();

        Self {
            path: path.to_owned(),
            identifier: identifier.to_string(),
            friendly_name: friendly_name.to_string(),
            events: BTreeMap::new(),
        }
    }

    pub fn new_with_name(path: &Path, name: String) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();

        Self {
            path: path.to_owned(),
            identifier: identifier.to_string(),
            friendly_name: name,
            events: BTreeMap::new(),
        }
    }

    pub fn from_dir(path: &Path) -> Result<Self> {
        let events = BTreeMap::<DateTime<Tz>, Vec<Event<Tz>>>::new();

        if !path.is_dir() {
            return Err(Error::new(
                ErrorKind::CalendarParse,
                &format!("'{}' is not a directory", path.display()),
            ));
        }

        let event_file_iter = fs::read_dir(&path)?
            .map(|dir| {
                dir.map_or_else(
                    |_| -> Result<_> { Err(Error::from(ErrorKind::CalendarParse)) },
                    |file: fs::DirEntry| -> Result<Event<Tz>> {
                        Event::<Tz>::from_file(file.path().as_path())
                    },
                )
            })
            .inspect(|res| {
                if let Err(err) = res {
                    log::warn!("{}", err)
                }
            })
            .filter_map(Result::ok);

        for event in event_file_iter {
            events.entry(event.begin()).or_default().push(event);
        }

        Ok(Calendar {
            path: path.to_owned(),
            identifier: path.file_stem().unwrap().to_string_lossy().to_string(),
            friendly_name: String::default(),
            events,
        })
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.set_name(name);
        self
    }

    pub fn set_name(&mut self, name: String) {
        self.friendly_name = name;
    }
}

impl<Tz: TimeZone> Calendarlike<Tz> for Calendar<Tz> {
    fn name(&self) -> &str {
        &self.friendly_name
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn event_iter(&self) -> Box<dyn Iterator<Item = &dyn Eventlike<Tz>>> {
        Box::new(
            self.events
                .iter()
                .flat_map(|(_, v)| v.iter())
                .map(|ev| (ev as &dyn Eventlike<Tz>)),
        )
    }

    fn new_event(&mut self) {
        unimplemented!()
    }
}

pub struct Collection<Tz: TimeZone = Local> {
    path: PathBuf,
    friendly_name: String,
    calendars: Vec<Calendar<Tz>>,
}

impl<Tz: TimeZone> Collection<Tz> {
    pub fn from_dir(path: &Path) -> Result<Self> {
        if !path.is_dir() {
            return Err(Error::new(
                ErrorKind::CalendarParse,
                &format!("'{}' is not a directory", path.display()),
            ));
        }

        let calendars: Vec<Calendar<Tz>> = fs::read_dir(&path)?
            .map(|dir| {
                dir.map_or_else(
                    |_| -> Result<_> { Err(Error::from(io::ErrorKind::InvalidData)) },
                    |file: fs::DirEntry| -> Result<Calendar<Tz>> {
                        Calendar::<Tz>::from_dir(file.path().as_path())
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
            path: path.to_owned(),
            friendly_name: path.file_stem().unwrap().to_string_lossy().to_string(),
            calendars,
        })
    }

    pub fn calendars_from_dir(path: &Path, calendar_specs: &[CalendarSpec]) -> Result<Self> {
        if !path.is_dir() {
            return Err(Error::new(
                ErrorKind::CalendarParse,
                &format!("'{}' is not a directory", path.display()),
            ));
        }

        if calendar_specs.is_empty() {
            return Self::from_dir(path);
        }

        let calendars: Vec<Calendar<Tz>> = calendar_specs
            .into_iter()
            .filter_map(|spec| match Calendar::<Tz>::from_dir(&path.join(spec.id)) {
                Ok(calendar) => Some(calendar.with_name(spec.name)),
                Err(_) => None,
            })
            .collect();

        Ok(Collection {
            path: path.to_owned(),
            friendly_name: path.file_stem().unwrap().to_string_lossy().to_string(),
            calendars,
        })
    }
}

impl<Tz: TimeZone> Collectionlike<Tz> for Collection<Tz> {
    fn name(&self) -> &str {
        &self.friendly_name
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn calendar_iter(&self) -> Box<dyn Iterator<Item = &dyn Calendarlike<Tz>>> {
        Box::new(self.calendars.iter().map(|c| c as &dyn Calendarlike<Tz>))
    }

    fn event_iter(&self) -> Box<dyn Iterator<Item = &dyn Eventlike<Tz>>> {
        Box::new(self.calendars.iter().flat_map(|c| c.event_iter()))
    }

    fn new_calendar(&mut self) {}
}
