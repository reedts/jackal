use crate::ical::event::Event;
use chrono::{TimeZone, Utc};
use ical::parser::ical::component::IcalCalendar;
use ical::parser::ical::IcalParser;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub struct Calendar<Tz: TimeZone> {
    path: PathBuf,
    ical: IcalCalendar,
    events: Vec<Event<Tz>>,
}

impl Calendar<Utc> {
    pub fn from(path: &Path) -> io::Result<Calendar<Utc>> {
        let buf = io::BufReader::new(File::open(path)?);

        let mut reader = IcalParser::new(buf);

        let ical: IcalCalendar = match reader.next() {
            Some(cal) => match cal {
                Ok(c) => c,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "No calendar could be read from '{p}': {e}",
                            p = path.to_str().unwrap_or(""),
                            e = e
                        ),
                    ))
                }
            },
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("No calendar found in '{}'", path.to_str().unwrap_or("")),
                ))
            }
        };

        let events: Vec<Event<Utc>> = ical
            .events
            .iter()
            .map(|ev| Event::from(ev.clone()))
            .filter_map(Result::ok)
            .collect();

        Ok(Calendar {
            path: path.into(),
            ical, 
            events
        })
    }

    pub fn events(&self) -> &Vec<Event<Utc>> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event<Utc>> {
        &mut self.events
    }
}
