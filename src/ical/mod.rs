mod calendar;
mod error;

pub use calendar::{Calendar, Collection, DateTimeSpec, Event, OccurrenceSpec};
pub use error::{Error, ErrorKind};

use chrono::{Month, NaiveDate, Utc};

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;

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
struct EventBuilder {
    summary: String,
    occurence: OccurrenceSpec,
}

impl EventBuilder {
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_occurnce(mut self, occurence: OccurrenceSpec) -> Self {
        self.occurence = occurence;
        self
    }
    // pub fn finish(self) -> IcalResult<Calendar> {}
}
