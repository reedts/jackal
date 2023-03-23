use chrono::{
    DateTime, Datelike, Duration, Month, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday,
};
use rrule::RRule;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ical::parser::ical::component::{IcalAlarm, IcalCalendar, IcalEvent, IcalTimeZone};
use ical::parser::ical::IcalParser;
use ical::parser::Component;
use ical::property::Property;

use super::datetime::*;
use super::{PropertyList, ISO8601_2004_LOCAL_FORMAT, ISO8601_2004_UTC_FORMAT};

use crate::provider::tz::*;
use crate::provider::{
    days_of_month, AlarmGenerator, AlarmTrigger, Error, ErrorKind, Eventlike, OccurrenceRule,
    Result, TimeSpan, Uid,
};

struct IcalAlarmGenerator {
    trigger: AlarmTrigger,
    repeat: Option<u32>,
    wait: Option<Duration>,
    description: Option<String>,
}

impl IcalAlarmGenerator {
    // There are EXACTLY THREE values for the ACTION property.
    // Anything else will be ignored by jackal to avoid further problems
    // with these VALARM items.
    const VALID_ACTION_VALUES: [&str; 3] = ["DISPLAY", "AUDIO", "EMAIL"];

    pub fn finish(self, event: Uid) -> AlarmGenerator {
        AlarmGenerator::new(
            self.trigger,
            self.repeat,
            self.wait,
            self.description,
            event,
        )
    }
}

impl TryFrom<&IcalAlarm> for IcalAlarmGenerator {
    type Error = Error;
    fn try_from(value: &IcalAlarm) -> std::result::Result<Self, Self::Error> {
        // Check if specified VALARM component is compatible.
        if value
            .get_property("ACTION")
            .filter(|p| Self::VALID_ACTION_VALUES.contains(&p.value.as_deref().unwrap_or_default()))
            .is_none()
        {
            return Err(Error::new(
                ErrorKind::EventParse,
                "VALARM has invalid ACTION value",
            ));
        }

        let trigger = if let Some(t) = value.get_property("TRIGGER") {
            // Check whether trigger value is a datetime
            let datetime = t.params.as_ref().and_then(|p| {
                p.iter().find(|(name, values)| {
                    name == "VALUE" && values.first().unwrap() == "DATE-TIME"
                })
            });

            if datetime.is_some() {
                let naivedt = NaiveDateTime::parse_from_str(
                    t.value.as_deref().unwrap(),
                    ISO8601_2004_UTC_FORMAT,
                )
                .map_err(|_| {
                    Error::new(
                        ErrorKind::TimeParse,
                        "Datetime in 'TRIGGER' value must be in UTC",
                    )
                })?;
                AlarmTrigger::Absolute(Utc.from_utc_datetime(&naivedt))
            } else {
                // Check whether 'RELATED' is defined
                let related = t.params.as_ref().and_then(|p| {
                    p.iter().find_map(|(name, values)| {
                        if name == "RELATED" {
                            values.first().map(String::as_str)
                        } else {
                            None
                        }
                    })
                });

                match related.unwrap_or("START") {
                    "START" => AlarmTrigger::Start(
                        t.value.as_ref().unwrap().parse::<IcalDuration>()?.into(),
                    ),
                    "END" => {
                        AlarmTrigger::End(t.value.as_ref().unwrap().parse::<IcalDuration>()?.into())
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::EventParse,
                            "Invalid value for RELATED in VALARM component",
                        ))
                    }
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::EventParse,
                "No TRIGGER specified in VALARM component",
            ));
        };

        let repeat = value
            .get_property("REPEAT")
            .and_then(|v| v.value.as_ref().unwrap().parse::<u32>().ok());
        let wait: Option<Duration> = value.get_property("DURATION").and_then(|v| {
            v.value
                .as_ref()
                .unwrap()
                .parse::<IcalDuration>()
                .ok()
                .map(<IcalDuration as Into<Duration>>::into)
        });

        // If ACTION is DISPLAY or EMAIL, RFC5545 states that DESCRIPTION must be present.
        // However, as jackal also should get along with ACTION=AUDIO which DOES NOT require
        // DESCRIPTION to be set (and seems to be the default for Mac-Calendar... ugh...) we
        // do not enforce this here.
        let description = value
            .get_property("DESCRIPTION")
            .and_then(|v| v.value.clone());

        if repeat.is_some() && wait.is_none() {
            Err(Error::new(
                ErrorKind::ParseError,
                "REPEAT and DURATION must both be specified",
            ))
        } else {
            Ok(IcalAlarmGenerator {
                trigger,
                repeat,
                wait,
                description: description.to_owned(),
            })
        }
    }
}

