use chrono::{Date, DateTime, Duration, Local, NaiveTime, TimeZone};
use uuid::Uuid;
use std::path::Path;

mod ical;

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

    pub fn as_datetime<Tz2: TimeZone>(&self, tz: &Tz2) -> chrono::DateTime<Tz2> {
        use Occurrence::*;
        match self {
            Allday(date) => date
                .as_date(tz)
                .and_time(NaiveTime::from_hms(0, 0, 0))
                .unwrap(),
            Onetime(timespan) => timespan.begin().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }

    pub fn begin<Tz2: TimeZone>(&self, tz: &Tz2) -> chrono::DateTime<Tz2> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(0, 0, 0),
            Onetime(timespan) => timespan.begin().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }

    pub fn end<Tz2: TimeZone>(&self, tz: &Tz2) -> chrono::DateTime<Tz2> {
        use Occurrence::*;
        match self {
            Allday(date) => date.as_date(tz).and_hms(23, 59, 59),
            Onetime(timespan) => timespan.end().as_datetime(tz).clone(),
            Instant(datetime) => datetime.as_datetime(tz).clone(),
        }
    }
}

trait Eventlike<Tz: TimeZone = Local> {
    fn title(&self) -> &str;
    fn uuid(&self) -> &Uuid;
    fn summary(&self) -> &str;
    fn occurrence(&self) -> &Occurrence<Tz>;
    fn begin(&self) -> DateTime<Tz>;
    fn end(&self) -> DateTime<Tz>;
    fn duration(&self) -> Duration;
}

trait Calendarlike<Tz: TimeZone = Local> {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn events(&self) -> dyn Iterator<Item = &dyn Eventlike>;
    fn new_event(&mut self);
}
