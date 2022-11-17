use chrono_tz::Tz;
use std::path::{Path, PathBuf};
use uuid;

use super::{Calendarlike, Eventlike};

pub struct Calendar<Event: Eventlike> {
    pub(super) path: PathBuf,
    pub(super) _identifier: String,
    pub(super) friendly_name: String,
    pub(super) tz: Tz,
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
        }
    }

    pub fn _new_with_name(path: &Path, name: String) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: name,
            tz: Tz::UTC,
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
    fn name(&self) -> &str {
        &self.friendly_name
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tz(&self) -> &Tz {
        &self.tz
    }
}
