use chrono::{Duration, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone};
use chrono_tz::{OffsetComponents, OffsetName};
use itertools::Itertools;
use rrule::RRuleSet;
use serde_with::DeserializeFromStr;
use std::convert::TryFrom;
use std::fmt::Display;
use std::iter::FromIterator;
use std::str::FromStr;
use std::vec::Vec;

use super::error::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionRule {
    Single(NaiveDateTime),
    Recurring(RRuleSet),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub utc_offset_secs: i32,
    pub dst_offset_secs: i32,
    pub id: String,
    pub name: Option<String>,
    pub rule: TransitionRule,
}

impl Transition {
    const MIN_FREQUENCY_DAYS: i64 = 365;

    pub fn latest_before(&self, before: &NaiveDateTime) -> Option<(NaiveDateTime, &Transition)> {
        use TransitionRule::*;

        match &self.rule {
            Single(dt) if dt <= before => Some((dt.clone(), self)),
            Recurring(rrule) => {
                let utc = rrule::Tz::UTC.from_utc_datetime(before);
                let relevant_transitions = rrule
                    .clone()
                    .after(utc - Duration::days(Self::MIN_FREQUENCY_DAYS))
                    .before(utc)
                    .all_unchecked();
                relevant_transitions
                    .into_iter()
                    .max()
                    .map(|dt| (dt.naive_utc(), self))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSet {
    pub transitions: Vec<Transition>,
}

impl TransitionSet {
    pub fn latest_before<'transitions>(
        &'transitions self,
        local: &NaiveDateTime,
    ) -> Vec<&'transitions Transition> {
        let latest_transitions: Vec<(NaiveDateTime, &Transition)> = self
            .transitions
            .iter()
            .filter_map(|transition| transition.latest_before(&local))
            .collect();

        latest_transitions
            .iter()
            .max_set_by_key(|(k, _)| k)
            .iter()
            .map(|(_, transition)| *transition)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TzOffset {
    tz: Tz,
    pub utc_offset_secs: i32,
    pub dst_offset_secs: i32,
    pub id: String,
    pub name: Option<String>,
}

impl Offset for TzOffset {
    fn fix(&self) -> FixedOffset {
        FixedOffset::east_opt(self.utc_offset_secs + self.dst_offset_secs)
            .expect("Seconds should be in range")
    }
}

impl Display for TzOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.as_deref().unwrap_or(self.id.as_str()))
    }
}

#[derive(Clone, Debug, Default, DeserializeFromStr, PartialEq)]
pub enum Tz {
    #[default]
    Local,
    Iana(chrono_tz::Tz),
    Custom {
        id: String,
        transitions: TransitionSet,
    },
}

impl Tz {
    const LOCAL_ID: &'static str = "Localtime";

    pub fn utc() -> Self {
        Self::Iana(chrono_tz::UTC)
    }

