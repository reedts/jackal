use chrono::{Date, DateTime, Duration, Local, Month, NaiveDate, NaiveTime, TimeZone};
use std::convert::From;
use std::path::Path;
use uuid::Uuid;

pub mod error;
pub mod ical;

pub use error::*;

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
            TimeSpan::TimePoints(begin, _) => begin.clone(),
            TimeSpan::Duration(begin, _) => begin.clone(),
        }
    }

    pub fn end(&self) -> DateTime<Tz> {
        match &self {
            TimeSpan::TimePoints(_, end) => end.clone(),
            TimeSpan::Duration(begin, dur) => begin.clone().and_duration(dur.as_chrono_duration()),
        }
    }

    pub fn duration(&self) -> Duration {
        match &self {
            TimeSpan::TimePoints(start, end) => (end - start),
            TimeSpan::Duration(_, dur) => dur,
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
    Allday(DateTime<Tz>),
    Onetime(TimeSpan<Tz>),
    Instant(DateTime<Tz>),
}

impl<Tz: TimeZone> Default for Occurrence<Tz> {
    fn default() -> Self {
        Occurrence::Instant(DateTime::default())
    }
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

    pub fn as_date<Tz2: TimeZone>(&self, tz: &Tz2) -> Date<Tz2> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz),
            Onetime(timespan) => timespan.begin().as_date(tz),
            Instant(datetime) => datetime.as_date(tz),
        }
    }

    pub fn as_datetime(&self) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date
                .as_date()
                .and_time(NaiveTime::from_hms(0, 0, 0))
                .unwrap(),
            Onetime(timespan) => timespan.begin().as_datetime().clone(),
            Instant(datetime) => datetime.as_datetime().clone(),
        }
    }

    pub fn begin(&self) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date().and_hms(0, 0, 0),
            Onetime(timespan) => timespan.begin().as_datetime().clone(),
            Instant(datetime) => datetime.as_datetime().clone(),
        }
    }

    pub fn end(&self) -> chrono::DateTime<Tz> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date().and_hms(23, 59, 59),
            Onetime(timespan) => timespan.end().as_datetime().clone(),
            Instant(datetime) => datetime.as_datetime().clone(),
        }
    }

    pub fn duration(&self) -> Duration {
        use Occurrence::*;

        match self {
            Allday(_) => Duration::num_hours(24),
            Onetime(timespan) => timespan.into(),
            Instant(_) => Duration::num_seconds(0),
        }
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
    fn events_iter(&self) -> dyn Iterator<Item = &dyn Eventlike>;
    fn new_event(&mut self);
}

pub trait Collectionlike<Tz: TimeZone = Local> {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn calendar_iter(&self) -> dyn Iterator<Item = &dyn Calendarlike>;
    fn event_iter(&self) -> dyn Iterator<Item = &dyn Eventlike>;
    fn new_calendar(&mut self);
}

pub fn load_collection(provider: &str, path: &Path) -> Result<impl Collectionlike> {
    match provider {
        "ical" => ical::Collection::from_dir(path),
    }
}
