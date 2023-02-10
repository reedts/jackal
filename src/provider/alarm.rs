use chrono::{DateTime, Duration, TimeZone, Utc};
use derive_more::Constructor;

use super::tz::*;
use super::{Eventlike, Occurrence, OccurrenceIter, TimeSpan, Uid};

#[derive(Clone)]
pub struct Alarm<'e, Tz: TimeZone> {
    datetime: DateTime<Tz>,
    generator: &'e AlarmGenerator,
    occurrence: Occurrence<'e>,
}

impl<'e, Tz: TimeZone> PartialEq for Alarm<'e, Tz> {
    fn eq(&self, other: &Self) -> bool {
        self.datetime == other.datetime
    }
}

impl<Tz: TimeZone> Eq for Alarm<'_, Tz> {}

impl<'e, Tz: TimeZone> PartialOrd for Alarm<'e, Tz> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.datetime.cmp(&other.datetime))
    }
}

impl<'e, Tz: TimeZone> Ord for Alarm<'e, Tz> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.datetime.cmp(&other.datetime)
    }
}

impl<'e, Tz: TimeZone> Alarm<'e, Tz> {
    pub fn datetime(&self) -> &DateTime<Tz> {
        &self.datetime
    }

    pub fn description(&self) -> Option<&str> {
        self.generator.description.as_deref()
    }

    pub fn occurrence(&self) -> &Occurrence<'e> {
        &self.occurrence
    }
}

#[derive(Clone)]
pub enum AlarmTrigger {
    Start(Duration),
    End(Duration),
    Absolute(DateTime<Utc>),
}

#[derive(Clone, Constructor)]
pub struct AlarmGenerator {
    trigger: AlarmTrigger,
    repeat: Option<u32>,
    wait: Option<Duration>,
    description: Option<String>,
    event_uid: Uid,
}

impl AlarmGenerator {
    pub fn occurrence_alarms<'a>(&'a self, occurrence: Occurrence<'a>) -> Vec<Alarm<'a, Tz>> {
        let mut span = occurrence.span.clone();
        let mut ret = vec![match self.trigger {
            AlarmTrigger::Start(d) => {
                span = span.add_to_begin(d);
                Alarm {
                    datetime: span.begin(),
                    generator: &self,
                    occurrence: occurrence.clone(),
                }
            }
            AlarmTrigger::End(d) => {
                span = span.add_to_end(d);
                Alarm {
                    datetime: span.end(),
                    generator: &self,
                    occurrence: occurrence.clone(),
                }
            }
            AlarmTrigger::Absolute(dt) => {
                span = TimeSpan::Instant(dt.with_timezone(&Tz::utc()));
                Alarm {
                    datetime: span.begin(),
                    generator: &self,
                    occurrence: occurrence.clone(),
                }
            }
        }];

        let repitions: Vec<Alarm<'a, Tz>> = (1..self.repeat.unwrap_or(1))
            .into_iter()
            .scan(span, |span, _| match self.trigger {
                AlarmTrigger::Start(_) | AlarmTrigger::Absolute(_) => {
                    // FIXME: .clone() necessary?
                    *span = span.clone().add_to_begin(self.wait.unwrap());
                    Some(Alarm {
                        datetime: span.begin(),
                        generator: &self,
                        occurrence: occurrence.clone(),
                    })
                }
                AlarmTrigger::End(_) => {
                    // FIXME: .clone() necessary?
                    *span = span.clone().add_to_end(self.wait.unwrap());
                    Some(Alarm {
                        datetime: span.end(),
                        generator: &self,
                        occurrence: occurrence.clone(),
                    })
                }
            })
            .collect();
        ret.extend(repitions);
        ret
    }

    pub fn all_alarms<'a>(&'a self, event: &'a impl Eventlike) -> AlarmIter<'a> {
        AlarmIter::new(event, &self)
    }

    pub fn event_uid(&self) -> &str {
        self.event_uid.as_ref()
    }
}

pub struct AlarmIter<'a> {
    next: Vec<Alarm<'a, Tz>>,
    rrule_iter: OccurrenceIter<'a, Tz>,
    inner: &'a AlarmGenerator,
    event: &'a dyn Eventlike,
}

impl<'a> AlarmIter<'a> {
    fn new(event: &'a impl Eventlike, generator: &'a AlarmGenerator) -> Self {
        let mut rrule_iter = event.occurrence_rule().iter();
        let next = rrule_iter
            .next()
            .map(|ts| generator.occurrence_alarms(Occurrence { span: ts, event }))
            .unwrap_or_default();
        AlarmIter {
            next,
            rrule_iter,
            inner: generator,
            event: event as &dyn Eventlike,
        }
    }
}

impl<'a> Iterator for AlarmIter<'a> {
    type Item = Alarm<'a, Tz>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(alarm) = self.next.pop() {
            if self.next.is_empty() {
                self.next = self
                    .rrule_iter
                    .next()
                    .map(|ts| {
                        self.inner.occurrence_alarms(Occurrence {
                            span: ts,
                            event: self.event,
                        })
                    })
                    .unwrap_or_default();
            }
            Some(alarm)
        } else {
            None
        }
    }
}