#[derive(Clone)]
pub struct Event {
    path: PathBuf,
    occurrence: OccurrenceRule<Tz>,
    alarms: Vec<AlarmGenerator>,
    ical: IcalCalendar,
    tz: Tz,
}

pub fn uid_from_path(path: &Path) -> Option<String> {
    Some(path.file_stem().unwrap().to_str()?.to_owned())
}

impl Event {
    pub fn new(path: &Path, occurrence: OccurrenceRule<Tz>) -> Result<Self> {
        let uid = uid_from_path(path).ok_or_else(|| {
            Error::new(
                ErrorKind::EventParse,
                &format!(
                    "Uid derived from path '{}' is not an utf8 string",
                    path.display()
                ),
            )
        })?;

        let mut ical_calendar = IcalCalendar::new();
        ical_calendar.properties = vec![
            Property {
                name: "PRODID".to_owned(),
                params: None,
                value: Some(super::JACKAL_PRODID.to_owned()),
            },
            Property {
                name: "VERSION".to_owned(),
                params: None,
                value: Some(super::JACKAL_CALENDAR_VERSION.to_owned()),
            },
        ];

        ical_calendar
            .timezones
            .push((&occurrence.timezone()).into());

        let mut ical_event = IcalEvent::new();
        ical_event.properties = vec![
            Property {
                name: "UID".to_owned(),
                params: None,
                value: Some(uid),
            },
            Property {
                name: "DTSTAMP".to_owned(),
                params: None,
                value: Some(generate_timestamp()),
            },
        ];

        match &occurrence {
            OccurrenceRule::Onetime(ts) => {
                ical_event
                    .properties
                    .append(&mut IcalTimeSpan(ts.clone()).into());
            }
            OccurrenceRule::Recurring(ts, rrule) => {
                ical_event
                    .properties
                    .append(&mut IcalTimeSpan(ts.clone()).into());
                ical_event.properties.push(Property {
                    name: "RRULE".to_owned(),
                    params: None,
                    value: Some(rrule.to_string()),
                });
            }
        }

        ical_calendar.events.push(ical_event);

        let tz = occurrence.timezone();

        assert!(
            path.is_file(),
            "File property assured at beginning of function."
        );
        Ok(Event {
            path: path.to_owned(),
            occurrence,
            ical: ical_calendar,
            alarms: Vec::new(),
            tz,
        })
    }

