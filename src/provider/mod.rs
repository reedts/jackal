use chrono::{
    Date, DateTime, Datelike, Duration, Local, Month, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Timelike,
};
use chrono_tz::Tz;
use genawaiter::{
    rc::{Co, Gen},
    Generator,
};
use nom::character::complete::{alpha1, char, i32};
use nom::combinator::all_consuming;
use nom::multi::separated_list1;
use nom::sequence::separated_pair;
use nom::{error as nerror, Err, IResult};
use num_traits::FromPrimitive;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::From;
use std::default::Default;
use std::iter::FromIterator;
use std::ops::{Bound, RangeBounds};
use std::path::Path;
use std::str::FromStr;
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

#[derive(Clone, Copy, Default, PartialOrd, Ord, PartialEq, Eq)]
pub enum Frequency {
    #[default]
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl FromStr for Frequency {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "secondly" => Ok(Frequency::Secondly),
            "minutely" => Ok(Frequency::Minutely),
            "hourly" => Ok(Frequency::Hourly),
            "daily" => Ok(Frequency::Daily),
            "weekly" => Ok(Frequency::Weekly),
            "monthly" => Ok(Frequency::Monthly),
            "yearly" => Ok(Frequency::Yearly),
            _ => Err(Error::new(
                ErrorKind::RecurRuleParse,
                &format!("Could not match '{}' to a recurrence frequency", s),
            )),
        }
    }
}

// impl FromStr for RecurFrequency {
//     type Err = Error;
//     fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
//         let (_, (freq, filters)): (&str, (&str, Vec<i32>)) = all_consuming(separated_pair(
//             alpha1,
//             char(':'),
//             separated_list1(char(','), i32),
//         ))(s)
//         .map_err(|_: nom::Err<nerror::Error<&str>>| {
//             Error::new(ErrorKind::RecurRuleParse, "Could not parse recurrence rule")
//         })?;

//         let frequency = freq.parse::<Frequency>()?;

//         Ok(RecurFrequency {
//             frequency,
//             filters: filters.into_iter().collect(),
//         })
//     }
// }

#[derive(Clone)]
pub enum RecurLimit<Tz: TimeZone = chrono_tz::Tz> {
    Count(u32),
    DateTime(DateTime<Tz>),
    Infinite,
}

impl<Tz: TimeZone> RecurLimit<Tz> {
    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> RecurLimit<Tz2> {
        match self {
            RecurLimit::Count(i) => RecurLimit::<Tz2>::Count(i),
            RecurLimit::DateTime(dt) => RecurLimit::<Tz2>::DateTime(dt.with_timezone(tz)),
            RecurLimit::Infinite => RecurLimit::<Tz2>::Infinite,
        }
    }
}

// Ordering of elements is adapted from ical
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum RecurFilter {
    ByMonth,
    ByWeekNo,
    ByYearDay,
    ByMonthDay,
    ByDay,
    ByHour,
    ByMinute,
    BySecond,
    BySetPos,
}

impl FromStr for RecurFilter {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bymonth" => Ok(RecurFilter::ByMonth),
            "byweekno" => Ok(RecurFilter::ByWeekNo),
            "byyearday" => Ok(RecurFilter::ByYearDay),
            "bymonthday" => Ok(RecurFilter::ByMonthDay),
            "byday" => Ok(RecurFilter::ByDay),
            "byhour" => Ok(RecurFilter::ByHour),
            "byminute" => Ok(RecurFilter::ByMinute),
            "bysecond" => Ok(RecurFilter::BySecond),
            "bysetpos" => Ok(RecurFilter::BySetPos),
            s @ _ => Err(Error::new(
                ErrorKind::RecurRuleParse,
                &format!("'{}' does not match a recurrence filter key", s),
            )),
        }
    }
}

#[derive(Clone, Ord, Eq)]
struct RecurFilterRule(BTreeMap<RecurFilter, i32>);

impl IntoIterator for RecurFilterRule {
    type Item = (RecurFilter, i32);
    type IntoIter = std::collections::btree_map::IntoIter<RecurFilter, i32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(RecurFilter, i32)> for RecurFilterRule {
    fn from_iter<T: IntoIterator<Item = (RecurFilter, i32)>>(iter: T) -> Self {
        RecurFilterRule(BTreeMap::from_iter(iter))
    }
}

impl PartialEq for RecurFilterRule {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .next()
            .unwrap()
            .eq(&other.0.iter().next().unwrap())
    }
}

impl PartialOrd for RecurFilterRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = if let Some(v) = self.0.iter().next() {
        } else {
            return None;
        };

        let rhs = if let Some(v) = self.0.iter().next() {
        } else {
            return None;
        };

        lhs.partial_cmp(&rhs)
    }
}

fn expand_rules(
    filters: &BTreeMap<RecurFilter, BTreeSet<i32>>,
    root: Option<RecurFilterRule>,
) -> BTreeSet<RecurFilterRule> {
    if filters.is_empty() {
        return BTreeSet::default();
    }

    let up_to_now = root.unwrap_or(RecurFilterRule(BTreeMap::from(
        [filters
            .iter()
            .rev()
            .next()
            .map(|(f, vs)| (*f, vs.iter().next().unwrap().clone()))
            .unwrap(); 1],
    )));
    // Is never empty
    let (last_freq, last_value) = up_to_now.0.iter().rev().next().unwrap();
    let rest = filters.range(..last_freq);

    let mut rules: BTreeSet<RecurFilterRule> = BTreeSet::default();

    if rest.clone().peekable().peek().is_none() {
        rules.insert(up_to_now);
    } else {
        for (freq, values) in rest.into_iter() {
            for value in values.range(last_value..) {
                let mut new_root = up_to_now.clone();
                new_root.0.insert(*freq, *value);
                rules.append(&mut expand_rules(&filters, Some(new_root)));
            }
        }
    }

    return rules;
}

