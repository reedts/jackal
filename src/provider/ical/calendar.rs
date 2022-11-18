use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::config::CalendarConfig;
use crate::provider;
use crate::provider::{
    Calendarlike, Eventlike, MutCalendarlike, NewEvent, OccurrenceRule, TimeSpan,
};

use super::event::Event;
use super::{Error, ErrorKind, Result};

pub type Calendar = provider::Calendar<Event>;

pub fn from_dir(path: &Path, config: &CalendarConfig) -> Result<Calendar> {
    let mut events = BTreeMap::<DateTime<Tz>, Vec<Rc<Event>>>::new();

    if !path.is_dir() {
        return Err(Error::new(
            ErrorKind::CalendarParse,
            &format!("'{}' is not a directory", path.display()),
        ));
    }

    let event_file_iter = fs::read_dir(&path)?
        .map(|dir| {
            dir.map_or_else(
                |_| -> Result<_> { Err(Error::from(ErrorKind::CalendarParse)) },
                |file: fs::DirEntry| -> Result<Event> { Event::from_file(file.path().as_path()) },
            )
        })
        .inspect(|res| {
            if let Err(err) = res {
                log::warn!("{}", err)
            }
        })
        .filter_map(Result::ok);

    // TODO: use `BTreeMap::first_entry` once it's stable: https://github.com/rust-lang/rust/issues/62924
    let tz = if let Some((_, event)) = events.iter().next() {
        *(event.first().unwrap().tz())
    } else {
        Tz::UTC
    };

    let now = tz.from_utc_datetime(&Utc::now().naive_utc());

    for event in event_file_iter {
        let event_rc = Rc::new(event);

        event_rc
            .occurrence_rule()
            .iter()
            .skip_while(|dt| dt < &(now - Duration::days(356)))
            .take_while(|dt| dt <= &(now + Duration::days(356)))
            .for_each(|dt| events.entry(dt).or_default().push(Rc::clone(&event_rc)));
    }

    Ok(Calendar {
        path: path.to_owned(),
        _identifier: config.id.clone(),
        friendly_name: config.name.clone(),
        tz,
        events,
    })
}

impl MutCalendarlike for Calendar {
    fn add_event(&mut self, new_event: NewEvent<Tz>) -> Result<()> {
        let mut occurrence = if let Some(end) = new_event.end {
            OccurrenceRule::Onetime(TimeSpan::from_start_and_end(new_event.begin, end))
        } else if let Some(duration) = new_event.duration {
            OccurrenceRule::Onetime(TimeSpan::from_start_and_duration(new_event.begin, duration))
        } else {
            OccurrenceRule::Onetime(TimeSpan::from_start(new_event.begin))
        };

        if let Some(rrule) = new_event.rrule {
            occurrence = occurrence.with_recurring(
                rrule.build(
                    new_event
                        .begin
                        .with_timezone(&rrule::Tz::Tz(new_event.begin.timezone())),
                )?,
            );
        }

        let mut event = Rc::new(Event::new(&self.path, occurrence)?);

        if let Some(title) = new_event.title {
            Rc::get_mut(&mut event).unwrap().set_title(title.as_ref());
        }

        if let Some(description) = new_event.description {
            Rc::get_mut(&mut event)
                .unwrap()
                .set_summary(description.as_ref());
        }

        // TODO: serde
        // let mut file = fs::File::create(event.path())?;
        // write!(&mut file, "{}", event.ical);
        log::info!("{:?}", event.as_ical());

        let now = self.tz.from_utc_datetime(&Utc::now().naive_utc());

        event
            .occurrence_rule()
            .iter()
            .skip_while(|dt| dt < &(now - Duration::days(356)))
            .take_while(|dt| dt <= &(now + Duration::days(356)))
            .for_each(|dt| self.events.entry(dt).or_default().push(Rc::clone(&event)));

        Ok(())
    }
}
