use crate::ical::event::Event;
use chrono::{FixedOffset, TimeZone};
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

impl Calendar<FixedOffset> {
    pub fn from(path: &Path) -> io::Result<Calendar<FixedOffset>> {
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

        let events: Vec<Event<FixedOffset>> = ical
            .events
            .iter()
            .map(|ev| Event::from(ev.clone()))
            .inspect(|ev| {
                if let Err(e) = ev {
                    println!("ERROR: {:?}", e)
                }
            })
            .filter_map(Result::ok)
            .collect();

        Ok(Calendar {
            path: path.into(),
            ical,
            events,
        })
    }

    pub fn events(&self) -> &Vec<Event<FixedOffset>> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event<FixedOffset>> {
        &mut self.events
    }
}
