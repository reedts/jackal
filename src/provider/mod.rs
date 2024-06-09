use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use rrule::RRule;
use std::default::Default;
use std::ops::{Bound, RangeBounds};
use std::path::Path;

pub mod alarm;
pub mod calendar;
pub mod datetime;
pub mod error;
pub mod tz;

pub mod ical;

pub use alarm::*;
pub use calendar::*;
pub use datetime::*;
pub use error::*;

use self::tz::*;

pub type Result<T> = std::result::Result<T, self::Error>;

pub type Uid = String;

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

    pub fn days(&self) -> Vec<TimeSpan<Tz>> {
        fn unroll<'dt, Tz: TimeZone>(
            from: &'dt DateTime<Tz>,
            to: &'dt DateTime<Tz>,
        ) -> impl Iterator<Item = TimeSpan<Tz>> + 'dt {
            let begin_date = from.date_naive();
            let end_date = to.date_naive();
            let tz = from.timezone();

            begin_date
                .iter_days()
                .take_while(move |date| date <= &end_date)
                .map(move |date| {
                    if &date == &begin_date {
                        TimeSpan::TimePoints(
                            from.clone(),
                            tz.from_local_datetime(
                                (begin_date + Duration::days(1))
                                    .and_hms_opt(0, 0, 0)
                                    .as_ref()
                                    .unwrap(),
                            )
                            .latest()
                            .expect("At least one LocalTime must exist"),
                        )
                    } else if &date == &end_date {
                        TimeSpan::TimePoints(
                            tz.from_local_datetime(end_date.and_hms_opt(0, 0, 0).as_ref().unwrap())
                                .earliest()
                                .expect("At least one LocalTime must exist"),
                            to.clone(),
                        )
                    } else {
                        TimeSpan::Allday(date, None, tz.clone())
                    }
                })
        }

        if self.span.num_days() > 1 {
            match &self.span {
                TimeSpan::Allday(begin, end, tz) => {
                    if let Some(e) = end {
                        begin
                            .iter_days()
                            .take_while(|date| date < e)
                            .map(|date| TimeSpan::Allday(date, None, tz.clone()))
                            .collect()
                    } else {
                        vec![TimeSpan::Allday(begin.clone(), None, tz.clone())]
                    }
                }
                TimeSpan::TimePoints(begin, end) => unroll(begin, end).collect(),
                TimeSpan::Duration(begin, dur) => {
                    unroll(begin, &(begin.clone() + dur.clone())).collect()
                }
                ts @ TimeSpan::Instant(_) => vec![ts.clone()],
            }
        } else {
            vec![self.span.clone()]
        }
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
    fn event_by_uid(&self, uid: &str) -> Option<&dyn Eventlike>;
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
