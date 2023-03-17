use chrono::{
    DateTime, Datelike, Duration, Month, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday,
};
use ical::parser::ical::component::{
    IcalTimeZone, IcalTimeZoneTransition, IcalTimeZoneTransitionType,
};
use ical::parser::Component;
use ical::property::Property;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{char, digit1, one_of},
    combinator::{all_consuming, map, map_res, opt},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use num_traits::FromPrimitive;
use rrule::{RRule, RRuleSet};
use std::convert::TryFrom;
use std::fmt::Display;
use std::iter::FromIterator;
use std::str::FromStr;
use tz;
use tz::timezone;

use crate::provider::days_of_month;

use super::tz::*;
use super::{Error, ErrorKind, Result, TimeSpan};
use super::{ISO8601_2004_LOCAL_FORMAT, ISO8601_2004_LOCAL_FORMAT_DATE, ISO8601_2004_UTC_FORMAT};

pub fn weekday_to_ical(weekday: Weekday) -> String {
    let mut s = format!("{}", weekday).to_uppercase();
    s.pop();
    s
}

pub fn generate_timestamp() -> String {
    let tstamp = Utc::now();
    format!("{}Z", tstamp.format(ISO8601_2004_LOCAL_FORMAT))
}

#[derive(Default, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct IcalDuration {
    sign: i8,
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl IcalDuration {
    pub fn weeks(sign: i8, weeks: i64) -> Self {
        IcalDuration {
            sign,
            weeks,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }

    pub fn datetime(sign: i8, days: i64, hours: i64, minutes: i64, seconds: i64) -> Self {
        IcalDuration {
            sign,
            weeks: 0,
            days,
            hours,
            minutes,
            seconds,
        }
    }

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
                weeks,
                days: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            },
        ))
    }

    fn parse_datetime_format(input: &str) -> IResult<&str, Self> {
        let (input, days) = opt(Self::value_with_designator("D"))(input)?;

        let (input, time) = opt(preceded(
            char('T'),
            tuple((
                opt(Self::value_with_designator("H")),
                opt(Self::value_with_designator("M")),
                opt(Self::value_with_designator("S")),
            )),
        ))(input)?;

        let (hours, minutes, seconds) = time.unwrap_or_default();

        if days.is_none() && hours.is_none() && minutes.is_none() && seconds.is_none() {
            Err(nom::Err::Error(nom::error::ParseError::from_error_kind(
                input,
                nom::error::ErrorKind::Verify,
            )))
        } else {
            Ok((
                input,
                Self {
                    sign: 1,
                    weeks: 0,
                    days: days.unwrap_or_default(),
                    hours: hours.unwrap_or_default(),
                    minutes: minutes.unwrap_or_default(),
                    seconds: seconds.unwrap_or_default(),
                },
            ))
        }
    }

    fn to_duration(&self) -> chrono::Duration {
        chrono::Duration::seconds(
            self.sign as i64
                * ((self.weeks * 7 * 24 * 60 * 60)
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

impl Display for IcalDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::with_capacity(10);
        output += if self.sign >= 0 { "+" } else { "-" };
        output += "P";
        if self.weeks != 0 {
            output += &format!("{}W", self.weeks.abs());
        } else {
            output += "T";
            if self.days != 0 {
                output += &format!("{}D", self.days.abs())
            }
            if self.hours != 0 {
                output += &format!("{}H", self.hours.abs())
            }
            if self.minutes != 0 {
                output += &format!("{}M", self.minutes.abs())
            }
            if self.seconds != 0 {
                output += &format!("{}S", self.seconds.abs())
            }
        }

        f.write_str(&output)
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

impl From<chrono::Duration> for IcalDuration {
    fn from(dur: chrono::Duration) -> Self {
        // Check if duration only consists of weeks
        if dur.is_zero() {
            IcalDuration::default()
        } else if dur - chrono::Duration::weeks(dur.num_weeks()) == chrono::Duration::zero() {
            IcalDuration::weeks(dur.num_milliseconds().signum() as i8, dur.num_weeks())
        } else {
            let days = dur.num_days();
            let mut rest = dur - Duration::days(days);
            let hours = rest.num_hours();
            rest = rest - Duration::hours(hours);
            let minutes = rest.num_minutes();
            rest = rest - Duration::minutes(minutes);
            let seconds = rest.num_seconds();
            IcalDuration::datetime(
                dur.num_seconds().signum() as i8,
                days,
                hours,
                minutes,
                seconds,
            )
        }
    }
}

impl From<IcalDuration> for Duration {
    fn from(dur: IcalDuration) -> Self {
        dur.to_duration()
    }
}

impl From<&Tz> for IcalTimeZone {
    fn from(value: &Tz) -> Self {
        fn create_timezone_transitions(
            transition: IcalTimeZoneTransitionType,
            tz_name: String,
            from_offset: i32,
            to_offset: i32,
            transition_day: &tz::timezone::RuleDay,
        ) -> IcalTimeZoneTransition {
            let mut tr = IcalTimeZoneTransition::new(transition);
            tr.add_property(Property {
                name: "TZNAME".to_string(),
                params: None,
                value: Some(tz_name),
            });
            tr.add_property(Property {
                name: "TZOFFSETFROM".to_string(),
                params: None,
                value: Some(format!("{:+05}", from_offset)),
            });
            tr.add_property(Property {
                name: "TZOFFSETTO".to_string(),
                params: None,
                value: Some(format!("{:+05}", to_offset)),
            });
            // FIXME: this does not conform to RFC5545 and should be fixed
            // once we know how to correctly get DST start(/end)
            //
            // HERE BE DRAGONS!!!!
            let dtstart = match transition_day {
                tz::timezone::RuleDay::MonthWeekDay(mwd) => NaiveDate::from_weekday_of_month_opt(
                    1970,
                    mwd.month().into(),
                    Weekday::from_u8(mwd.week_day()).unwrap(),
                    mwd.week(),
                )
                .unwrap(),
                tz::timezone::RuleDay::Julian0WithLeap(days) => {
                    NaiveDate::from_yo_opt(1970, (days.get() + 1) as u32).unwrap()
                }
                tz::timezone::RuleDay::Julian1WithoutLeap(days) => {
                    NaiveDate::from_yo_opt(1970, days.get() as u32).unwrap()
                }
            }
            .and_hms_opt(2, 0, 0)
            .unwrap();

            let num_days_of_month =
                days_of_month(&Month::from_u32(dtstart.month()).unwrap(), dtstart.year());
            let day_occurrences_before = dtstart.day() % 7;
            let day_occurrences_after = (num_days_of_month - dtstart.day()) % 7;

            let offset: i32 = if day_occurrences_after == 0 {
                -1
            } else {
                day_occurrences_before as i32 + 1
            };

            tr.add_property(Property {
                name: "DTSTART".to_string(),
                params: None,
                value: Some(dtstart.format(ISO8601_2004_LOCAL_FORMAT).to_string()),
            });

            // We generate this RRULE by hand for now
            tr.add_property(Property {
                name: "RRULE".to_string(),
                params: None,
                value: Some(format!(
                    "FREQ=YEARLY;BYMONTH={};BYDAY={:+1}{}",
                    dtstart.month(),
                    offset,
                    weekday_to_ical(dtstart.weekday())
                )),
            });

            tr
        }

        let mut tz_spec = IcalTimeZone::new();
        tz_spec.add_property(Property {
            name: "TZID".to_owned(),
            params: None,
            value: Some(value.id().to_owned()),
        });

        match value {
            Tz::Local | Tz::Iana(chrono_tz::UTC) => IcalTimeZone::default(),
            Tz::Iana(tz) => {
                let tz_info =
                    tz::TimeZone::from_posix_tz(tz.name()).expect("IANA specifier must exist");

                if let Some(rule) = tz_info.as_ref().extra_rule() {
                    match rule {
                        tz::timezone::TransitionRule::Alternate(alt_time) => {
                            let std_offset_min = alt_time.std().ut_offset() * 60;
                            let dst_offset_min = alt_time.dst().ut_offset() * 60;
                            let dst_start_day = alt_time.dst_start();
                            let dst_end_day = alt_time.dst_end();

                            // Transition for standard to dst timezone
                            let std_to_dst = create_timezone_transitions(
                                IcalTimeZoneTransitionType::STANDARD,
                                alt_time.std().time_zone_designation().to_string(),
                                std_offset_min,
                                dst_offset_min,
                                dst_start_day,
                            );
                            tz_spec.transitions.push(std_to_dst);

                            // Transition for dst timezone back to standard
                            let dst_to_std = create_timezone_transitions(
                                IcalTimeZoneTransitionType::DAYLIGHT,
                                alt_time.dst().time_zone_designation().to_string(),
                                dst_offset_min,
                                std_offset_min,
                                dst_end_day,
                            );
                            tz_spec.transitions.push(dst_to_std);
                        }
                        _ => (),
                    };
                }
                tz_spec
            }
            Tz::Custom { id: _, transitions } => {
                for transition in transitions.transitions.iter() {
                    let transition_rule = if transition.dst_offset_secs > 0 {
                        IcalTimeZoneTransitionType::STANDARD
                    } else {
                        IcalTimeZoneTransitionType::DAYLIGHT
                    };

                    let mut tr = IcalTimeZoneTransition::new(transition_rule);

                    if let Some(name) = transition.name.as_deref() {
                        tr.add_property(Property {
                            name: "TZNAME".to_string(),
                            params: None,
                            value: Some(name.to_string()),
                        });
                    }

                    match tr.transition {
                        IcalTimeZoneTransitionType::STANDARD => {
                            tr.add_property(Property {
                                name: "TZOFFSETFROM".to_string(),
                                params: None,
                                value: Some(format!("{:+05}", transition.dst_offset_secs / 3600)),
                            });
                            tr.add_property(Property {
                                name: "TZOFFSETTO".to_string(),
                                params: None,
                                value: Some(format!("{:+05}", transition.utc_offset_secs / 3600)),
                            });
                        }
                        IcalTimeZoneTransitionType::DAYLIGHT => {
                            tr.add_property(Property {
                                name: "TZOFFSETFROM".to_string(),
                                params: None,
                                value: Some(format!("{:+05}", transition.utc_offset_secs / 3600)),
                            });
                            tr.add_property(Property {
                                name: "TZOFFSETTO".to_string(),
                                params: None,
                                value: Some(format!("{:+05}", transition.dst_offset_secs / 3600)),
                            });
                        }
                    }

                    tz_spec.transitions.push(tr);
                }
                tz_spec
            }
        }
    }
}

impl TryFrom<&IcalTimeZone> for Tz {
    type Error = Error;
    fn try_from(value: &IcalTimeZone) -> std::result::Result<Self, Self::Error> {
        fn parse_offset(s: &str) -> Result<i32> {
            let (_, (sign, hours, minutes, seconds)) = tuple((
                map_res(
                    one_of::<_, _, (&str, nom::error::ErrorKind)>("+-"),
                    |s: char| (s.to_string() + "1").parse::<i32>(),
                ),
                map_res(take(2usize), |s: &str| s.parse::<i32>()),
                map_res(take(2usize), |s: &str| s.parse::<i32>()),
                map_res(opt(take(2usize)), |s: Option<&str>| {
                    s.unwrap_or("0").parse::<i32>()
                }),
            ))(s)?;

            Ok(sign * (hours * 3600 + minutes * 60 + seconds))
        }

        let id = value
            .get_property("TZID")
            .ok_or(Error::new(ErrorKind::TimezoneError, "Timezone has no id"))?
            .value
            .as_deref()
            .unwrap();

        // First try for "well-known" (IANA) TZID
        if let tz @ Ok(_) = id.parse::<Tz>() {
            return tz;
        }

        // If not well-known we build a Tz from custom transitions
        let mut transitions = Vec::<Transition>::with_capacity(value.transitions.len());

        for ical_transition in value.transitions.iter() {
            let (utc_offset_secs, dst_offset_secs) = match ical_transition.transition {
                IcalTimeZoneTransitionType::STANDARD => (
                    parse_offset(
                        ical_transition
                            .get_property("TZOFFSETTO")
                            .unwrap()
                            .value
                            .as_ref()
                            .unwrap(),
                    )
                    .expect("TZOFFSETTO not convertible"),
                    0,
                ),
                IcalTimeZoneTransitionType::DAYLIGHT => {
                    let utc_offset = parse_offset(
                        ical_transition
                            .get_property("TZOFFSETFROM")
                            .unwrap()
                            .value
                            .as_ref()
                            .unwrap(),
                    )
                    .expect("TZOFFSETFROM not convertible");
                    (
                        utc_offset,
                        parse_offset(
                            ical_transition
                                .get_property("TZOFFSETTO")
                                .unwrap()
                                .value
                                .as_ref()
                                .unwrap(),
                        )
                        .expect("TZOFFSETTO not convertible")
                            - utc_offset,
                    )
                }
            };

            let name = ical_transition
                .get_property("TZNAME")
                .and_then(|prop| prop.value.to_owned());

            // build RRULE for custom timezone
            // There is no need to provide a Tz here as DTSTART in VTIMEZONE
            // must contain a local (or 'floating') value
            let dtstart = IcalDateTime::from_property(
                ical_transition.get_property("DTSTART").ok_or(Error::new(
                    ErrorKind::TimezoneError,
                    &format!(
                        "Missing DTSTART for timezone '{}'",
                        name.as_deref().unwrap_or(&id)
                    ),
                ))?,
                None,
            )?;

            let rule = if let Some(rrule_str) = ical_transition
                .get_property("RRULE")
                .and_then(|prop| prop.value.as_deref())
            {
                let rrule = rrule_str
                    .parse::<RRule<rrule::Unvalidated>>()
                    .expect("Could not parse RRULE of timezone");
                TransitionRule::Recurring(rrule.build(dtstart.as_datetime(&rrule::Tz::LOCAL))?)
            } else {
                TransitionRule::Single(dtstart.as_naive_local())
            };

            transitions.push(Transition {
                utc_offset_secs,
                dst_offset_secs,
                id: id.to_string(),
                name,
                rule,
            });
        }

        Ok(Self::from_iter(transitions))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcalDateTime {
    Date(NaiveDate),
    Floating(NaiveDateTime),
    Utc(DateTime<Utc>),
    Local(DateTime<Tz>),
}

impl IcalDateTime {
    pub fn from_property(property: &Property, tz: Option<&Tz>) -> Result<Self> {
        let val = property
            .value
            .as_ref()
            .ok_or(Error::from(ErrorKind::DateParse).with_msg("Missing datetime value"))?;

        let has_options = property.params.is_some();
        let mut used_tz: Option<Tz> = None;

        if has_options {
            // check if value is date
            if property
                .params
                .as_ref()
                .unwrap()
                .iter()
                .find(|(name, values)| name == "VALUE" && values[0] == "DATE")
                .is_some()
            {
                return Ok(Self::Date(NaiveDate::parse_from_str(
                    val,
                    ISO8601_2004_LOCAL_FORMAT_DATE,
                )?));
            }

            // check for TZID in options
            if let Some((_, values)) = &property
                .params
                .as_ref()
                .unwrap()
                .iter()
                .find(|(name, _)| name == "TZID")
            {
                if tz.is_some() && tz.unwrap().id() == values[0] {
                    used_tz = Some(tz.unwrap().clone())
                } else {
                    used_tz = Some(values[0].parse::<Tz>()?)
                }
            };
        }

        if let Ok(dt) = NaiveDateTime::parse_from_str(val, ISO8601_2004_LOCAL_FORMAT) {
            if let Some(tz) = used_tz {
                Ok(Self::Local(tz.from_local_datetime(&dt).earliest().unwrap()))
            } else {
                Ok(Self::Floating(dt))
            }
        } else if let Ok(dt) = NaiveDateTime::parse_from_str(val, ISO8601_2004_UTC_FORMAT) {
            Ok(Self::Utc(Utc.from_utc_datetime(&dt)))
        } else {
            let date = NaiveDate::parse_from_str(val, ISO8601_2004_LOCAL_FORMAT_DATE)?;
            Ok(Self::Date(date))
        }
    }

    pub fn timezone(&self) -> Tz {
        match self {
            IcalDateTime::Date(_) | IcalDateTime::Floating(_) => Tz::Local,
            IcalDateTime::Utc(_) => Tz::utc(),
            IcalDateTime::Local(dt) => dt.timezone(),
        }
    }

    pub fn as_datetime<Tz: TimeZone>(&self, tz: &Tz) -> chrono::DateTime<Tz> {
        match self {
            IcalDateTime::Date(dt) => tz.from_utc_datetime(&dt.and_hms_opt(0, 0, 0).unwrap()),
            IcalDateTime::Floating(dt) => tz.from_utc_datetime(&dt),
            IcalDateTime::Utc(dt) => dt.with_timezone(&tz),
            IcalDateTime::Local(dt) => dt.with_timezone(&tz),
        }
    }

    pub fn as_naive_local(&self) -> NaiveDateTime {
        match self {
            IcalDateTime::Date(dt) => dt.and_hms_opt(0, 0, 0).unwrap(),
            IcalDateTime::Floating(dt) => dt.clone(),
            IcalDateTime::Utc(dt) => dt.naive_local(),
            IcalDateTime::Local(dt) => dt.naive_local(),
        }
    }

    pub fn as_date<Tz: TimeZone>(&self) -> NaiveDate {
        match self {
            IcalDateTime::Date(dt) => dt.clone(),
            IcalDateTime::Floating(dt) => dt.date(),
            IcalDateTime::Utc(dt) => dt.date_naive(),
            IcalDateTime::Local(dt) => dt.date_naive(),
        }
    }

    pub fn to_property(&self, name: String) -> Property {
        Property {
            name,
            params: match &self {
                IcalDateTime::Local(dt) => {
                    Some(vec![("TZID".to_owned(), vec![dt.offset().id.clone()])])
                }
                IcalDateTime::Date(_) => Some(vec![("VALUE".to_owned(), vec!["DATE".to_owned()])]),
                _ => None,
            },
            value: Some(self.to_string()),
        }
    }
}

impl Display for IcalDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IcalDateTime::Utc(dt) => write!(
                f,
                "{}",
                dt.format(&format!("{}Z", ISO8601_2004_LOCAL_FORMAT))
            ),
            IcalDateTime::Date(date) => {
                write!(f, "{}", date.format(ISO8601_2004_LOCAL_FORMAT_DATE))
            }
            IcalDateTime::Floating(dt) => write!(f, "{}", dt.format(ISO8601_2004_LOCAL_FORMAT)),
            IcalDateTime::Local(dt) => write!(f, "{}", dt.format(ISO8601_2004_LOCAL_FORMAT)),
        }
    }
}

impl From<NaiveDate> for IcalDateTime {
    fn from(date: NaiveDate) -> Self {
        IcalDateTime::Date(date)
    }
}

impl From<NaiveDateTime> for IcalDateTime {
    fn from(dt: NaiveDateTime) -> Self {
        IcalDateTime::Floating(dt)
    }
}

impl From<DateTime<Utc>> for IcalDateTime {
    fn from(dt: DateTime<Utc>) -> Self {
        IcalDateTime::Utc(dt)
    }
}

impl From<DateTime<Tz>> for IcalDateTime {
    fn from(dt: DateTime<Tz>) -> Self {
        if let Tz::Iana(chrono_tz::Tz::UTC) = &dt.timezone() {
            Self::from(dt.with_timezone(&Utc {}))
        } else {
            IcalDateTime::Local(dt)
        }
    }
}

impl FromStr for IcalDateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, ISO8601_2004_UTC_FORMAT) {
            Ok(IcalDateTime::Utc(Utc {}.from_utc_datetime(&dt)))
        } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT) {
            // At this point we can only interpret this format as Floating time as we don't know
            // whether the ical property also holds the TZID parameter
            Ok(IcalDateTime::Floating(dt))
        } else if let Ok(dt) = NaiveDate::parse_from_str(s, ISO8601_2004_LOCAL_FORMAT_DATE) {
            Ok(IcalDateTime::Date(dt))
        } else {
            Err(Error::new(
                ErrorKind::TimeParse,
                &format!("Could not extract datetime from '{}'", s),
            ))
        }
    }
}