    pub fn new_with_ical_properties(
        path: &Path,
        occurrence: OccurrenceRule<Tz>,
        properties: PropertyList,
    ) -> Result<Self> {
        let mut event = Self::new(path, occurrence)?;

        let new_properties: Vec<_> = properties
            .into_iter()
            .filter(|p| {
                event
                    .ical
                    .properties
                    .iter()
                    .find(|v| v.name == p.name)
                    .is_none()
            })
            .collect();

        event.ical.events[0].properties.extend(new_properties);

        Ok(event)
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let buf = io::BufReader::new(fs::File::open(path)?);

        let mut reader = IcalParser::new(buf);

        let ical: IcalCalendar = match reader.next() {
            Some(cal) => match cal {
                Ok(c) => c,
                Err(e) => {
                    return Err(Error::from(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "No calendar could be read from '{p}': {e}",
                            p = path.display(),
                            e = e
                        ),
                    )))
                }
            },
            None => {
                return Err(Error::from(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("No calendar found in '{}'", path.display()),
                )))
            }
        };

        Self::from_ical(path, ical)
    }

    pub fn from_ical(path: &Path, ical: IcalCalendar) -> Result<Self> {
        if ical.events.len() > 1 {
            return Err(Error::from(ErrorKind::CalendarParse).with_msg(&format!(
                "Calendar '{}' has more than one event entry",
                path.display()
            )));
        }

        if ical.events.is_empty() {
            return Err(Error::from(ErrorKind::CalendarParse)
                .with_msg(&format!("Calendar '{}' has no event entry", path.display())));
        }

        let event = ical.events.first().unwrap();

        // TODO: Handle multiple VTIMEZONE definitions
        let tz = ical
            .timezones
            .first()
            .and_then(|tz| Tz::try_from(tz).ok())
            .unwrap_or(Tz::utc());

        let dtstart = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(
                ErrorKind::EventMissingKey,
                &format!("'{}': No DTSTART found", path.display()),
            ))?;

        let dtend = event.properties.iter().find(|p| p.name == "DTEND");
        // Check if DURATION is set
        let duration = event.properties.iter().find(|p| p.name == "DURATION");

        // Required (if METHOD not set)
        let dtstart_spec = IcalDateTime::from_property(dtstart, Some(&tz)).map_err(|e| {
            let msg = e.to_string();
            e.with_msg(&format!("'{}': {}", path.display(), msg))
        })?;

        // DTEND does not HAVE to be specified...
        let mut occurrence = if let Some(dt) = dtend {
            // ...but if set it must be parseable
            let dtend_spec = IcalDateTime::from_property(dt, Some(&tz)).map_err(|e| {
                let msg = e.to_string();
                e.with_msg(&format!("'{}': {}", path.display(), msg))
            })?;

            match &dtend_spec {
                IcalDateTime::Date(date) => {
                    if let IcalDateTime::Date(bdate) = dtstart_spec {
                        OccurrenceRule::Onetime(TimeSpan::allday_until(bdate, *date, tz.clone()))
                    } else {
                        return Err(Error::new(
                            ErrorKind::DateParse,
                            &format!(
                                "'{}': DTEND must also be of type 'DATE' if DTSTART is",
                                path.display()
                            ),
                        ));
                    }
                }
                dt @ _ => OccurrenceRule::Onetime(TimeSpan::from_start_and_end(
                    dtstart_spec.as_datetime(&tz),
                    dt.as_datetime(&tz),
                )),
            }
        } else if let Some(duration) = duration {
            let dur_spec = IcalDuration::try_from(duration).map_err(|e| {
                let msg = e.to_string();
                e.with_msg(&format!("'{}': {}", path.display(), msg))
            })?;
            OccurrenceRule::Onetime(TimeSpan::from_start_and_duration(
                dtstart_spec.as_datetime(&tz),
                dur_spec.into(),
            ))
        } else {
            // If neither DTEND, nor DURATION is specified event duration depends solely
            // on DTSTART. RFC 5545 states, that if DTSTART is...
            //  ... a date spec, the event has to have the duration of a single day
            //  ... a datetime spec, the event has to have the dtstart also as dtend
            match dtstart_spec {
                IcalDateTime::Date(d) => OccurrenceRule::Onetime(TimeSpan::allday(d, tz.clone())),
                dt => OccurrenceRule::Onetime(TimeSpan::from_start(dt.as_datetime(&tz))),
            }
        };

        let ical_rrule = event.properties.iter().find(|p| p.name == "RRULE");

        if let Some(rule) = ical_rrule {
            if let Ok(ruleset) = rule
                .value
                .as_ref()
                .unwrap()
                .parse::<RRule<rrule::Unvalidated>>()
            {
                let start = occurrence.first().begin();
                let rrule_tz: rrule::Tz = occurrence.timezone().try_into().unwrap_or_else(|_| {
                    log::warn!("RRULE in event '{}' uses a custom defined timezone which is currently not supported (see https://github.com/fmeringdal/rust-rrule/pull/85). Falling back
                        to local timezone. Datetime may be wrong!", path.display());
                    rrule::Tz::LOCAL
                });

                let mut rrule_set =
                    ruleset
                        .build(start.with_timezone(&rrule_tz))
                        .map_err(|err| {
                            Error::new(
                                ErrorKind::EventParse,
                                &format!("'{}': {}", path.display(), err),
                            )
                        })?;

                // collect and add RDATES
                let rdates: Vec<DateTime<rrule::Tz>> = event.properties.iter().filter(|p| p.name == "RDATE").map(|property| {
                    let ical_dt = IcalDateTime::from_property(property, Some(&tz))
                        .expect("RDATE has invalid datetime");
                    let rdate_tz: rrule::Tz = ical_dt.timezone().try_into().unwrap_or_else(|_| {
                    log::warn!("RRULE in event '{}' uses a custom defined timezone which is currently not supported\
                        (see https://github.com/fmeringdal/rust-rrule/pull/85).\
                        Falling back to local timezone. Datetime may be wrong!", path.display());
                    rrule::Tz::LOCAL
                    });
                    ical_dt.as_datetime(&rdate_tz)
                }).collect();
                rrule_set = rrule_set.set_rdates(rdates);

                // collect and add EXDATES
                let exdates: Vec<DateTime<rrule::Tz>> = event.properties.iter().filter(|p| p.name == "EXDATE").map(|property| {
                    let ical_dt = IcalDateTime::from_property(property, Some(&tz))
                        .expect("RDATE has invalid datetime");
                    let rdate_tz: rrule::Tz = ical_dt.timezone().try_into().unwrap_or_else(|_| {
                    log::warn!("RRULE in event '{}' uses a custom defined timezone which is currently not supported\
                        (see https://github.com/fmeringdal/rust-rrule/pull/85).\
                        Falling back to local timezone. Datetime may be wrong!", path.display());
                    rrule::Tz::LOCAL
                    });
                    ical_dt.as_datetime(&rdate_tz)
                }).collect();
                rrule_set = rrule_set.set_exdates(exdates);

                occurrence = occurrence.with_recurring(rrule_set);
            }
        }

        let alarms: Vec<AlarmGenerator> = event
            .alarms
            .iter()
            .map(|a| IcalAlarmGenerator::try_from(a))
            .inspect(|r| {
                if let Err(e) = r {
                    log::error!("{}: {}", path.display(), e);
                }
            })
            .filter_map(|a| {
                a.ok().map(|outer| {
                    outer.finish(
                        event
                            .get_property("UID")
                            .map(|prop| prop.value.as_ref().unwrap().to_owned())
                            .unwrap(),
                    )
                })
            })
            .collect();

        // TODO: Check for exdate

        // TODO: VTIMEZONE

        Ok(Event {
            path: path.into(),
            occurrence,
            alarms,
            ical,
            tz,
        })
    }

    fn get_property_value(&self, name: &str) -> Option<&str> {
        if let Some(prop) = self.ical.events[0]
            .properties
            .iter()
            .find(|prop| prop.name == name)
        {
            prop.value.as_deref()
        } else {
            None
        }
    }

    fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.ical.events[0]
            .properties
            .iter_mut()
            .find(|prop| prop.name == name)
    }

    pub fn set_summary(&mut self, summary: &str) {
        self.set_title(summary)
    }

    pub fn set_title(&mut self, title: &str) {
        if let Some(property) = self.get_property_mut("SUMMARY") {
            property.value = Some(title.to_owned());
        } else {
            self.ical.events[0].add_property(Property {
                name: "SUMMARY".to_owned(),
                params: None,
                value: Some(title.to_owned()),
            });
        };
    }

    pub fn set_description(&mut self, desc: &str) {
        if let Some(property) = self.get_property_mut("DESCRIPTION") {
            property.value = Some(desc.to_owned());
        } else {
            self.ical.events[0].add_property(Property {
                name: "DESCRIPTION".to_owned(),
                params: None,
                value: Some(desc.to_owned()),
            });
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn as_ical(&self) -> &IcalCalendar {
        &self.ical
    }

    pub fn ical_event(&self) -> &IcalEvent {
        &self.ical.events[0]
    }

    pub(super) fn move_to_dir(mut self, dir: &Path) -> Self {
        assert!(dir.is_dir(), "Provided path must point to a directory");
        self.path = dir.join(self.path.file_name().unwrap());
        self
    }

    // Note: This is really a "best effort" approach here, since we 1. cannot really assume that
    // paths contain the uuid and 2. cannot canonicalize, e.g., the path of a deleted file...
    // We assume here, however, that both paths have been canonicalized.
    pub fn matches(&self, path: &Path) -> bool {
        self.path == path
    }
}

impl Eventlike for Event {
    fn title(&self) -> &str {
        self.get_property_value("SUMMARY").unwrap()
    }

    fn uid(&self) -> &str {
        self.get_property_value("UID").unwrap()
    }

    fn summary(&self) -> &str {
        self.title()
    }

    fn description(&self) -> Option<&str> {
        self.get_property_value("DESCRIPTION")
    }

    fn occurrence_rule(&self) -> &OccurrenceRule<Tz> {
        &self.occurrence
    }

    fn tz(&self) -> &Tz {
        &self.tz
    }

    fn duration(&self) -> Duration {
        self.occurrence.duration().into()
    }

    fn alarms(&self) -> Vec<&AlarmGenerator> {
        self.alarms.iter().collect()
    }
}

impl From<Event> for IcalEvent {
    fn from(event: Event) -> Self {
        event.ical.events[0].clone()
    }
}

impl From<Event> for IcalCalendar {
    fn from(event: Event) -> Self {
        event.ical
    }
}
