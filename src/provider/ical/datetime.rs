use chrono::{
    Date, DateTime, Duration, Month, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;
use ical::property::Property;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, one_of},
    combinator::{all_consuming, map_res, opt},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::convert::TryFrom;
use std::str::FromStr;

use crate::provider::{Error, ErrorKind, Result};

use super::{ISO8601_2004_LOCAL_FORMAT, ISO8601_2004_LOCAL_FORMAT_DATE};

pub fn _days_of_month(month: &Month, year: i32) -> u64 {
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
    Local(DateTime<Tz>),
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