impl Default for IcalDateTime {
    fn default() -> Self {
        IcalDateTime::Floating(NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
    }
}
pub struct IcalTimeSpan(pub TimeSpan<Tz>);

impl From<IcalTimeSpan> for Vec<Property> {
    fn from(ts: IcalTimeSpan) -> Self {
        let mut ret = Vec::<Property>::with_capacity(2);
        match ts.0 {
            TimeSpan::Allday(start_date, end_date, _) => {
                ret.push(IcalDateTime::from(start_date).to_property("DTSTART".to_owned()));
                if let Some(date) = end_date {
                    ret.push(IcalDateTime::from(date).to_property("DTEND".to_owned()));
                }
            }
            TimeSpan::Instant(dt) => {
                ret.push(IcalDateTime::from(dt).to_property("DTSTART".to_owned()))
            }
            TimeSpan::Duration(dt, dur) => {
                ret.push(IcalDateTime::from(dt).to_property("DTSTART".to_owned()));
                ret.push(Property {
                    name: "DURATION".to_owned(),
                    params: None,
                    value: Some(IcalDuration::from(dur).to_string()),
                });
            }
            TimeSpan::TimePoints(start_dt, end_dt) => {
                ret.push(IcalDateTime::from(start_dt).to_property("DTSTART".to_owned()));
                ret.push(IcalDateTime::from(end_dt).to_property("DTEND".to_owned()));
            }
        }

        ret
    }
}
