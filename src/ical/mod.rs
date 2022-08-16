mod calendar;
mod error;

pub use calendar::{Calendar, Collection, DateTime, Duration, Event, Occurrence, TimeSpan};
pub use error::{Error, ErrorKind};

use chrono::{Month, NaiveDate, Utc};
use ical::parser::{ical::component::IcalEvent, Component};
use ical::property::Property;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;
type PropertyList = Vec<Property>;

const JACKAL_PRODID: &'static str = "-//JACKAL//NONSGML Calendar//EN";
const JACKAL_CALENDAR_VERSION: &'static str = "2.0";

const ISO8601_2004_LOCAL_FORMAT: &'static str = "%Y%m%dT%H%M%S";
const ISO8601_2004_LOCAL_FORMAT_DATE: &'static str = "%Y%m%d";

pub fn days_of_month(month: &Month, year: i32) -> u64 {
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

fn generate_timestamp() -> String {
    let tstamp = Utc::now();
    format!("{}Z", tstamp.format(ISO8601_2004_LOCAL_FORMAT))
}

#[derive(Default)]
pub struct EventBuilder {
    path: PathBuf,
    start: DateTime,
    end: Option<DateTime>,
    dur: Option<Duration>,
    ical: IcalEvent,
}

impl EventBuilder {
    pub fn new(path: &Path) -> Self {
        let mut builder = Self::default();
        builder.path = path.to_owned();

        builder
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.ical.add_property(Property {
            name: "SUMMARY".to_owned(),
            params: None,
            value: Some(summary),
        });
        self
    }

    pub fn with_start(mut self, start: DateTime) -> Self {
        self.start = start;
        self
    }

    pub fn with_end(mut self, end: DateTime) -> Self {
        self.dur = None;
        self.end = Some(end);
        self
    }

    pub fn with_dur(mut self, dur: Duration) -> Self {
        self.end = None;
        self.dur = Some(dur);
        self
    }

    pub fn with_loc(mut self, loc: String) -> Self {
        self.ical.add_property(Property {
            name: "LOCATION".to_owned(),
            params: None,
            value: Some(loc),
        });
        self
    }

    pub fn finish(self) -> IcalResult<Calendar> {
        let mut event = if let Some(dtspec) = self.end {
            Event::new_with_ical_properties(
                Occurrence::Onetime(TimeSpan::TimePoints(self.start, dtspec)),
                self.ical.properties,
            )
        } else if let Some(durspec) = self.dur {
            Event::new_with_ical_properties(
                Occurrence::Onetime(TimeSpan::Duration(self.start, durspec)),
                self.ical.properties,
            )
        } else {
            Event::new_with_ical_properties(Occurrence::Instant(self.start), self.ical.properties)
        };

        Ok(Calendar::from_event(&self.path, event))
    }
}
