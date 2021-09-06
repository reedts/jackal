mod calendar;
mod error;

pub use calendar::{Calendar, Collection, Event, OccurrenceSpec};
pub use error::{Error, ErrorKind};

use chrono::{Month, NaiveDate};

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;

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
