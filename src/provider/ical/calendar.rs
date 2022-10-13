use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use log;
use std::collections::BTreeMap;
use std::convert::{AsRef, From};
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::config::CalendarConfig;
use crate::provider;
use crate::provider::{CalendarMut, Eventlike, NewEvent, Occurrence, TimeSpan};

use super::{
    weekday_to_ical, Error, ErrorKind, PropertyList, Result, ICAL_FILE_EXT,
    ISO8601_2004_LOCAL_FORMAT, ISO8601_2004_LOCAL_FORMAT_DATE,
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

impl From<IcalDuration> for Duration {
    fn from(dur: IcalDuration) -> Self {
        dur.as_chrono_duration()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcalDateTime {
    Date(NaiveDate),
    Floating(NaiveDateTime),
    Utc(DateTime<Utc>),
    Local(DateTime<chrono_tz::Tz>),
}

impl TryFrom<&Property> for IcalDateTime {
    type Error = Error;

    fn try_from(value: &Property) -> Result<Self> {
        let val = value
            .value
            .as_ref()
            .ok_or(Self::Error::from(ErrorKind::DateParse).with_msg("Missing datetime value"))?;

        let has_options = value.params.is_some();
        let mut tz: Option<Tz> = None;

        if has_options {
            // check if value is date
            if let Some(_) = &value
                .params
                .as_ref()
                .unwrap()
                .iter()
                .find(|o| o.0 == "VALUE" && o.1[0] == "DATE")
            {
                return Ok(Self::Date(NaiveDate::parse_from_str(
                    val,
                    ISO8601_2004_LOCAL_FORMAT_DATE,
                )?));
            }

            // check for TZID in options
            if let Some(option) = &value
                .params
                .as_ref()
                .unwrap()
                .iter()
                .find(|o| o.0 == "TZID")
            {
                tz = Some(
                    option.1[0]
                        .parse::<chrono_tz::Tz>()
                        .map_err(|err: String| Error::new(ErrorKind::DateParse, err.as_str()))?,
                )
            };
        }

        if let Ok(dt) = NaiveDateTime::parse_from_str(val, ISO8601_2004_LOCAL_FORMAT) {
            if let Some(tz) = tz {
                Ok(Self::Local(tz.from_local_datetime(&dt).earliest().unwrap()))
            } else {
                if val.ends_with("Z") {
                    Ok(Self::Utc(DateTime::<Utc>::from_utc(dt, Utc)))
                } else {
                    Ok(Self::Floating(dt))
                }
            }
        } else {
            let date = NaiveDate::parse_from_str(val, ISO8601_2004_LOCAL_FORMAT_DATE)?;
            Ok(Self::Date(date))
        }
    }
}

impl FromStr for IcalDateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Ok(dt) =
            NaiveDateTime::parse_from_str(s, &format!("{}z", ISO8601_2004_LOCAL_FORMAT_DATE))
        {
            return Ok(IcalDateTime::Utc(Utc {}.from_utc_datetime(&dt)));
        }

        if let Ok(dt) = NaiveDate::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT_DATE) {
            return Ok(IcalDateTime::Date(dt));
        }

        Err(Error::new(
            ErrorKind::TimeParse,
            &format!("Could not extract datetime from '{}'", s),
        ))
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for IcalDateTime {
    fn from(dt: DateTime<Tz>) -> Self {
        let fixed_offset = dt.offset().fix();

        if fixed_offset.utc_minus_local() == 0 {
            IcalDateTime::Utc(dt.with_timezone(&Utc {}))
        } else {
            // FIXME: There is currently no possibility to recreate a
            // chrono_tz::Tz from a chrono::DateTime<FixedOffset>
            // We use a UTC datetime and rely on the ical::Event to properly
            // catch this case
            IcalDateTime::Utc(dt.with_timezone(&Utc {}))
        }
    }
}

impl Default for IcalDateTime {
    fn default() -> Self {
        IcalDateTime::Floating(NaiveDateTime::from_timestamp(0, 0))
    }
}

impl IcalDateTime {
    pub fn _is_date(&self) -> bool {
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
            IcalDateTime::Local(dt) => dt.with_timezone(&tz),
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

    pub fn _with_tz(self, tz: &chrono_tz::Tz) -> Self {
        match self {
            IcalDateTime::Date(dt) => {
                IcalDateTime::Local(tz.from_utc_datetime(&dt.and_hms(0, 0, 0)))
            }
            IcalDateTime::Floating(dt) => IcalDateTime::Local(tz.from_utc_datetime(&dt)),
            IcalDateTime::Utc(dt) => IcalDateTime::Local(dt.with_timezone(&tz)),
            IcalDateTime::Local(dt) => IcalDateTime::Local(dt.with_timezone(&tz)),
        }
    }

    pub fn _and_duration(self, duration: chrono::Duration) -> Self {
        match self {
            IcalDateTime::Date(dt) => IcalDateTime::Date(dt + duration),
            IcalDateTime::Floating(dt) => IcalDateTime::Floating(dt + duration),
            IcalDateTime::Utc(dt) => IcalDateTime::Utc(dt + duration),
            IcalDateTime::Local(dt) => IcalDateTime::Local(dt + duration),
        }
    }
}

#[derive(Clone)]
pub struct Event {
    path: PathBuf,
    occurrence: Occurrence<Tz>,
    ical: IcalCalendar,
    tz: Tz,
}

impl Event {
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

        if let Tz::UTC = occurrence.timezone() {
            ()
        } else {
            // push timezone information
            let mut tz_spec = IcalTimeZone::new();
            tz_spec.add_property(Property {
                name: "TZID".to_owned(),
                params: None,
                value: Some(occurrence.begin().offset().tz_id().to_string()),
            });

            tz_spec.add_property(Property {
                name: "TZNAME".to_owned(),
                params: None,
                value: Some(occurrence.begin().offset().abbreviation().to_string()),
            });

            let tz_info = tz::TimeZone::from_posix_tz(occurrence.begin().offset().tz_id())?;

            if let Some(rule) = tz_info.as_ref().extra_rule() {
                match rule {
                    TransitionRule::Alternate(alt_time) => {
                        let std_offset_min = alt_time.std().ut_offset() * 60;
                        let dst_offset_min = alt_time.dst().ut_offset() * 60;
                        let dst_start_day = alt_time.dst_start();
                        let dst_end_day = alt_time.dst_end();

                        // Transition for standard to dst timezone
                        let mut std_to_dst = IcalTimeZoneTransition::new();
                        std_to_dst.add_property(Property {
                            name: "TZNAME".to_string(),
                            params: None,
                            value: Some(alt_time.std().time_zone_designation().to_string()),
                        });
                        std_to_dst.add_property(Property {
                            name: "TZOFFSETFROM".to_string(),
                            params: None,
                            value: Some(format!("{:+05}", std_offset_min)),
                        });
                        std_to_dst.add_property(Property {
                            name: "TZOFFSETTO".to_string(),
                            params: None,
                            value: Some(format!("{:+05}", dst_offset_min)),
                        });
                        // FIXME: this does not conform to RFC5545 and should be fixed
                        // once we know how to correctly get DST start(/end)
                        //
                        // HERE BE DRAGONS!!!!
                        let dtstart = match dst_start_day {
                            RuleDay::MonthWeekDay(mwd) => NaiveDate::from_weekday_of_month(
                                1970,
                                mwd.month().into(),
                                Weekday::from_u8(mwd.week_day()).unwrap(),
                                mwd.week(),
                            ),
                            RuleDay::Julian0WithLeap(days) => {
                                NaiveDate::from_yo(1970, (days.get() + 1) as u32)
                            }
                            RuleDay::Julian1WithoutLeap(days) => {
                                NaiveDate::from_yo(1970, days.get() as u32)
                            }
                        }
                        .and_hms(2, 0, 0);

                        let num_days_of_month = days_of_month(
                            &Month::from_u32(dtstart.month()).unwrap(),
                            dtstart.year(),
                        );
                        let day_occurrences_before = dtstart.day() % 7;
                        let day_occurrences_after = (num_days_of_month - dtstart.day()) % 7;

                        let offset: i32 = if day_occurrences_after == 0 {
                            -1
                        } else {
                            day_occurrences_before as i32 + 1
                        };

                        std_to_dst.add_property(Property {
                            name: "DTSTART".to_string(),
                            params: None,
                            value: Some(dtstart.format(ISO8601_2004_LOCAL_FORMAT).to_string()),
                        });

                        // We generate this RRULE by hand for now
                        std_to_dst.add_property(Property {
                            name: "RRULE".to_string(),
                            params: None,
                            value: Some(format!(
                                "FREQ=YEARLY;BYMONTH={};BYDAY={:+1}{}",
                                dtstart.month(),
                                offset,
                                weekday_to_ical(dtstart.weekday())
                            )),
                        });

                        tz_spec.transitions.push(std_to_dst);

                        // Transition for dst timezone back to standard
                        let mut dst_to_std = IcalTimeZoneTransition::new();
                        dst_to_std.add_property(Property {
                            name: "TZNAME".to_string(),
                            params: None,
                            value: Some(alt_time.std().time_zone_designation().to_string()),
                        });
                        dst_to_std.add_property(Property {
                            name: "TZOFFSETFROM".to_string(),
                            params: None,
                            value: Some(format!("{:+05}", dst_offset_min)),
                        });
                        dst_to_std.add_property(Property {
                            name: "TZOFFSETTO".to_string(),
                            params: None,
                            value: Some(format!("{:+05}", std_offset_min)),
                        });

                        // FIXME: this does not conform to RFC5545 and should be fixed
                        // once we know how to correctly get DST start(/end)
                        //
                        // HERE BE DRAGONS!!!!
                        let dtstart = match dst_end_day {
                            RuleDay::MonthWeekDay(mwd) => NaiveDate::from_weekday_of_month(
                                1970,
                                mwd.month().into(),
                                Weekday::from_u8(mwd.week_day()).unwrap(),
                                mwd.week(),
                            ),
                            RuleDay::Julian0WithLeap(days) => {
                                NaiveDate::from_yo(1970, (days.get() + 1) as u32)
                            }
                            RuleDay::Julian1WithoutLeap(days) => {
                                NaiveDate::from_yo(1970, days.get() as u32)
                            }
                        }
                        .and_hms(3, 0, 0);

                        let num_days_of_month = days_of_month(
                            &Month::from_u32(dtstart.month()).unwrap(),
                            dtstart.year(),
                        );
                        let day_occurrences_before = dtstart.day() % 7;
                        let day_occurrences_after = (num_days_of_month - dtstart.day()) % 7;

                        let offset: i32 = if day_occurrences_after == 0 {
                            -1
                        } else {
                            day_occurrences_before as i32 + 1
                        };

                        dst_to_std.add_property(Property {
                            name: "DTSTART".to_string(),
                            params: None,
                            value: Some(dtstart.format(ISO8601_2004_LOCAL_FORMAT).to_string()),
                        });

                        // We generate this RRULE by hand for now
                        dst_to_std.add_property(Property {
                            name: "RRULE".to_string(),
                            params: None,
                            value: Some(format!(
                                "FREQ=YEARLY;BYMONTH={};BYDAY={:+1}{}",
                                dtstart.month(),
                                offset,
                                weekday_to_ical(dtstart.weekday())
                            )),
                        });

                        tz_spec.transitions.push(dst_to_std);
                    }
                    _ => (),
                }
            }

            ical_calendar.timezones.push(tz_spec);
        }

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

        let tz = occurrence.timezone();

        Ok(Event {
            path: if path.is_file() {
                path.to_owned()
            } else {
                path.join(&uid.to_string()).with_extension(ICAL_FILE_EXT)
            },
            occurrence,
            ical: ical_calendar,
            tz,
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

    pub fn from_ical(path: &Path, ical: IcalCalendar) -> Result<Self> {
        if ical.events.len() > 1 {
            return Err(Error::from(ErrorKind::CalendarParse).with_msg(&format!(
                "Calendar '{}' has more than one event entry",
                path.display()
            )));
        }

        if ical.events.is_empty() {
            return Err(Error::from(ErrorKind::CalendarParse)
                .with_msg(&format!("Calendar '{}' has no event entry", path.display())));
        }

        let event = ical.events.first().unwrap();

        let dtstart = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(ErrorKind::EventMissingKey, "No DTSTART found"))?;

        let dtend = event.properties.iter().find(|p| p.name == "DTEND");
        // Check if DURATION is set
        let duration = event.properties.iter().find(|p| p.name == "DURATION");

        // Required (if METHOD not set)
        let dtstart_spec = IcalDateTime::try_from(dtstart)?;

        // Set TZ id based on start spec
        let tz = if let IcalDateTime::Local(dt) = dtstart_spec {
            dt.timezone()
        } else {
            chrono_tz::UTC
        };

        // DTEND does not HAVE to be specified...
        let mut occurrence = if let Some(dt) = dtend {
            // ...but if set it must be parseable
            let dtend_spec = IcalDateTime::try_from(dt)?;
            match &dtend_spec {
                IcalDateTime::Date(date) => {
                    if let IcalDateTime::Date(bdate) = dtstart_spec {
                        Occurrence::Onetime(TimeSpan::allday_until(
                            tz.from_utc_date(&bdate),
                            tz.from_utc_date(&date),
                        ))
                    } else {
                        return Err(Error::new(
                            ErrorKind::DateParse,
                            "DTEND must also be of type 'DATE' if DTSTART is",
                        ));
                    }
                }
                dt @ _ => Occurrence::Onetime(TimeSpan::from_start_and_end(
                    dtstart_spec.as_datetime(&tz),
                    dt.as_datetime(&tz),
                )),
            }
        } else if let Some(duration) = duration {
            let dur_spec = IcalDuration::try_from(duration)?;
            Occurrence::Onetime(TimeSpan::from_start_and_duration(
                dtstart_spec.as_datetime(&tz),
                dur_spec.into(),
            ))
        } else {
            // If neither DTEND, nor DURATION is specified event duration depends solely
            // on DTSTART. RFC 5545 states, that if DTSTART is...
            //  ... a date spec, the event has to have the duration of a single day
            //  ... a datetime spec, the event has to have the dtstart also as dtend
            match dtstart_spec {
                date @ IcalDateTime::Date(_) => {
                    Occurrence::Onetime(TimeSpan::allday(date.as_date(&tz)))
                }
                dt => Occurrence::Onetime(TimeSpan::from_start(dt.as_datetime(&tz))),
            }
        };

        let ical_rrule = event.properties.iter().find(|p| p.name == "RRULE");

        if let Some(rule) = ical_rrule {
            if let Ok(ruleset) = rule
                .value
                .as_ref()
                .unwrap()
                .parse::<RRule<rrule::Unvalidated>>()
            {
                let start = occurrence.begin();
                let tz = occurrence.timezone();
                occurrence =
                    occurrence.recurring(ruleset.build(start.with_timezone(&rrule::Tz::Tz(tz)))?);
            }
        }

        // TODO: VTIMEZONE
        // TODO: Check for exdate

        Ok(Event {
            path: path.into(),
            occurrence,
            ical,
            tz,
        })
    }

    fn get_property_value(&self, name: &str) -> Option<&str> {
        if let Some(prop) = self.ical.events[0]
            .properties
            .iter()
            .find(|prop| prop.name == name)
        {
            prop.value.as_deref()
        } else {
            None
        }
    }

    fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.ical.events[0]
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical.events[0]
    }
}

impl Eventlike for Event {
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

    fn description(&self) -> Option<&str> {
        self.get_property_value("DESCRIPTION")
    }

    fn set_summary(&mut self, summary: &str) {
        self.set_title(summary);
    }

    fn occurrence(&self) -> &Occurrence<Tz> {
        &self.occurrence
    }

    fn set_occurrence(&mut self, _occurrence: Occurrence<Tz>) {
        // TODO: implement
        unimplemented!()
    }

    fn tz(&self) -> &Tz {
        &self.tz
    }

    fn set_tz(&mut self, tz: &Tz) {
        self.tz = *tz;
        self.occurrence = self.occurrence.clone().with_tz(tz);
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
        event.ical.events[0].clone()
    }
}

impl From<Event> for IcalCalendar {
    fn from(event: Event) -> Self {
        event.ical
    }
}

pub struct Calendar {
    path: PathBuf,
    _identifier: String,
    friendly_name: String,
    tz: Tz,
    events: BTreeMap<DateTime<Tz>, Vec<Rc<Event>>>,
}

impl Calendar {
    pub fn _new(path: &Path) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();
        let friendly_name = identifier.clone();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: friendly_name.to_string(),
            tz: Tz::UTC,
            events: BTreeMap::new(),
        }
    }

    pub fn _new_with_name(path: &Path, name: String) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: name,
            tz: Tz::UTC,
            events: BTreeMap::new(),
        }
    }

    pub fn from_dir(path: &Path) -> Result<Self> {
        let mut events = BTreeMap::<DateTime<Tz>, Vec<Rc<Event>>>::new();

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
                    |file: fs::DirEntry| -> Result<Event> {
                        Event::from_file(file.path().as_path())
                    },
                )
            })
            .inspect(|res| {
                if let Err(err) = res {
                    log::warn!("{}", err)
                }
            })
            .filter_map(Result::ok);

        // TODO: use `BTreeMap::first_entry` once it's stable: https://github.com/rust-lang/rust/issues/62924
        let tz = if let Some((_, event)) = events.iter().next() {
            *(event.first().unwrap().tz())
        } else {
            Tz::UTC
        };

        let now = tz.from_utc_datetime(&Utc::now().naive_utc());

        for event in event_file_iter {
            let event_rc = Rc::new(event);

            event_rc
                .occurrence()
                .iter()
                .skip_while(|dt| dt < &(now - Duration::days(356)))
                .take_while(|dt| dt <= &(now + Duration::days(356)))
                .for_each(|dt| events.entry(dt).or_default().push(Rc::clone(&event_rc)));
        }

        Ok(Calendar {
            path: path.to_owned(),
            _identifier: path.file_stem().unwrap().to_string_lossy().to_string(),
            friendly_name: String::default(),
            tz,
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

impl Calendarlike for Calendar {
    fn name(&self) -> &str {
        &self.friendly_name
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tz(&self) -> &Tz {
        &self.tz
    }

    fn set_tz(&mut self, _tz: &Tz) {
        unimplemented!();
    }

    fn event_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &(dyn Eventlike + 'a)> + 'a> {
        Box::new(
            self.events
                .iter()
                .flat_map(|(_, v)| v.iter())
                .map(|ev| (ev.as_ref() as &dyn Eventlike)),
        )
    }

    fn filter_events<'a>(
        &'a self,
        filter: EventFilter,
    ) -> Box<dyn Iterator<Item = (&DateTime<Tz>, &(dyn Eventlike + 'a))> + 'a> {
        // TODO: Change once https://github.com/rust-lang/rust/issues/86026 is stable
        let real_begin = match filter.begin {
            Bound::Included(dt) => {
                Bound::Included(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            Bound::Excluded(dt) => {
                Bound::Excluded(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            _ => Bound::Unbounded,
        };
        let real_end = match filter.end {
            Bound::Included(dt) => {
                Bound::Included(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            Bound::Excluded(dt) => {
                Bound::Excluded(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            _ => Bound::Unbounded,
        };

        Box::new(
            self.events
                .range((real_begin, real_end))
                .flat_map(|(e, v)| v.iter().map(move |ev| (e, ev.as_ref() as &dyn Eventlike))),
        )
    }
}

impl EventExt for Calendar {
    fn add_event(&mut self, new_event: NewEvent<Tz>) -> Result<()> {
        let mut occurrence = if let Some(end) = new_event.end {
            Occurrence::Onetime(TimeSpan::from_start_and_end(new_event.begin, end))
        } else if let Some(duration) = new_event.duration {
            Occurrence::Onetime(TimeSpan::from_start_and_duration(new_event.begin, duration))
        } else {
            Occurrence::Onetime(TimeSpan::from_start(new_event.begin))
        };

        if let Some(rrule) = new_event.rrule {
            occurrence = occurrence.recurring(
                rrule.build(
                    new_event
                        .begin
                        .with_timezone(&rrule::Tz::Tz(new_event.begin.timezone())),
                )?,
            );
        }

        let mut event = Rc::new(Event::new(&self.path, occurrence)?);

        if let Some(title) = new_event.title {
            Rc::get_mut(&mut event).unwrap().set_title(title.as_ref());
        }

        if let Some(description) = new_event.description {
            Rc::get_mut(&mut event)
                .unwrap()
                .set_summary(description.as_ref());
        }

        // TODO: serde
        // let mut file = fs::File::create(event.path())?;
        // write!(&mut file, "{}", event.ical);
        log::info!("{:?}", event.ical);

        let now = self.tz.from_utc_datetime(&Utc::now().naive_utc());

        event
            .occurrence()
            .iter()
            .skip_while(|dt| dt < &(now - Duration::days(356)))
            .take_while(|dt| dt <= &(now + Duration::days(356)))
            .for_each(|dt| self.events.entry(dt).or_default().push(Rc::clone(&event)));

        Ok(())
    }
}

impl ExtCalendar for Calendar {}

pub type Calendar = provider::Calendar<Event>;

pub fn from_dir(path: &Path, config: &CalendarConfig) -> Result<Calendar> {
    let mut events = BTreeMap::<DateTime<Tz>, Vec<Rc<Event>>>::new();

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
                |file: fs::DirEntry| -> Result<Event> { Event::from_file(file.path().as_path()) },
            )
        })
        .inspect(|res| {
            if let Err(err) = res {
                log::warn!("{}", err)
            }
        })
        .filter_map(Result::ok);

    // TODO: use `BTreeMap::first_entry` once it's stable: https://github.com/rust-lang/rust/issues/62924
    let tz = if let Some((_, event)) = events.iter().next() {
        *(event.first().unwrap().tz())
    } else {
        Tz::UTC
    };

    let now = tz.from_utc_datetime(&Utc::now().naive_utc());

    for event in event_file_iter {
        let event_rc = Rc::new(event);

        event_rc
            .occurrence()
            .iter()
            .skip_while(|dt| dt < &(now - Duration::days(356)))
            .take_while(|dt| dt <= &(now + Duration::days(356)))
            .for_each(|dt| events.entry(dt).or_default().push(Rc::clone(&event_rc)));
    }

    Ok(Calendar {
        path: path.to_owned(),
        _identifier: config.id.clone(),
        friendly_name: config.name.clone(),
        tz,
        events,
    })
}

impl CalendarMut for Calendar {
    fn add_event(&mut self, new_event: NewEvent<Tz>) -> Result<()> {
        let mut occurrence = if let Some(end) = new_event.end {
            Occurrence::Onetime(TimeSpan::from_start_and_end(new_event.begin, end))
        } else if let Some(duration) = new_event.duration {
            Occurrence::Onetime(TimeSpan::from_start_and_duration(new_event.begin, duration))
        } else {
            Occurrence::Onetime(TimeSpan::from_start(new_event.begin))
        };

        if let Some(rrule) = new_event.rrule {
            occurrence = occurrence.recurring(
                rrule.build(
                    new_event
                        .begin
                        .with_timezone(&rrule::Tz::Tz(new_event.begin.timezone())),
                )?,
            );
        }

        let mut event = Rc::new(Event::new(&self.path, occurrence)?);

        if let Some(title) = new_event.title {
            Rc::get_mut(&mut event).unwrap().set_title(title.as_ref());
        }

        if let Some(description) = new_event.description {
            Rc::get_mut(&mut event)
                .unwrap()
                .set_summary(description.as_ref());
        }

        // TODO: serde
        // let mut file = fs::File::create(event.path())?;
        // write!(&mut file, "{}", event.ical);
        log::info!("{:?}", event.as_ical());

        let now = self.tz.from_utc_datetime(&Utc::now().naive_utc());

        event
            .occurrence()
            .iter()
            .skip_while(|dt| dt < &(now - Duration::days(356)))
            .take_while(|dt| dt <= &(now + Duration::days(356)))
            .for_each(|dt| self.events.entry(dt).or_default().push(Rc::clone(&event)));

        Ok(())
    }

    fn events_mut<'a>(
        &'a mut self,
    ) -> provider::EventIter<'a, <Self as provider::Calendarlike>::Event> {
        unimplemented!()
    }
}