    pub fn id(&self) -> &str {
        match self {
            Tz::Local => Self::LOCAL_ID,
            Tz::Iana(tz) => tz.name(),
            Tz::Custom { id, transitions: _ } => id.as_str(),
        }
    }
}

impl TimeZone for Tz {
    type Offset = TzOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        offset.tz.clone()
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        use Tz::*;
        match self {
            Local => {
                let offset = chrono::Local.offset_from_local_date(local);

                offset.map(|offs| TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offs.local_minus_utc(),
                    dst_offset_secs: 0,
                    id: "Localtime".to_string(),
                    name: None,
                })
            }
            Iana(tz) => {
                let offset = tz.offset_from_local_date(local);

                offset.map(|offs| TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offs.base_utc_offset().num_seconds() as i32,
                    dst_offset_secs: offs.dst_offset().num_seconds() as i32,
                    id: offs.tz_id().to_owned(),
                    name: Some(offs.abbreviation().to_owned()),
                })
            }
            Custom { .. } => {
                let earliest =
                    self.offset_from_local_datetime(&local.and_hms_opt(0, 0, 0).unwrap());
                let latest =
                    self.offset_from_local_datetime(&local.and_hms_opt(23, 59, 59).unwrap());

                use LocalResult::*;
                match (earliest, latest) {
                    (result @ Single(_), _) => result,
                    (_, result @ Single(_)) => result,
                    (Ambiguous(offset, _), _) => Single(offset),
                    (_, Ambiguous(offset, _)) => Single(offset),
                    (None, None) => None,
                }
            }
        }
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        use Tz::*;
        match self {
            Local => {
                let offset = chrono::Local.offset_from_local_datetime(local);

                offset.map(|offs| TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offs.local_minus_utc(),
                    dst_offset_secs: 0,
                    id: "Localtime".to_string(),
                    name: None,
                })
            }
            Iana(tz) => {
                let offset = tz.offset_from_local_datetime(local);

                offset.map(|offs| TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offs.base_utc_offset().num_seconds() as i32,
                    dst_offset_secs: offs.dst_offset().num_seconds() as i32,
                    id: offs.tz_id().to_owned(),
                    name: Some(offs.abbreviation().to_owned()),
                })
            }
            Custom { id: _, transitions } => {
                let latest_transitions = transitions.latest_before(local);

                match latest_transitions.len() {
                    0 => LocalResult::None,
                    1 => {
                        let transition = latest_transitions.first().unwrap();
                        LocalResult::Single(TzOffset {
                            tz: self.clone(),
                            utc_offset_secs: transition.utc_offset_secs,
                            dst_offset_secs: transition.dst_offset_secs,
                            id: transition.id.clone(),
                            name: transition.name.clone(),
                        })
                    }
                    _ => {
                        let first = latest_transitions.get(0).unwrap();
                        let second = latest_transitions.get(1).unwrap();

                        LocalResult::Ambiguous(
                            TzOffset {
                                tz: self.clone(),
                                utc_offset_secs: first.utc_offset_secs,
                                dst_offset_secs: first.dst_offset_secs,
                                id: first.id.clone(),
                                name: first.name.clone(),
                            },
                            TzOffset {
                                tz: self.clone(),
                                utc_offset_secs: second.utc_offset_secs,
                                dst_offset_secs: second.dst_offset_secs,
                                id: second.id.clone(),
                                name: second.name.clone(),
                            },
                        )
                    }
                }
            }
        }
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        use Tz::*;
        match self {
            Local => {
                let offset = chrono::Local.offset_from_utc_date(utc);
                TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offset.local_minus_utc(),
                    dst_offset_secs: 0,
                    id: "Localtime".to_owned(),
                    name: None,
                }
            }
            Iana(tz) => {
                let offset = tz.offset_from_utc_date(utc);

                TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offset.base_utc_offset().num_seconds() as i32,
                    dst_offset_secs: offset.dst_offset().num_seconds() as i32,
                    id: offset.tz_id().to_owned(),
                    name: Some(offset.abbreviation().to_owned()),
                }
            }
            Custom { .. } => self.offset_from_utc_datetime(&utc.and_hms_opt(12, 0, 0).unwrap()),
        }
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        use Tz::*;
        match self {
            Local => {
                let offset = chrono::Local.offset_from_utc_datetime(utc);
                TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offset.local_minus_utc(),
                    dst_offset_secs: 0,
                    id: "Localtime".to_owned(),
                    name: None,
                }
            }
            Iana(tz) => {
                let offset = tz.offset_from_utc_datetime(utc);

                TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: offset.base_utc_offset().num_seconds() as i32,
                    dst_offset_secs: offset.dst_offset().num_seconds() as i32,
                    id: offset.tz_id().to_owned(),
                    name: Some(offset.abbreviation().to_owned()),
                }
            }
            Custom { id: _, transitions } => {
                let transition = transitions
                    .latest_before(utc)
                    .pop()
                    .expect("UTC datetime should fall in exactly ONE transition span");

                TzOffset {
                    tz: self.clone(),
                    utc_offset_secs: transition.utc_offset_secs,
                    dst_offset_secs: transition.dst_offset_secs,
                    id: transition.id.clone(),
                    name: transition.name.clone(),
                }
            }
        }
    }
}

impl FromIterator<Transition> for Tz {
    fn from_iter<T: IntoIterator<Item = Transition>>(iter: T) -> Self {
        let transitions = Vec::from_iter(iter);
        let id = transitions
            .first()
            .expect("Transition list must not be empty")
            .id
            .clone();

        Tz::Custom {
            id,
            transitions: TransitionSet { transitions },
        }
    }
}

impl FromStr for Tz {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase = s.to_lowercase();

        if matches!(lowercase.as_str(), "localtime" | "local") {
            Ok(Tz::Local)
        } else if let Ok(tz) = s.parse::<chrono_tz::Tz>() {
            Ok(Tz::Iana(tz))
        } else {
            // The only other known alternative is the `Tz::Custom` variant.
            // However for this to construct we require a definition of a set of
            // transitions. Otherwise the custom timezone has no purpose
            Err(Error::new(
                ErrorKind::ParseError,
                &format!("Timezone '{}' not recognized", s),
            ))
        }
    }
}

impl TryFrom<Tz> for rrule::Tz {
    type Error = Error;
    fn try_from(value: Tz) -> Result<Self, Self::Error> {
        match value {
            Tz::Local => Ok(rrule::Tz::LOCAL),
            Tz::Iana(tz) => Ok(rrule::Tz::Tz(tz)),
            Tz::Custom { .. } => Err(Error::new(
                ErrorKind::TimezoneError,
                "Custom timezone is not comaptible with RRULE",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iana_tz() {
        let dt = NaiveDate::from_ymd_opt(2020, 9, 8)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();
        let chronotz = "Europe/Berlin"
            .parse::<chrono_tz::Tz>()
            .expect("'Europe/Berlin' is a valid IANA timezone");

        let tz = Tz::Iana(chronotz.clone());

        assert_eq!(chronotz.from_utc_datetime(&dt), tz.from_utc_datetime(&dt));
    }
}
