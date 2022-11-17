pub mod calendar;
pub mod datetime;
pub mod event;
pub mod ser;

pub use calendar::Calendar;
pub use event::Event;

use std::fs;
use std::io;
use std::path::Path;

use super::{Error, ErrorKind, Result};
use crate::config::CollectionConfig;
use crate::provider;

use ical::property::Property;

type PropertyList = Vec<Property>;

const JACKAL_PRODID: &'static str = "-//JACKAL//NONSGML Calendar//EN";
const JACKAL_CALENDAR_VERSION: &'static str = "2.0";

const ISO8601_2004_LOCAL_FORMAT: &'static str = "%Y%m%dT%H%M%S";
const ISO8601_2004_LOCAL_FORMAT_DATE: &'static str = "%Y%m%d";

const ICAL_FILE_EXT: &'static str = ".ics";

pub fn from_dir(path: &Path, config: &CollectionConfig) -> Vec<Calendar> {
    if !path.is_dir() {
        return Err(Error::new(
            ErrorKind::CalendarParse,
            &format!("'{}' is not a directory", path.display()),
        ));
    }

    // TODO: Fix ordering of configs/calendar dirs
    let calendars: Vec<Calendar> = fs::read_dir(&path)?
        .zip(&config.calendars)
        .map(|(dir, config)| {
            dir.map_or_else(
                |_| -> Result<_> { Err(Error::from(io::ErrorKind::InvalidData)) },
                |file: fs::DirEntry| -> Result<Calendar> {
                    calendar::from_dir(file.path().as_path(), &config)
                },
            )
        })
        .inspect(|res| {
            if let Err(err) = res {
                log::warn!("{}", err)
            }
        })
        .filter_map(Result::ok)
        .collect();

    calendars
}

// pub fn calendars_from_dir(path: &Path, calendar_specs: &[CalendarSpec]) -> Result<Collection> {
//     if !path.is_dir() {
//         return Err(Error::new(
//             ErrorKind::CalendarParse,
//             &format!("'{}' is not a directory", path.display()),
//         ));
//     }

//     if calendar_specs.is_empty() {
//         return Self::from_dir(path);
//     }

//     let calendars: Vec<Calendar> = calendar_specs
//         .into_iter()
//         .filter_map(|spec| match Calendar::from_dir(&path.join(&spec.id)) {
//             Ok(calendar) => Some(calendar.with_name(spec.name.clone())),
//             Err(_) => None,
//         })
//         .collect();

//     Ok(Collection {
//         path: path.to_owned(),
//         friendly_name: path.file_stem().unwrap().to_string_lossy().to_string(),
//         calendars,
//     })
// }
