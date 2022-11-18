use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use store_interval_tree::{Interval, IntervalTree};
use uuid;

use super::{Calendarlike, Eventlike, Occurrence, OccurrenceRule};

pub struct Calendar<Event: Eventlike> {
    pub(super) path: PathBuf,
    pub(super) _identifier: String,
    pub(super) friendly_name: String,
    pub(super) tz: Tz,
    pub(super) events: IntervalTree<DateTime<Utc>, Event>,
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
            events: IntervalTree::new(),
        }
    }

    pub fn _new_with_name(path: &Path, name: String) -> Self {
        let identifier = uuid::Uuid::new_v4().hyphenated();

        Self {
            path: path.to_owned(),
            _identifier: identifier.to_string(),
            friendly_name: name,
            tz: Tz::UTC,
            events: IntervalTree::new(),
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

    fn events_in<'a>(
        &'a self,
        begin: Bound<DateTime<Utc>>,
        end: Bound<DateTime<Utc>>,
    ) -> Vec<Occurrence<'a>> {
        let begin_dt = match begin {
            Bound::Unbounded => Utc.with_ymd_and_hms(1970, 0, 0, 0, 0, 0).unwrap(),
            Bound::Included(dt) => dt,
            Bound::Excluded(dt) => dt + Duration::seconds(1),
        };

        let end_dt = match begin {
            Bound::Unbounded => Utc.with_ymd_and_hms(1970, 0, 0, 0, 0, 0).unwrap(),
            Bound::Included(dt) => dt,
            Bound::Excluded(dt) => dt - Duration::seconds(1),
        };
        self.events
            .query(&Interval::new(begin, end))
            .flat_map(|entry| {
                entry
                    .value()
                    .occurrence_rule()
                    .iter()
                    .skip_while(|ts| ts.begin().with_timezone(&Utc) <= begin_dt)
                    .take_while(|ts| ts.begin().with_timezone(&Utc) >= end_dt)
                    .map(|ts| Occurrence {
                        span: ts.with_tz(&Utc),
                        event: entry.value() as &dyn Eventlike,
                    })
            })
            .collect()
    }
}
