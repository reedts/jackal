use chrono::{
    DateTime,
    FixedOffset,
    NaiveDateTime,
    TimeZone,
    Utc
};
use chrono_tz::{Tz, UTC};

use ical::property::Property;
use ical::parser::ical::component::IcalEvent;

use crate::ical::{Error, ErrorKind};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Event<Tz: TimeZone> {
    begin: DateTime<Tz>,
    end: DateTime<Tz>,
    ical_event: IcalEvent,
}

#[derive(Debug)]
pub struct EventParseError {
    message: String
}

fn parse_prop_to_date_time(property: &Property) -> Result<DateTime<Utc>> {
    type DT = DateTime<FixedOffset>;
    let iso8601_2004_local_format = "%Y%M%DT%H%M%S";
    if !property.name.contains("^DT") {
        return Err(Error::new(ErrorKind::EventMissingKey).with_msg("No valid DTSTART found"));
    }

    let value = match &property.value {
        Some(v) => v,
        None => return Err(Error::new(ErrorKind::EventMissingKey).with_msg("No corresponding timestamp value"))
    };

    // Return if is already UTC
    if value.contains('Z') {
        return match DT::parse_from_rfc3339(&value) {
            Ok(dt) => Ok(dt.with_timezone(&Utc)),
            Err(_) => Err(Error::new(ErrorKind::TimeParse).with_msg("Parsing of timestamp to rfc3339 not possible"))
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
                    Ok(tz_date.with_timezone(&Utc))
                },
                None => {
                    // Must be localtime
                    let naive_dt = NaiveDateTime::parse_from_str(&value, iso8601_2004_local_format).unwrap();
                    // TODO: Configure this for local timezone
                    let tz_offset = FixedOffset::west(0);
                    Ok(Utc.from_utc_datetime(&(naive_dt - tz_offset)))
                }
            }
        },
        None => {
            // Must be localtime
            let naive_dt = NaiveDateTime::parse_from_str(&value, iso8601_2004_local_format).unwrap();
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
            None        => return Err(Error::new(ErrorKind::EventMissingKey).with_msg("No DTSTART found"))
        };

        let dend = match ical_event.properties.iter().find(|p| p.name == "DTEND") {
            Some(end) => parse_prop_to_date_time(&end)?,
            None      => return Err(Error::new(ErrorKind::EventMissingKey).with_msg("No DTEND found"))
        };


        Ok(Event {
            begin: dstart,
            end: dend,
            ical_event,
        })
    }

    pub fn summary(&self) -> &str {
        self.ical_event.properties.iter()
            .find(|prop| prop.name == "SUMMARY")
            .unwrap()
            .value.as_ref().unwrap().as_str()
    }

    pub fn begin(&self) -> &DateTime<Utc> {
        &self.begin
    }

    pub fn end(&self) -> &DateTime<Utc> {
        &self.end
    }
}

