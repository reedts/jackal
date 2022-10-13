use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use uuid;

use super::{Calendarlike, EventFilter, EventIter, Eventlike};

pub struct Calendar<Event: Eventlike> {
    pub(super) path: PathBuf,
    pub(super) _identifier: String,
    pub(super) friendly_name: String,
    pub(super) tz: Tz,
    pub(super) events: BTreeMap<DateTime<Tz>, Vec<Rc<Event>>>,
}

impl<Event: Eventlike> Calendar<Event> {
    pub fn _new(path: &Path) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();
        let friendly_name = identifier.clone();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: friendly_name.to_string(),
            tz: Tz::UTC,
            events: BTreeMap::new(),
        }
    }

    pub fn _new_with_name(path: &Path, name: String) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: name,
            tz: Tz::UTC,
            events: BTreeMap::new(),
        }
    }

    pub fn _with_name(mut self, name: String) -> Self {
        self._set_name(name);
        self
    }

    pub fn _set_name(&mut self, name: String) {
        self.friendly_name = name;
    }
}

impl<Event: Eventlike> Calendarlike for Calendar<Event> {
    type Event = Event;
    fn name(&self) -> &str {
        &self.friendly_name
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tz(&self) -> &Tz {
        &self.tz
    }

    fn set_tz(&mut self, _tz: &Tz) {
        unimplemented!();
    }

    fn events<'a>(&'a self) -> EventIter<'a, Self::Event> {
        EventIter {
            inner: Box::new(
                self.events
                    .iter()
                    .flat_map(|(_, v)| v.iter())
                    .map(|v| v.as_ref()),
            ),
        }
    }

    fn filter_events<'a>(&'a self, filter: EventFilter) -> EventIter<'a, Self::Event> {
        // TODO: Change once https://github.com/rust-lang/rust/issues/86026 is stable
        let real_begin = match filter.begin {
            Bound::Included(dt) => {
                Bound::Included(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            Bound::Excluded(dt) => {
                Bound::Excluded(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            _ => Bound::Unbounded,
        };
        let real_end = match filter.end {
            Bound::Included(dt) => {
                Bound::Included(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            Bound::Excluded(dt) => {
                Bound::Excluded(self.tz().from_local_datetime(&dt).earliest().unwrap())
            }
            _ => Bound::Unbounded,
        };

        EventIter {
            inner: Box::new(
                self.events
                    .range((real_begin, real_end))
                    .flat_map(|(_, v)| v.iter())
                    .map(|v| v.as_ref()),
            ),
        }
    }
}
