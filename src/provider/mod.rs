use chrono::{DateTime, Duration, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use rrule::RRule;
use std::default::Default;
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod calendar;
pub mod datetime;
pub mod error;

pub mod ical;

pub use calendar::*;
pub use datetime::*;
pub use error::*;

pub type Result<T> = std::result::Result<T, self::Error>;

pub struct EventFilter {
    pub begin: Bound<NaiveDateTime>,
    pub end: Bound<NaiveDateTime>,
}

impl Default for EventFilter {
    fn default() -> Self {
        EventFilter {
            begin: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }
}

impl EventFilter {
    pub fn _from_datetime(mut self, date: Bound<NaiveDateTime>) -> Self {
        self.begin = date;
        self
    }

    pub fn _to_datetime(mut self, date: Bound<NaiveDateTime>) -> Self {
        self.end = date;
        self
    }

    pub fn datetime_range<R: RangeBounds<NaiveDateTime>>(mut self, range: R) -> Self {
        self.begin = range.start_bound().cloned();
        self.end = range.end_bound().cloned();

        self
    }
}

pub struct NewEvent<Tz: TimeZone> {
    pub begin: DateTime<Tz>,
    pub tz: Tz,
    pub end: Option<DateTime<Tz>>,
    pub duration: Option<Duration>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rrule: Option<RRule<rrule::Unvalidated>>,
}

impl<Tz: TimeZone> NewEvent<Tz> {
    pub fn new(begin: DateTime<Tz>) -> NewEvent<Tz> {
        let tz = begin.timezone();
        NewEvent {
            begin,
            tz,
            end: None,
            duration: None,
            title: None,
            description: None,
            rrule: None,
        }
    }
    pub fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_string());
    }

    pub fn set_description(&mut self, description: &str) {
        self.description = Some(description.to_string());
    }

    pub fn set_begin(&mut self, begin: NaiveDateTime) {
        self.begin = self.tz.from_local_datetime(&begin).earliest().unwrap();
    }

    pub fn _set_end(&mut self, end: NaiveDateTime) {
        self.end = Some(self.tz.from_local_datetime(&end).earliest().unwrap());
        self.duration = None;
    }

    pub fn _set_duration(&mut self, duration: Duration) {
        self.duration = Some(duration);
        self.end = None;
    }

    pub fn _set_repeat(&mut self, freq: rrule::Frequency, interval: u16) {
        self.rrule = Some(RRule::new(freq).interval(interval));
    }
}

pub trait Eventlike {
    fn title(&self) -> &str;
    fn set_title(&mut self, title: &str);
    fn uuid(&self) -> Uuid;
    fn summary(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn set_summary(&mut self, summary: &str);
    fn occurrence(&self) -> &Occurrence<Tz>;
    fn set_occurrence(&mut self, occurrence: Occurrence<Tz>);
    fn tz(&self) -> &Tz;
    fn set_tz(&mut self, tz: &Tz);
    fn begin(&self) -> DateTime<Tz>;
    fn end(&self) -> DateTime<Tz>;
    fn duration(&self) -> Duration;
}

pub struct EventIter<'a, E: Eventlike> {
    inner: Box<dyn Iterator<Item = &'a E> + 'a>,
}

impl<'a, E: Eventlike> Iterator for EventIter<'a, E> {
    type Item = &'a E;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub trait Calendarlike {
    type Event: Eventlike;
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn tz(&self) -> &Tz;
    fn set_tz(&mut self, tz: &Tz);
    fn event_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &(dyn Eventlike + 'a)> + 'a>;
    fn filter_events<'a>(
        &'a self,
        filter: EventFilter,
    ) -> Box<dyn Iterator<Item = (&DateTime<Tz>, &(dyn Eventlike + 'a))> + 'a>;
    fn new_event(&mut self);
}

pub trait CalendarMut: Calendarlike {
    fn events_mut<'a>(&'a mut self) -> EventIter<'a, <Self as Calendarlike>::Event>;
    fn add_event(&mut self, event: NewEvent<Tz>) -> Result<()>;
}

pub struct CalendarIter<'a, C: Calendarlike> {
    inner: std::slice::Iter<'a, C>,
}

impl<'a, C: Calendarlike> Iterator for CalendarIter<'a, C> {
    type Item = <std::slice::Iter<'a, C> as Iterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct CalendarIterMut<'a, C: CalendarMut> {
    inner: std::slice::IterMut<'a, C>,
}

impl<'a, C: CalendarMut> Iterator for CalendarIterMut<'a, C> {
    type Item = <std::slice::IterMut<'a, C> as Iterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct Collection<C: Calendarlike + CalendarMut> {
    _name: String,
    _path: PathBuf,
    calendars: Vec<C>,
}

impl<C: Calendarlike + CalendarMut> Collection<C> {
    pub fn _name(&self) -> &str {
        &self._name
    }

    pub fn _path(&self) -> &Path {
        &self._path
    }

    pub fn calendars<'a>(&'a self) -> CalendarIter<'a, C> {
        CalendarIter {
            inner: self.calendars.iter(),
        }
    }

    pub fn calendars_mut<'a>(&'a mut self) -> CalendarIterMut<'a, C> {
        CalendarIterMut {
            inner: self.calendars.iter_mut(),
        }
    }

    pub fn _events<'a>(&'a self) -> EventIter<'a, C::Event> {
        unimplemented!()
    }

    pub fn _new_calendar(&mut self) {
        unimplemented!()
    }
}