#[derive(Clone)]
pub struct RecurRule<Tz: TimeZone = chrono_tz::Tz> {
    freq: Frequency,
    limit: RecurLimit<Tz>,
    interval: u32,
    filters: BTreeSet<RecurFilterRule>,
}

impl<Tz: TimeZone> RecurRule<Tz> {
    pub fn new(freq: Frequency) -> Self {
        RecurRule {
            freq,
            limit: RecurLimit::Infinite,
            interval: 1,
            filters: BTreeSet::default(),
        }
    }

    pub fn new_with_filters<It>(freq: Frequency, it: It) -> Self
    where
        It: IntoIterator<Item = (RecurFilter, Vec<i32>)>,
    {
        let map: BTreeMap<RecurFilter, BTreeSet<i32>> = it
            .into_iter()
            .map(|(f, v)| (f, BTreeSet::<i32>::from_iter(v)))
            .collect();

        RecurRule {
            freq,
            limit: RecurLimit::Infinite,
            interval: 1,
            filters: expand_rules(&map, None),
        }
    }

    pub fn with_interval(mut self, interval: u32) -> Self {
        self.set_interval(interval);
        self
    }

    pub fn with_limit(mut self, limit: RecurLimit<Tz>) -> Self {
        self.set_limit(limit);
        self
    }

    pub fn unlimited(mut self) -> Self {
        self.set_limit(RecurLimit::Infinite);
        self
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }

    pub fn set_interval_opt(&mut self, interval: Option<u32>) {
        if let Some(i) = interval {
            self.interval = i;
        }
    }

    pub fn set_limit(&mut self, limit: RecurLimit<Tz>) {
        self.limit = limit;
    }

    pub fn with_tz<Tz2: TimeZone>(self, tz: &Tz2) -> RecurRule<Tz2> {
        RecurRule {
            freq: self.freq,
            limit: self.limit.with_tz(tz),
            interval: self.interval,
            filters: self.filters,
        }
    }

    async fn occurrences_from_worker<'a>(&'a self, from: DateTime<Tz>, co: Co<DateTime<Tz>>) {
        let rules = if !self.filters.is_empty() {
            Some(&self.filters)
        } else {
            None
        };

        let mut count = 0u32;
        let mut current = from;

        // Each iterations goes through each filter rules (if present)
        // Otherwise, exactly ONE occurence is yielded
        loop {
            // Yield current as first occurrence
            co.yield_(current.clone()).await;

            // Perform range check
            match &self.limit {
                RecurLimit::Count(i) if &count >= i => return,
                RecurLimit::DateTime(d) if &current >= d => return,
                _ => (),
            }

            // Apply base frequency interval
            let base_duration = match self.freq {
                // Years can vary in days so we iterate `self.interval` many years to
                // get the correct days for each year
                Frequency::Yearly => Duration::days(
                    (current.year() as u32..=(current.year() as u32 + self.interval))
                        .map(|y| days_of_year(y as i32) as i64)
                        .sum(),
                ),
                // The same goes for months
                Frequency::Monthly => {
                    let mut days = 0u32;
                    let mut year = current.year();
                    let mut month = Month::from_u32(current.month()).unwrap();
                    for _ in 0..self.interval {
                        days += days_of_month(&month, year);
                        month = month.succ();

                        // Year overflow
                        if month == Month::January {
                            year += 1;
                        }
                    }

                    Duration::days(days as i64)
                }
                Frequency::Weekly => Duration::weeks(self.interval as i64),
                Frequency::Daily => Duration::days(self.interval as i64),
                Frequency::Hourly => Duration::hours(self.interval as i64),
                Frequency::Minutely => Duration::minutes(self.interval as i64),
                Frequency::Secondly => Duration::seconds(self.interval as i64),
            };

            // Check if filter rules exist
            if let Some(rules) = rules {
                for rule in rules.iter() {
                    let mut rule_current = current.clone();

                    for (filter, value) in rule.0.iter() {
                        match filter {
                            RecurFilter::ByMonth => {}
                            _ => (),
                        }
                    }
                }
            } else {
                // No filter rules, we can just apply
                co.yield_(current.clone()).await;
            }

            count += 1;
        }
    }

    pub fn occurrences_from<'a>(
        &'a self,
        from: &DateTime<Tz>,
    ) -> Gen<DateTime<Tz>, (), impl std::future::Future<Output = ()> + 'a> {
        Gen::new(|co| self.occurrences_from_worker(from.clone(), co))
    }
}

#[derive(Clone)]
pub enum Occurrence<Tz: TimeZone> {
    Onetime(TimeSpan<Tz>),
    Recurring(TimeSpan<Tz>, RecurRule<Tz>),
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
            Recurring(ts, rrule) => Occurrence::<Tz2>::Recurring(ts.with_tz(tz), rrule.with_tz(tz)),
        }
    }

    pub fn recurring(self, rule: RecurRule<Tz>) -> Self {
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
