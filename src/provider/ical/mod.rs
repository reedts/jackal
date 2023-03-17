pub mod calendar;
pub mod datetime;
pub mod event;
pub mod ser;

pub use calendar::Calendar;
pub use event::Event;

use std::path::{Path, PathBuf};

use crate::config::CalendarConfig;
use crate::provider::*;

use ::ical::parser::Component;
use ::ical::property::Property;

type PropertyList = Vec<Property>;

const JACKAL_PRODID: &'static str = "-//JACKAL//NONSGML Calendar//EN";
const JACKAL_CALENDAR_VERSION: &'static str = "2.0";

const ISO8601_2004_LOCAL_FORMAT: &'static str = "%Y%m%dT%H%M%S";
const ISO8601_2004_UTC_FORMAT: &'static str = "%Y%m%dT%H%M%SZ";
const ISO8601_2004_LOCAL_FORMAT_DATE: &'static str = "%Y%m%d";

const ICAL_FILE_EXT: &'static str = "ics";

pub fn from_dir(
    path: &Path,
    config: &[CalendarConfig],
    event_sink: &std::sync::mpsc::Sender<crate::events::Event>,
) -> Result<Vec<ProviderCalendar>> {
    if !path.is_dir() {
        return Err(Error::new(
            ErrorKind::CalendarParse,
            &format!("'{}' is not a directory", path.display()),
        ));
    }

    let calendars = config
        .iter()
        .map(|c| calendar::from_dir(path.join(PathBuf::from(&c.id)).as_ref(), c, event_sink))
        .inspect(|res| {
            if let Err(err) = res {
                log::error!("Could not load calendar: {}", err)
            }
        })
        .filter_map(Result::ok)
        .map(ProviderCalendar::Ical)
        .collect();

    Ok(calendars)
}
