use chrono_tz::Tz;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::config::CalendarConfig;
use crate::provider::ical::event::uid_from_path;
use crate::provider::ical::ICAL_FILE_EXT;
use crate::provider::{self, CalendarCore, Eventlike};
use crate::provider::{MutCalendarlike, NewEvent, OccurrenceRule, TimeSpan};

use super::ser::to_string;
use super::{Error, ErrorKind, Event, Result};

pub struct Calendar {
    inner: provider::CalendarCore<Event>,
    _modification_watcher: notify::RecommendedWatcher,
    pending_modifications: mpsc::Receiver<CalendarModification>,
    current_modifications: HashSet<CalendarModification>,
}

impl std::ops::Deref for Calendar {
    type Target = CalendarCore<Event>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub fn from_dir(
    path: &Path,
    config: &CalendarConfig,
    event_sink: &std::sync::mpsc::Sender<crate::events::Event>,
) -> Result<Calendar> {
    if !path.is_dir() {
        return Err(Error::new(
            ErrorKind::CalendarParse,
            &format!("'{}' is not a directory", path.display()),
        ));
    }

    let mut event_file_iter = fs::read_dir(&path)?
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
        .filter_map(Result::ok)
        .peekable();

    let tz = if let Some(event) = event_file_iter.peek() {
        *(event.tz())
    } else {
        Tz::UTC
    };

    let mut inner = CalendarCore::new(path.to_owned(), config.id.clone(), config.name.clone(), tz);

    for event in event_file_iter {
        if let Err(e) = inner.insert(event) {
            return Err(Error::new(
                ErrorKind::CalendarParse,
                &format!("Duplicate event uid '{}'", e.uid()),
            ));
        }
    }

    let (wachter, queue) = ical_watcher(path, event_sink.clone());
    Ok(Calendar {
        inner,
        _modification_watcher: wachter,
        pending_modifications: queue,
        current_modifications: HashSet::new(),
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

        let event_uid = uuid::Uuid::new_v4();
        let target_path =
            self.path
                .join(&format!("{}.{}", event_uid.as_hyphenated(), ICAL_FILE_EXT));
        let source_path =
            std::env::temp_dir().join(&format!("{}.{}", event_uid.as_hyphenated(), ICAL_FILE_EXT));

        self.current_modifications
            .insert(CalendarModification::Create(target_path.clone()));

        let mut file = fs::File::create(&source_path)?;

        let mut event = Event::new(&source_path, occurrence)?;

        if let Some(title) = new_event.title {
            event.set_title(title.as_ref());
        }

        if let Some(description) = new_event.description {
            event.set_summary(description.as_ref());
        }

        // TODO: serde
        let s = to_string(&event.as_ical())?;
        log::info!("{}", s);
        file.write_all(s.as_bytes())?;

        // fs::rename does not work over different mount points
        fs::copy(&source_path, &target_path)?;
        fs::remove_file(source_path)?;

        self.inner
            .insert(event.move_to_dir(&target_path.parent().unwrap()))
            .map_err(|e| {
                Error::new(
                    ErrorKind::CalendarParse,
                    &format!("Duplicate event uid '{}'", e.uid()),
                )
            })?;

        self.current_modifications
            .remove(&CalendarModification::Create(target_path.clone()));

        Ok(())
    }
    fn process_external_modifications(&mut self) {
        fn remove_for_path(calendar: &mut CalendarCore<Event>, path: &Path) {
            let Some(uid) = uid_from_path(path) else {
                log::warn!("Unable to obtain uid from file removal event path '{}'", path.to_string_lossy());
                return;
            };
            if !calendar.remove_via_uid(&uid) {
                log::info!(
                    "Event with uid {} could not be removed (double remove event?)",
                    uid
                );
            }
        }
        fn add_for_path(calendar: &mut CalendarCore<Event>, path: &Path) {
            let event = match Event::from_file(path) {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("{}", e);
                    return;
                }
            };
            if let Err(event) = calendar.insert(event) {
                log::info!(
                    "Event with uid {} is already in the calendar (double insert event?)",
                    event.uid()
                );
            }
        }
        for m in self.pending_modifications.try_iter() {
            if !self.current_modifications.contains(&m) {
                match m {
                    CalendarModification::Create(path) => add_for_path(&mut self.inner, &path),
                    CalendarModification::Remove(path) => remove_for_path(&mut self.inner, &path),
                    CalendarModification::Modify(path) => {
                        remove_for_path(&mut self.inner, &path);
                        add_for_path(&mut self.inner, &path);
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum CalendarModification {
    Create(PathBuf),
    Remove(PathBuf),
    Modify(PathBuf),
}

#[must_use]
fn ical_watcher(
    path: &Path,
    event_sink: mpsc::Sender<crate::events::Event>,
) -> (
    notify::RecommendedWatcher,
    mpsc::Receiver<CalendarModification>,
) {
    use notify::{RecursiveMode, Watcher};

    fn is_ical(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            ext == ICAL_FILE_EXT
        } else {
            false
        }
    }

    fn relevant_modification(event: notify::Event) -> Option<CalendarModification> {
        use notify::event::*;
        match event.kind {
            EventKind::Create(CreateKind::File) if is_ical(&event.paths[0]) => {
                Some(CalendarModification::Create(event.paths[0].clone()))
            }
            EventKind::Remove(RemoveKind::File)
            | EventKind::Modify(ModifyKind::Name(RenameMode::From))
                if is_ical(&event.paths[0]) =>
            {
                Some(CalendarModification::Remove(event.paths[0].clone()))
            }
            EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Name(RenameMode::To))
                if is_ical(&event.paths[0]) =>
            {
                Some(CalendarModification::Modify(event.paths[0].clone()))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                // TODO: Maybe we want to return both events here.
                // However, for the specific case of ical we don't really expect a rename (from
                // ical to ical) because that would imply a changing of uuids!
                if is_ical(&event.paths[0]) {
                    Some(CalendarModification::Remove(event.paths[0].clone()))
                } else if is_ical(&event.paths[1]) {
                    // It may appear weird that we are emiting "modify" events when something is
                    // renamed/moved to an .ics file. The reason for this is that we have no
                    // information about whether the file existed before. Hence we take the safe
                    // option of (possibly pointlessly) removing old files.
                    Some(CalendarModification::Modify(event.paths[1].clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    let (queue_writer, queue_reader) = mpsc::channel();

    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                if let Some(m) = relevant_modification(event) {
                    let _ = event_sink.send(crate::events::Event::ExternalModification);
                    let _ = queue_writer.send(m);
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        })
        .unwrap();

    watcher.watch(path, RecursiveMode::Recursive).unwrap();
    (watcher, queue_reader)
}
