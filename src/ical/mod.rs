mod calendar;
mod error;
mod event;

pub use calendar::Calendar;
pub use error::{Error, ErrorKind};
pub use event::Event;

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;

use ::ical::property::Property;

const ISO8601_2004_LOCAL_FORMAT: &str = "%Y%m%dT%H%M%S";
const ISO8601_2004_LOCAL_FORMAT_DATE: &str = "%Y%m%d";

pub(crate) fn prop_value_in_zulu(timestamp: &Property) -> bool {
    if let Some(v) = &timestamp.value {
        if v.contains('Z') {
            return true;
        }
    }

    false
}

// TODO: Make more ... sophisticated
pub(crate) fn parse_prop_to_date_time(property: &Property) -> IcalResult<DateTime<FixedOffset>> {
    let mut found_str_dt: Option<&str> = None;

    // Check if property has value
    if let Some(dt) = &property.value {
        // Return if already UTC
        if prop_value_in_zulu(&property) {
            return match DateTime::parse_from_rfc3339(property.value.as_ref().unwrap()) {
                Ok(dt) => Ok(dt),
                Err(_) => Err(Error::new(ErrorKind::TimeParse)
                    .with_msg("Parsing of timestamp to rfc3339 not possible")),
            };
        } else {
            // Further investigation of parameters necessary
            found_str_dt = Some(dt);
        }
    }

    // Unpack any parameters
    if let Some(params) = &property.params {
        for param in params {
            let (name, values) = &param;

            match name.as_str() {
                "TZID" => {
                    if let Ok(tz) = values.first().unwrap().parse::<Tz>() {
                        if let Some(dt) = found_str_dt {
                            let dt = NaiveDateTime::parse_from_str(&dt, ISO8601_2004_LOCAL_FORMAT)?;
                            let offset =
                                tz.offset_from_local_datetime(&dt).earliest().unwrap().fix();
                            return Ok(offset.from_local_datetime(&dt).earliest().unwrap());
                        }
                    } else {
                        return Err(
                            Error::new(ErrorKind::TimeParse).with_msg("Unable to parse timezone")
                        );
                    }
                }
                "VALUE" if values.first().unwrap() == "DATE" => {
                    // Date only
                    if let Some(date_str) = found_str_dt {
                        let date =
                            NaiveDate::parse_from_str(&date_str, ISO8601_2004_LOCAL_FORMAT_DATE)?;
                        let datetime = date.and_time(NaiveTime::from_hms(0, 0, 0));
                        let offset = Utc
                            .offset_from_local_datetime(&datetime)
                            .earliest()
                            .unwrap()
                            .fix();
                        return Ok(offset.from_local_datetime(&datetime).earliest().unwrap());
                    } else {
                        return Err(Error::new(ErrorKind::EventMissingKey)
                            .with_msg("Could not find valid timestamp"));
                    }
                }
                _ => continue,
            }
        }
    }

    // No parameters found, check if value was found earlier
    if let Some(dt) = found_str_dt {
        // Must be localtime (or gibberish)
        let naive_local = NaiveDateTime::parse_from_str(&dt, ISO8601_2004_LOCAL_FORMAT)?;
        let offset = Utc
            .offset_from_local_datetime(&naive_local)
            .earliest()
            .unwrap()
            .fix();
        Ok(offset.from_local_datetime(&naive_local).earliest().unwrap())
    } else {
        Err(Error::new(ErrorKind::EventMissingKey).with_msg("Could not find valid timestamp"))
    }
}
