use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use rrule::RRule;
use std::default::Default;
use std::ops::{Bound, RangeBounds};
use std::path::Path;

pub mod alarm;
pub mod calendar;
pub mod datetime;
pub mod error;

pub mod ical;

pub use alarm::*;
pub use calendar::*;
pub use datetime::*;
pub use error::*;

pub type Result<T> = std::result::Result<T, self::Error>;

type Uid = String;

pub enum EventFilter {
    InRange(Bound<NaiveDateTime>, Bound<NaiveDateTime>),
}

impl Default for EventFilter {
    fn default() -> Self {
        EventFilter::InRange(Bound::Unbounded, Bound::Unbounded)
    }
}

impl EventFilter {
    pub fn datetime_range<R: RangeBounds<NaiveDateTime>>(self, range: R) -> Self {
        EventFilter::InRange(range.start_bound().cloned(), range.end_bound().cloned())
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

    pub fn set_end(&mut self, end: NaiveDateTime) {
        self.end = Some(self.tz.from_local_datetime(&end).earliest().unwrap());
        self.duration = None;
    }

    pub fn set_duration(&mut self, duration: Duration) {
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
    fn alarms(&self) -> Vec<&AlarmGenerator>;
}

#[derive(Clone)]
pub struct Occurrence<'a> {
    pub span: TimeSpan<Tz>,
    pub event: &'a dyn Eventlike,
}

impl Occurrence<'_> {
    pub fn begin(&self) -> DateTime<Tz> {
        self.span.begin()
    }

    pub fn end(&self) -> DateTime<Tz> {
        self.span.end()
    }

    pub fn event(&self) -> &dyn Eventlike {
        self.event
    }

    pub fn alarms<'e>(&'e self) -> Vec<Alarm<'e, Tz>> {
        self.event
            .alarms()
            .iter()
            .flat_map(|alarm| alarm.occurrence_alarms(self.clone()).into_iter())
            .collect()
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
    fn filter_events<'a>(&'a self, filter: EventFilter) -> Vec<Occurrence<'a>>;
    fn alarms_in<'a>(
        &'a self,
        begin: Bound<DateTime<Utc>>,
        end: Bound<DateTime<Utc>>,
    ) -> Vec<Alarm<'a, Tz>>;
}

pub trait MutCalendarlike: Calendarlike {
    fn add_event(&mut self, event: NewEvent<Tz>) -> Result<()>;
    fn process_external_modifications(&mut self);
}

pub enum ProviderCalendar {
    Ical(self::ical::Calendar),
}

impl ProviderCalendar {
    pub fn name(&self) -> &str {
        match self {
            ProviderCalendar::Ical(c) => c.name(),
        }
    }

    pub fn as_calendar(&self) -> &dyn Calendarlike {
        match self {
            ProviderCalendar::Ical(cal) => cal as &dyn Calendarlike,
        }
    }

    pub fn process_external_modifications(&mut self) {
        match self {
            ProviderCalendar::Ical(i) => i.process_external_modifications(),
        }
    }
}
