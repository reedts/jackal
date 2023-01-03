use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::BTreeMap;
use std::ops::{Bound, Deref};
use std::path::{Path, PathBuf};
use store_interval_tree::{Interval, IntervalTree};

use super::{Alarm, AlarmGenerator, Calendarlike, EventFilter, Eventlike, Occurrence, Uid};

pub struct CalendarCore<Event: Eventlike> {
    pub(super) path: PathBuf,
    pub(super) _identifier: String,
    pub(super) friendly_name: String,
    pub(super) tz: Tz,
    events: IntervalTree<DateTime<Utc>, Vec<Event>>,
    alarms: IntervalTree<DateTime<Utc>, Vec<AlarmGenerator>>,
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
            alarms: IntervalTree::new(),
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

    /// Try to insert the event into the calendar. If an event with the same uid is already
    /// present, return the to-be-inserted event as an error.
    pub fn insert(&mut self, event: Event) -> Result<(), Event> {
        let uid = event.uid().to_owned();

        if self.uid_to_interval.contains_key(&uid) {
            return Err(event);
        }

        let (first, last) = event.occurrence_rule().clone().with_tz(&Utc).as_range();
        let interval = Interval::new(first, last);

        let first_span = event.occurrence_rule().first();
        let last_span = event.occurrence_rule().last();

        // Check for alarms in event
        let alarms: Vec<AlarmGenerator> = event.alarms().into_iter().cloned().collect();

        // Insert alarms
        for alarm in alarms {
            let first_alarm = Bound::Included(
                alarm
                    .occurrence_alarms(Occurrence {
                        span: first_span.clone(),
                        event: &event,
                    })
                    .first()
                    .unwrap()
                    .datetime()
                    .with_timezone(&Utc),
            );
            let last_alarm = if let Some(last) = &last_span {
                Bound::Included(
                    alarm
                        .occurrence_alarms(Occurrence {
                            span: last.clone(),
                            event: &event,
                        })
                        .last()
                        .unwrap()
                        .datetime()
                        .with_timezone(&Utc),
                )
            } else {
                Bound::Unbounded
            };

            let interval = Interval::new(first_alarm, last_alarm);
            if let Some(mut entry) = self
                .alarms
                .query_mut(&interval)
                .find(|entry| entry.interval() == &interval)
            {
                entry.value().push(alarm)
            } else {
                self.alarms.insert(interval, vec![alarm])
            }
        }

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
        assert!(
            prev.is_none(),
            "Duplicate should have already been handled above"
        );

        Ok(())
    }

    pub fn find_by_uid(&self, uid: &str) -> Option<&Event> {
        self.uid_to_interval
            .get(uid)
            .and_then(|interval| {
                self.events
                    .query(&interval)
                    .find(|e| *e.interval() == *interval)
            })
            .and_then(|v| v.value().iter().find(|ev| ev.uid() == uid))
    }

    /// Try to remove an event with the specified id. Returns whether or not such an event was
    /// present before and thus successfully removed.
    pub fn remove_by_uid(&mut self, uid: &str) -> bool {
        let Some(interval) = self.uid_to_interval.remove(uid) else {
            return false;
        };

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
        true
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
                    .skip_while(|ts| ts.end().with_timezone(&Utc) < begin_dt)
                    .take_while(|ts| ts.begin().with_timezone(&Utc) < end_dt)
                    .map(move |ts| Occurrence {
                        span: ts.with_tz(&Tz::UTC),
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

    fn alarms_in<'a>(
        &'a self,
        begin: Bound<DateTime<Utc>>,
        end: Bound<DateTime<Utc>>,
    ) -> Vec<Alarm<'a, Tz>> {
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

        self.alarms
            .query(&Interval::new(begin, end))
            .flat_map(|entry| entry.value().iter())
            .flat_map(|alarm| {
                alarm
                    .all_alarms(self.find_by_uid(alarm.event_uid()).unwrap())
                    .skip_while(|alarm| alarm.datetime().with_timezone(&Utc) <= begin_dt)
                    .take_while(|alarm| alarm.datetime().with_timezone(&Utc) <= end_dt)
            })
            .collect()
    }
}
