use chrono::{Date, DateTime, Duration, Month, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use rrule::{RRuleSet, RRuleSetIter};
use std::convert::From;
use std::default::Default;
use std::ops::{Bound, RangeBounds};
use std::path::Path;
use uuid::Uuid;

pub mod error;
pub mod ical;

pub use error::*;

use crate::config::CalendarSpec;

pub type Result<T> = std::result::Result<T, self::Error>;

pub fn days_of_month(month: &Month, year: i32) -> u32 {
    if month.number_from_month() == 12 {
        NaiveDate::from_ymd(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd(year, month.number_from_month() as u32 + 1, 1)
    }
    .signed_duration_since(NaiveDate::from_ymd(
        year,
        month.number_from_month() as u32,
        1,
    ))
    .num_days() as u32
}

pub fn days_of_year(year: i32) -> u32 {
    NaiveDate::from_ymd(year, 1, 1)
        .signed_duration_since(NaiveDate::from_ymd(year + 1, 1, 1))
        .num_days() as u32
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeSpan<Tz: TimeZone> {
    Allday(Date<Tz>, Option<Date<Tz>>),
    TimePoints(DateTime<Tz>, DateTime<Tz>),
    Duration(DateTime<Tz>, Duration),
    Instant(DateTime<Tz>),
}

impl<Tz: TimeZone> TimeSpan<Tz> {
    pub fn from_start_and_end(begin: DateTime<Tz>, end: DateTime<Tz>) -> Self {
        TimeSpan::TimePoints(begin, end)
    }

    pub fn from_start_and_duration(begin: DateTime<Tz>, end: Duration) -> Self {
        TimeSpan::Duration(begin, end)
    }

    pub fn from_start(begin: DateTime<Tz>) -> Self {
        TimeSpan::Instant(begin)
    }

    pub fn allday(date: Date<Tz>) -> Self {
        TimeSpan::Allday(date, None)
    }

    pub fn allday_until(begin: Date<Tz>, end: Date<Tz>) -> Self {
        TimeSpan::Allday(begin, Some(end))
    }

    pub fn is_allday(&self) -> bool {
        matches!(self, TimeSpan::Allday(_, _))
    }

    pub fn is_instant(&self) -> bool {
        matches!(self, TimeSpan::Instant(_))
    }

    pub fn begin(&self) -> DateTime<Tz> {
        match &self {
            TimeSpan::Allday(begin, _) => begin.and_hms(0, 0, 0),
            TimeSpan::TimePoints(begin, _) => begin.clone(),
            TimeSpan::Duration(begin, _) => begin.clone(),
            TimeSpan::Instant(begin) => begin.clone(),
        }
    }

    pub fn end(&self) -> DateTime<Tz> {
        match &self {
            TimeSpan::Allday(begin, end) => end.as_ref().unwrap_or(&begin).and_hms(23, 59, 59),
            TimeSpan::TimePoints(_, end) => end.clone(),
            TimeSpan::Duration(begin, dur) => begin.clone() + dur.clone(),
            TimeSpan::Instant(end) => end.clone(),
        }
    }

    pub fn duration(&self) -> Duration {
        match &self {
            TimeSpan::Allday(begin, end) => end
                .as_ref()
                .map_or(Duration::hours(24), |e| e.clone() - begin.clone()),
            TimeSpan::TimePoints(start, end) => end.clone() - start.clone(),
            TimeSpan::Duration(_, dur) => dur.clone(),
            TimeSpan::Instant(_) => chrono::Duration::seconds(0),
        }
    }

    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> TimeSpan<Tz2> {
        match self {
            TimeSpan::Allday(begin, end) => {
                TimeSpan::<Tz2>::Allday(begin.with_timezone(tz), end.map(|e| e.with_timezone(tz)))
            }
            TimeSpan::TimePoints(begin, end) => {
                TimeSpan::<Tz2>::TimePoints(begin.with_timezone(tz), end.with_timezone(tz))
            }
            TimeSpan::Duration(begin, dur) => {
                TimeSpan::<Tz2>::Duration(begin.with_timezone(tz), dur)
            }
            TimeSpan::Instant(begin) => TimeSpan::<Tz2>::Instant(begin.with_timezone(tz)),
        }
    }
}

impl<Tz: TimeZone> From<TimeSpan<Tz>> for Duration {
    fn from(timespan: TimeSpan<Tz>) -> Self {
        timespan.duration()
    }
}

#[derive(Clone)]
pub enum Occurrence<Tz: TimeZone> {
    Onetime(TimeSpan<Tz>),
    Recurring(TimeSpan<Tz>, RRuleSet),
}

impl<Tz: TimeZone> Occurrence<Tz> {
    pub fn is_allday(&self) -> bool {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.is_allday(),
            Recurring(ts, _) => ts.is_allday(),
        }
    }

    pub fn is_onetime(&self) -> bool {
        use Occurrence::*;
        matches!(self, Onetime(_))
    }

    pub fn is_recurring(&self) -> bool {
        use Occurrence::*;
        matches!(self, Recurring(_, _))
    }

    pub fn as_date(&self) -> NaiveDate {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.begin().date_naive(),
            Recurring(ts, _) => ts.begin().date_naive(),
        }
    }

    pub fn as_datetime(&self) -> DateTime<Tz> {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.begin(),
            Recurring(ts, _) => ts.begin(),
        }
    }

    pub fn begin(&self) -> DateTime<Tz> {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.begin(),
            Recurring(ts, _) => ts.begin(),
        }
    }

    pub fn end(&self) -> DateTime<Tz> {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.end(),
            Recurring(ts, _) => ts.end(),
        }
    }

    pub fn duration(&self) -> Duration {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.duration(),
            Recurring(ts, _) => ts.duration(),
        }
    }

    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> Occurrence<Tz2> {
        use Occurrence::*;
        match self {
            Onetime(ts) => Occurrence::<Tz2>::Onetime(ts.with_tz(tz)),
            Recurring(ts, rrule) => Occurrence::<Tz2>::Recurring(ts.with_tz(tz), rrule),
        }
    }

    pub fn recurring(self, rule: RRuleSet) -> Self {
        use Occurrence::*;
        match self {
            Onetime(ts) => Occurrence::Recurring(ts, rule),
            Recurring(ts, _) => Occurrence::Recurring(ts, rule),
        }
    }

    pub fn timezone(&self) -> Tz {
        use Occurrence::*;
        match self {
            Onetime(ts) => ts.begin().timezone(),
            Recurring(ts, _) => ts.begin().timezone(),
        }
    }

    pub fn iter<'a>(&'a self) -> OccurrenceIter<'a, Tz> {
        use Occurrence::*;
        match self {
            Onetime(ts) => OccurrenceIter {
                start: Some(ts.begin()),
                rrule_iter: None,
                tz: self.timezone(),
            },
            Recurring(ts, rrule) => OccurrenceIter {
                start: None,
                rrule_iter: Some(rrule.into_iter()),
                tz: self.timezone(),
            },
        }
    }
}

