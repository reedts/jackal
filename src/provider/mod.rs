use chrono::{Date, DateTime, Duration, Local, Month, NaiveDate, NaiveTime, TimeZone};
use std::collections::BTreeMap;
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

pub fn days_of_month(month: &Month, year: i32) -> u64 {
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
    .num_days() as u64
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeSpan<Tz: TimeZone> {
    TimePoints(DateTime<Tz>, DateTime<Tz>),
    Duration(DateTime<Tz>, Duration),
}

impl<Tz: TimeZone> TimeSpan<Tz> {
    pub fn from_start_and_end(begin: DateTime<Tz>, end: DateTime<Tz>) -> Self {
        TimeSpan::TimePoints(begin, end)
    }

    pub fn from_start_and_duration(begin: DateTime<Tz>, end: Duration) -> Self {
        TimeSpan::Duration(begin, end)
    }

    pub fn begin(&self) -> DateTime<Tz> {
        match &self {
            TimeSpan::TimePoints(begin, _) => *begin,
            TimeSpan::Duration(begin, _) => *begin,
        }
    }

    pub fn end(&self) -> DateTime<Tz> {
        match &self {
            TimeSpan::TimePoints(_, end) => *end,
            TimeSpan::Duration(begin, dur) => *begin + *dur,
        }
    }

    pub fn duration(&self) -> Duration {
        match &self {
            TimeSpan::TimePoints(start, end) => *end - *start,
            TimeSpan::Duration(_, dur) => *dur,
        }
    }

    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> TimeSpan<Tz2> {
        match self {
            TimeSpan::TimePoints(begin, end) => {
                TimeSpan::<Tz2>::TimePoints(begin.with_timezone(tz), end.with_timezone(tz))
            }
            TimeSpan::Duration(begin, dur) => {
                TimeSpan::<Tz2>::Duration(begin.with_timezone(tz), dur)
            }
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
    Allday(Date<Tz>),
    Onetime(TimeSpan<Tz>),
    Instant(DateTime<Tz>),
}

impl<Tz: TimeZone> Occurrence<Tz> {
    pub fn is_allday(&self) -> bool {
        use Occurrence::*;
        matches!(self, Allday(_))
    }

    pub fn is_onetime(&self) -> bool {
        use Occurrence::*;
        matches!(self, Onetime(_))
    }

    pub fn as_date(&self) -> Date<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => *date,
            Onetime(timespan) => timespan.begin().date(),
            Instant(datetime) => datetime.date(),
        }
    }

    pub fn as_datetime(&self) -> DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.and_time(NaiveTime::from_hms(0, 0, 0)).unwrap(),
            Onetime(timespan) => timespan.begin(),
            Instant(datetime) => *datetime,
        }
    }

    pub fn begin(&self) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.and_hms(0, 0, 0),
            Onetime(timespan) => timespan.begin(),
            Instant(datetime) => *datetime,
        }
    }

    pub fn end(&self) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.and_hms(23, 59, 59),
            Onetime(timespan) => timespan.end(),
            Instant(datetime) => *datetime,
        }
    }

    pub fn duration(&self) -> Duration {
        use Occurrence::*;

        match self {
            Allday(_) => Duration::hours(24),
            Onetime(timespan) => timespan.duration(),
            Instant(_) => Duration::seconds(0),
        }
    }

    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> Occurrence<Tz2> {
        use Occurrence::*;
        match self {
            Allday(date) => Occurrence::<Tz2>::Allday(date.with_timezone(tz)),
            Onetime(timespan) => Occurrence::<Tz2>::Onetime(timespan.with_tz(tz)),
            Instant(dt) => Occurrence::<Tz2>::Instant(dt.with_timezone(tz)),
        }
    }

    pub fn timezone(&self) -> Tz {
        use Occurrence::*;
        match self {
            Allday(date) => date.timezone(),
            Onetime(timespan) => timespan.begin().timezone(),
            Instant(dt) => dt.timezone(),
        }
    }
}

struct EventFilter<'a, Tz: TimeZone> {
    inner: &'a BTreeMap<DateTime<Tz>, Vec<&'a dyn Eventlike<Tz>>>,
    begin: Bound<DateTime<Tz>>,
    end: Bound<DateTime<Tz>>,
}

impl<'a, Tz: TimeZone> EventFilter<'a, Tz> {
    pub fn from_datetime(self, date: Bound<DateTime<Tz>>) -> Self {
        self.begin = date;
        self
    }

    pub fn to_datetime(self, date: Bound<DateTime<Tz>>) -> Self {
        self.end = date;
        self
    }

    pub fn datetime_range<R: RangeBounds<DateTime<Tz>>>(self, range: R) -> Self {
        self.begin = range.start_bound().cloned();
        self.end = range.end_bound().cloned();

        self
    }

    pub fn apply(self) -> impl Iterator<Item = &'a dyn Eventlike<Tz>> {
        self.inner
            .range((self.begin, self.end))
            .into_iter()
            .flat_map(|(_, v)| v.iter().cloned())
    }
}

pub trait Eventlike<Tz: TimeZone = Local> {
    fn title(&self) -> &str;
    fn set_title(&mut self, title: &str);
    fn uuid(&self) -> Uuid;
    fn summary(&self) -> &str;
    fn set_summary(&mut self, summary: &str);
    fn occurrence(&self) -> &Occurrence<Tz>;
    fn set_occurrence(&mut self, occurrence: Occurrence<Tz>);
    fn begin(&self) -> DateTime<Tz>;
    fn end(&self) -> DateTime<Tz>;
    fn duration(&self) -> Duration;
}

pub trait Calendarlike<Tz: TimeZone = Local> {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn event_iter(&self) -> Box<dyn Iterator<Item = &dyn Eventlike<Tz>>>;
    fn new_event(&mut self);
}

pub trait Collectionlike<Tz: TimeZone = Local> {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn calendar_iter(&self) -> Box<dyn Iterator<Item = &dyn Calendarlike<Tz>>>;
    fn event_iter(&self) -> Box<dyn Iterator<Item = &dyn Eventlike<Tz>>>;
    fn new_calendar(&mut self);
}

pub fn load_collection<Tz: TimeZone>(
    provider: &str,
    path: &Path,
) -> Result<impl Collectionlike<Tz>> {
    match provider {
        "ical" => ical::Collection::<Tz>::from_dir(path),
    }
}

pub fn load_collection_with_calendars<Tz: TimeZone>(
    provider: &str,
    path: &Path,
    calendar_specs: &[CalendarSpec],
) -> Result<impl Collectionlike<Tz>> {
    match provider {
        "ical" => ical::Collection::<Tz>::calendars_from_dir(path, calendar_specs),
    }
}
