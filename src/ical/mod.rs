mod calendar;
mod error;
mod event;

pub use calendar::Calendar;
pub use error::{Error, ErrorKind};
pub use event::Event;

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;

use chrono::{DateTime, FixedOffset, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;

use ::ical::property::Property;

pub(crate) fn prop_value_in_zulu(timestamp: &Property) -> bool {
    if let Some(v) = &timestamp.value {
        if v.contains("Z") {
            return true;
        }
    }

    false
}

pub(crate) fn parse_prop_to_date_time(property: &Property) -> IcalResult<DateTime<FixedOffset>> {
    const iso8601_2004_local_format: &str = "%Y%M%DT%H%M%S";

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

    let params = &property.params;
    println!("{:?}", params);

    // Unpack any parameters
    if let Some(params) = &property.params {
        for param in params {
            let (name, value) = &param;
            if name == "TZID" {
                if let Ok(tz) = value.first().unwrap().parse::<Tz>() {
                    if let Some(dt) = found_str_dt {
                        let dt =
                            NaiveDateTime::parse_from_str(&dt, iso8601_2004_local_format).unwrap();
                        let offset = Utc
                            .offset_from_local_datetime(&dt)
                            .earliest()
                            .unwrap()
                            .fix();
                        return Ok(offset.from_local_datetime(&dt).earliest().unwrap());
                    }
                }
            } else if name == "DATE" {
                return Err(Error::new(ErrorKind::CalendarMissingKey));
            }
        }
        return Err(Error::new(ErrorKind::CalendarMissingKey));
    } else {
        // No parameters found, check if value was found earlier
        if let Some(dt) = found_str_dt {
            // Must be localtime (or gibberish)
            if let Ok(naive_local) = NaiveDateTime::parse_from_str(&dt, iso8601_2004_local_format) {
                let offset = Utc
                    .offset_from_local_datetime(&naive_local)
                    .earliest()
                    .unwrap()
                    .fix();
                return Ok(offset.from_local_datetime(&naive_local).earliest().unwrap());
            } else {
                return Err(Error::new(ErrorKind::TimeParse)
                    .with_msg("Failed to parse localtime from iso8601 format"));
            }
        } else {
            return Err(
                Error::new(ErrorKind::EventMissingKey).with_msg("Could not find valid timestamp")
            );
        }
    }
}
