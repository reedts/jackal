use chrono::{Date, DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{Tz, UTC};
use std::error::Error;

use crate::ical;
use ::ical::parser::ical::component::IcalEvent;
use ::ical::property::Property;

pub type Result<T> = std::result::Result<T, crate::ical::Error>;

#[derive(Clone)]
pub struct Event<Tz: TimeZone> {
    begin: DateTime<Tz>,
    end: DateTime<Tz>,
    all_day: bool,
    ical_event: IcalEvent,
}

#[derive(Debug)]
pub struct EventParseError {
    message: String,
}

fn prop_value_in_zulu(timestamp: &Property) -> bool {
    if let Some(v) = &timestamp.value {
        if v.contains("Z") {
            return true;
        }
    }

    false
}

fn parse_prop_to_date_time(property: &Property) -> Result<DateTime<Tz>> {
    type DT = DateTime<Tz>;
    const iso8601_2004_local_format: &str = "%Y%M%DT%H%M%S";

    let found_str_dt: Option<&str> = None;

    // Check if property has value
    if let Some(dt) = &property.value {
        // Return if already UTC
        if prop_value_in_zulu(&property) {
            return match DateTime::parse_from_rfc3339(&property.value.unwrap()) {
                Ok(dt) => Ok(dt.with_timezone(&UTC)),
                Err(_) => Err(ical::Error::new(ical::ErrorKind::TimeParse)
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
                tz = value.first().unwrap().parse()?;
            } else if name == "DATE" {
            }
        }

        Ok(date.unwrap())
    } else {
        // No parameters found, check if value was found earlier
        if let Some(dt) = found_str_dt {
            // Must be localtime (or gibberish)
            if let Some(naive_dt) = NaiveDateTime::parse_from_str(&dt, iso8601_2004_local_format) {}
            // TODO: How to get this from local timezone
            let tz_offset = FixedOffset::west(0);
            Ok(Utc.from_local_datetime(&(naive_dt - tz_offset)).unwrap())
        }
    }
}

impl Event<Utc> {
    pub fn from(ical_event: IcalEvent) -> Result<Self> {
        let dstart = match ical_event.properties.iter().find(|p| p.name == "DTSTART") {
            Some(begin) => parse_prop_to_date_time(&begin)?,
            None => {
                return Err(
                    ical::Error::new(ical::ErrorKind::EventMissingKey).with_msg("No DTSTART found")
                )
            }
        };

        let dend = match ical_event.properties.iter().find(|p| p.name == "DTEND") {
            Some(end) => parse_prop_to_date_time(&end)?,
            None => {
                return Err(
                    ical::Error::new(ical::ErrorKind::EventMissingKey).with_msg("No DTEND found")
                )
            }
        };

        Ok(Event {
            begin: dstart,
            end: dend,
            all_day: false,
            ical_event,
        })
    }

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

    pub fn begin_date(&self) -> Date<Utc> {
        self.begin().date()
    }

    pub fn begin(&self) -> &DateTime<Utc> {
        &self.begin
    }

    pub fn end(&self) -> &DateTime<Utc> {
        &self.end
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical_event
    }
}