pub struct OccurrenceIter<'a, Tz: TimeZone> {
    start: Option<DateTime<Tz>>,
    rrule_iter: Option<RRuleSetIter<'a>>,
    tz: Tz,
}

impl<Tz: TimeZone> Iterator for OccurrenceIter<'_, Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(it) = &mut self.rrule_iter {
            it.next().map(|dt| dt.with_timezone(&self.tz))
        } else {
            self.start.take()
        }
    }
}

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
    pub fn from_datetime(mut self, date: Bound<NaiveDateTime>) -> Self {
        self.begin = date;
        self
    }

    pub fn to_datetime(mut self, date: Bound<NaiveDateTime>) -> Self {
        self.end = date;
        self
    }

    pub fn datetime_range<R: RangeBounds<NaiveDateTime>>(mut self, range: R) -> Self {
        self.begin = range.start_bound().cloned();
        self.end = range.end_bound().cloned();

        self
    }
}

pub trait Eventlike {
    fn title(&self) -> &str;
    fn set_title(&mut self, title: &str);
    fn uuid(&self) -> Uuid;
    fn summary(&self) -> &str;
    fn set_summary(&mut self, summary: &str);
    fn occurrence(&self) -> &Occurrence<Tz>;
    fn set_occurrence(&mut self, occurrence: Occurrence<Tz>);
    fn tz(&self) -> &Tz;
    fn set_tz(&mut self, tz: &Tz);
    fn begin(&self) -> DateTime<Tz>;
    fn end(&self) -> DateTime<Tz>;
    fn duration(&self) -> Duration;
}

pub trait Calendarlike {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn tz(&self) -> &Tz;
    fn set_tz(&mut self, tz: &Tz);
    fn event_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &(dyn Eventlike + 'a)> + 'a>;
    fn filter_events<'a>(
        &'a self,
        filter: EventFilter,
    ) -> Box<dyn Iterator<Item = &(dyn Eventlike + 'a)> + 'a>;
    fn new_event(&mut self);
}

pub trait Collectionlike {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn calendar_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &(dyn Calendarlike + 'a)> + 'a>;
    fn event_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &(dyn Eventlike + 'a)> + 'a>;
    fn new_calendar(&mut self);
}

pub fn load_collection(provider: &str, path: &Path) -> Result<impl Collectionlike> {
    match provider {
        "ical" => ical::Collection::from_dir(path),
        _ => Err(Error::new(ErrorKind::CalendarParse, "No collection found")),
    }
}

pub fn load_collection_with_calendars(
    provider: &str,
    path: &Path,
    calendar_specs: &[CalendarSpec],
) -> Result<impl Collectionlike> {
    match provider {
        "ical" => ical::Collection::calendars_from_dir(path, calendar_specs),
        _ => Err(Error::new(ErrorKind::CalendarParse, "No collection found")),
    }
}
