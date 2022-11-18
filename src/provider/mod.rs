use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rrule::RRule;
use std::default::Default;
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};
use store_interval_tree::IntervalTreeIterator;

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
    fn uid(&self) -> &str;
    fn summary(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn occurrence_rule(&self) -> &OccurrenceRule<Tz>;
    fn tz(&self) -> &Tz;
    fn duration(&self) -> Duration;
}

pub struct Occurrence<'a> {
    span: TimeSpan<Utc>,
    event: &'a dyn Eventlike,
}

impl Occurrence<'_> {
    pub fn begin(&self) -> DateTime<Utc> {
        self.span.begin()
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.span.end()
    }

    pub fn event(&self) -> &dyn Eventlike {
        self.event
    }
}

pub trait Calendarlike {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn tz(&self) -> &Tz;
    fn events_in<'a>(
        &'a self,
        begin: Bound<DateTime<Utc>>,
        end: Bound<DateTime<Utc>>,
    ) -> Vec<Occurrence<'a>>;
}

pub trait MutCalendarlike: Calendarlike {
    fn add_event(&mut self, event: NewEvent<Tz>) -> Result<()>;
}

enum ProviderCalendar {
    Ical(self::ical::Calendar),
}
