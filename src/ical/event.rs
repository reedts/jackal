use chrono::{Date, DateTime, FixedOffset, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use std::error::Error;

use ::ical::parser::ical::component::IcalEvent;

use crate::ical;
use crate::ical::IcalResult;

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

impl Event<FixedOffset> {
    pub fn from(ical_event: IcalEvent) -> IcalResult<Self> {
        let dstart = match ical_event.properties.iter().find(|p| p.name == "DTSTART") {
            Some(begin) => ical::parse_prop_to_date_time(&begin)?,
            None => {
                return Err(
                    ical::Error::new(ical::ErrorKind::EventMissingKey).with_msg("No DTSTART found")
                )
            }
        };

        let dend = match ical_event.properties.iter().find(|p| p.name == "DTEND") {
            Some(end) => ical::parse_prop_to_date_time(&end)?,
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

    pub fn begin_date(&self) -> Date<FixedOffset> {
        self.begin().date()
    }

    pub fn begin(&self) -> &DateTime<FixedOffset> {
        &self.begin
    }

    pub fn end(&self) -> &DateTime<FixedOffset> {
        &self.end
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical_event
    }
}
