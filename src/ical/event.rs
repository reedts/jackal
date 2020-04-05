use chrono::{DateTime, FixedOffset, TimeZone};

use ical::parser::ical::component::IcalEvent;
use std::io;

#[derive(Clone)]
pub struct Event<Tz: TimeZone> {
    begin: DateTime<Tz>,
    end: DateTime<Tz>,
    ical_event: IcalEvent,
}

impl Event<FixedOffset> {
    pub fn from(ical_event: IcalEvent) -> io::Result<Self> {
        let begin_str = match ical_event.properties.iter().find(|p| p.name == "DTSTART") {
            Some(begin) => begin.value.as_ref().unwrap(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Could not find begin timepoint for event",
                ))
            }
        };

        let end_str = match ical_event.properties.iter().find(|p| p.name == "DTEND") {
            Some(end) => end.value.as_ref().unwrap(),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Could not find end timepoint for event",
                ))
            }
        };

        let begin = DateTime::<FixedOffset>::parse_from_rfc3339(begin_str).unwrap();
        let end = DateTime::<FixedOffset>::parse_from_rfc3339(end_str).unwrap();

        Ok(Event {
            begin,
            end,
            ical_event,
        })
    }
}
