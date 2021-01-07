use chrono::{Date, FixedOffset, TimeZone};

use ::ical::parser::ical::component::IcalEvent;

use crate::ical;
use crate::ical::{IcalResult, Occurence};

#[derive(Clone)]
pub struct Event<Tz: TimeZone> {
    begin: Occurence<Tz>,
    end: Occurence<Tz>,
    ical_event: IcalEvent,
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
        self.begin().inner_as_date()
    }

    pub fn begin(&self) -> &Occurence<FixedOffset> {
        &self.begin
    }

    pub fn end(&self) -> &Occurence<FixedOffset> {
        &self.end
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical_event
    }

    pub fn is_allday(&self) -> bool {
        self.begin.is_allday()
    }
}
