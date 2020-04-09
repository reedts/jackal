use chrono::{
    DateTime,
    FixedOffset,
    NaiveDateTime,
    TimeZone,
    Utc
};
use chrono_tz::{Tz, UTC};
use crate::ical::Calendar;

use ical::property::Property;
use ical::parser::ical::component::IcalEvent;
use std::io;

#[derive(Clone)]
pub struct Event<Tz: TimeZone> {
    begin: DateTime<Tz>,
    end: DateTime<Tz>,
    ical_event: IcalEvent,
}

fn parse_prop_to_date_time(property: &Property) -> Option<DateTime<Utc>> {
    type DT = DateTime<FixedOffset>;
    let iso8601_2004_local_format = "%Y%M%DT%H%M%S";
    println!("Parsing: {:?}", property);
    if !property.name.contains("^DT") {
        return None
    }
    
    let value = match &property.value {
        Some(v) => v,
        None => return None
    };

    // Return if is already UTC
    if value.contains('Z') {
        return match DT::parse_from_rfc3339(&value) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(_) => None
        }
    }
    
    // Check if TZID is defined
    let params = &property.params;

    match params {
        Some(param) => {
            match param.iter().find(|(name, _)| name == "TZID") {
                Some((_, tz)) => {
                    let tz: Tz = tz.first().unwrap().parse().unwrap();
                    let tz_date = tz.datetime_from_str(&value, iso8601_2004_local_format).unwrap();
                    Some(tz_date.with_timezone(&Utc))
                },
                None => {
                    // Must be localtime
                    let naive_dt = NaiveDateTime::parse_from_str(&value, iso8601_2004_local_format).unwrap();
                    // TODO: Configure this for local timezone
                    let tz_offset = FixedOffset::west(0);
                    Some(Utc.from_utc_datetime(&(naive_dt - tz_offset)))
                }
            }
        },
        None => {
            // Must be localtime
            let naive_dt = NaiveDateTime::parse_from_str(&value, iso8601_2004_local_format).unwrap();
            // TODO: How to get this from local timezone
            let tz_offset = FixedOffset::west(0);
            Some(Utc.from_local_datetime(&(naive_dt - tz_offset)).unwrap())
        }
    }
}
impl Event<Utc> {

    pub fn from(ical_event: IcalEvent) -> io::Result<Self> {
        let dstart = match ical_event.properties.iter().find(|p| p.name == "DTSTART") {
            Some(begin) => match parse_prop_to_date_time(&begin) {
                Some(dt) => dt,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Could not find begin timepoint for event",
                    ))
                }
            },
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Could not find begin timepoint for event",
                ))
            }
        };

        let dend = match ical_event.properties.iter().find(|p| p.name == "DTEND") {
            Some(end) => match parse_prop_to_date_time(&end) {
                Some(dt) => dt,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Could not find begin timepoint for event",
                    ))
                }
            },

            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Could not find end timepoint for event",
                ))
            }
        };


        Ok(Event {
            begin: dstart,
            end: dend,
            ical_event,
        })
    }
}
