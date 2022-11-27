use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::BTreeMap;
use std::ops::{Bound, Deref};
use std::path::{Path, PathBuf};
use store_interval_tree::{Interval, IntervalTree};

use super::{Calendarlike, EventFilter, Eventlike, Occurrence};

type Uid = String;

pub struct CalendarCore<Event: Eventlike> {
    pub(super) path: PathBuf,
    pub(super) _identifier: String,
    pub(super) friendly_name: String,
    pub(super) tz: Tz,
    events: IntervalTree<DateTime<Utc>, Vec<Event>>,
    uid_to_interval: BTreeMap<Uid, Interval<DateTime<Utc>>>,
}

impl<Event: Eventlike> CalendarCore<Event> {
    pub fn new(path: PathBuf, identifier: String, friendly_name: String, tz: Tz) -> Self {
        Self {
            path,
            _identifier: identifier,
            friendly_name,
            tz,
            events: IntervalTree::new(),
            uid_to_interval: BTreeMap::new(),
        }
    }

    //pub fn _new_with_name(path: &Path, name: String) -> Self {
    //    let identifier = uuid::Uuid::new_v4().hyphenated();

    //    Self {
    //        path: path.to_owned(),
    //        _identifier: identifier.to_string(),
    //        friendly_name: name,
    //        tz: Tz::UTC,
    //        events: IntervalTree::new(),
    //    }
    //}

    //pub fn _with_name(mut self, name: String) -> Self {
    //    self._set_name(name);
    //    self
    //}

    //pub fn _set_name(&mut self, name: String) {
    //    self.friendly_name = name;
    //}
    pub fn insert(&mut self, event: Event) {
        let (first, last) = event.occurrence_rule().clone().with_tz(&Utc {}).as_range();
        let interval = Interval::new(first, last);

        let uid = event.uid().to_owned();

        // check if interval is already in tree
        if let Some(mut entry) = self
            .events
            .query_mut(&interval)
            .find(|entry| entry.interval() == &interval)
        {
            entry.value().push(event)
        } else {
            self.events.insert(Interval::new(first, last), vec![event])
        }

        let prev = self.uid_to_interval.insert(uid, interval);
        assert!(prev.is_none(), "duplicate event uid");
    }

    pub fn remove_via_uid(&mut self, uid: &str) {
        let interval = self.uid_to_interval.remove(uid).unwrap();

        // There is no direct accessor for a specific interval in the intervaltree, meh...
        let mut entry = self
            .events
            .query_mut(&interval)
            .find(|e| *e.interval() == interval)
            .unwrap();

        let val = entry.value();
        val.retain(|e| e.uid() != uid);
        if val.is_empty() {
            self.events.delete(&interval);
        }
    }
}

impl<Event: Eventlike + 'static, T: Deref<Target = CalendarCore<Event>>> Calendarlike for T {
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
        let begin_dt = match &begin {
            Bound::Unbounded => DateTime::<Utc>::MIN_UTC,
            Bound::Included(dt) => dt.clone(),
            Bound::Excluded(dt) => dt.clone() + Duration::seconds(1),
        };

        let end_dt = match &end {
            Bound::Unbounded => DateTime::<Utc>::MAX_UTC,
            Bound::Included(dt) => dt.clone(),
            Bound::Excluded(dt) => dt.clone() - Duration::seconds(1),
        };
        self.events
            .query(&Interval::new(begin, end))
            .flat_map(|entry| entry.value().iter())
            .flat_map(|event| {
                event
                    .occurrence_rule()
                    .iter()
                    .skip_while(|ts| ts.begin().with_timezone(&Utc) <= begin_dt)
                    .take_while(|ts| ts.begin().with_timezone(&Utc) <= end_dt)
                    .map(move |ts| Occurrence {
                        span: ts.with_tz(&Utc),
                        event: event as &'a dyn Eventlike,
                    })
            })
            .collect()
    }

    fn filter_events<'a>(&'a self, filter: EventFilter) -> Vec<Occurrence<'a>> {
        match filter {
            EventFilter::InRange(begin, end) => {
                let begin_dt = match begin {
                    Bound::Unbounded => Bound::Unbounded,
                    Bound::Included(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
                    Bound::Excluded(dt) => Bound::Excluded(Utc.from_utc_datetime(&dt)),
                };

                let end_dt = match end {
                    Bound::Unbounded => Bound::Unbounded,
                    Bound::Included(dt) => Bound::Included(Utc.from_utc_datetime(&dt)),
                    Bound::Excluded(dt) => Bound::Excluded(Utc.from_utc_datetime(&dt)),
                };
                self.events_in(begin_dt, end_dt)
            }
        }
    }
}
