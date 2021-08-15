mod calendar;
mod error;

pub use calendar::{Calendar, Collection, Event, OccurrenceSpec};
pub use error::{Error, ErrorKind};

pub type IcalResult<T> = std::result::Result<T, crate::ical::Error>;

const ISO8601_2004_LOCAL_FORMAT: &'static str = "%Y%m%dT%H%M%S";
const ISO8601_2004_LOCAL_FORMAT_DATE: &'static str = "%Y%m%d";
