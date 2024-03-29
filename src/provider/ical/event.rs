use chrono::{Datelike, Duration, Month, NaiveDate, Weekday};
use chrono_tz::{OffsetName, Tz};
use num_traits::FromPrimitive;
use rrule::RRule;
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tz;
use tz::timezone::*;

use ical::parser::ical::component::{
    IcalCalendar, IcalEvent, IcalTimeZone, IcalTimeZoneTransition, Transition as IcalTransition,
};
use ical::parser::ical::IcalParser;
use ical::parser::Component;
use ical::property::Property;

use super::datetime::*;
use super::{PropertyList, ISO8601_2004_LOCAL_FORMAT};

use crate::provider::{
    days_of_month, Error, ErrorKind, Eventlike, OccurrenceRule, Result, TimeSpan,
};

#[derive(Clone)]
pub struct Event {
    path: PathBuf,
    occurrence: OccurrenceRule<Tz>,
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

        if let Tz::UTC = occurrence.timezone() {
            ()
        } else {
            // push timezone information
            let mut tz_spec = IcalTimeZone::new();
            tz_spec.add_property(Property {
                name: "TZID".to_owned(),
                params: None,
                value: Some(occurrence.first().begin().offset().tz_id().to_string()),
            });

            tz_spec.add_property(Property {
                name: "TZNAME".to_owned(),
                params: None,
                value: Some(
                    occurrence
                        .first()
                        .begin()
                        .offset()
                        .abbreviation()
                        .to_string(),
                ),
            });

            let tz_info = tz::TimeZone::from_posix_tz(occurrence.first().begin().offset().tz_id())?;

            if let Some(rule) = tz_info.as_ref().extra_rule() {
                fn create_timezone_transitions(
                    transition: IcalTransition,
                    tz_name: String,
                    from_offset: i32,
                    to_offset: i32,
                    transition_day: &RuleDay,
                ) -> IcalTimeZoneTransition {
                    let mut tr = IcalTimeZoneTransition::new(transition);
                    tr.add_property(Property {
                        name: "TZNAME".to_string(),
                        params: None,
                        value: Some(tz_name),
                    });
                    tr.add_property(Property {
                        name: "TZOFFSETFROM".to_string(),
                        params: None,
                        value: Some(format!("{:+05}", from_offset)),
                    });
                    tr.add_property(Property {
                        name: "TZOFFSETTO".to_string(),
                        params: None,
                        value: Some(format!("{:+05}", to_offset)),
                    });
                    // FIXME: this does not conform to RFC5545 and should be fixed
                    // once we know how to correctly get DST start(/end)
                    //
                    // HERE BE DRAGONS!!!!
                    let dtstart = match transition_day {
                        RuleDay::MonthWeekDay(mwd) => NaiveDate::from_weekday_of_month_opt(
                            1970,
                            mwd.month().into(),
                            Weekday::from_u8(mwd.week_day()).unwrap(),
                            mwd.week(),
                        )
                        .unwrap(),
                        RuleDay::Julian0WithLeap(days) => {
                            NaiveDate::from_yo_opt(1970, (days.get() + 1) as u32).unwrap()
                        }
                        RuleDay::Julian1WithoutLeap(days) => {
                            NaiveDate::from_yo_opt(1970, days.get() as u32).unwrap()
                        }
                    }
                    .and_hms_opt(2, 0, 0)
                    .unwrap();

                    let num_days_of_month =
                        days_of_month(&Month::from_u32(dtstart.month()).unwrap(), dtstart.year());
                    let day_occurrences_before = dtstart.day() % 7;
                    let day_occurrences_after = (num_days_of_month - dtstart.day()) % 7;

                    let offset: i32 = if day_occurrences_after == 0 {
                        -1
                    } else {
                        day_occurrences_before as i32 + 1
                    };

                    tr.add_property(Property {
                        name: "DTSTART".to_string(),
                        params: None,
                        value: Some(dtstart.format(ISO8601_2004_LOCAL_FORMAT).to_string()),
                    });

                    // We generate this RRULE by hand for now
                    tr.add_property(Property {
                        name: "RRULE".to_string(),
                        params: None,
                        value: Some(format!(
                            "FREQ=YEARLY;BYMONTH={};BYDAY={:+1}{}",
                            dtstart.month(),
                            offset,
                            weekday_to_ical(dtstart.weekday())
                        )),
                    });

                    tr
                }

                match rule {
                    TransitionRule::Alternate(alt_time) => {
                        let std_offset_min = alt_time.std().ut_offset() * 60;
                        let dst_offset_min = alt_time.dst().ut_offset() * 60;
                        let dst_start_day = alt_time.dst_start();
                        let dst_end_day = alt_time.dst_end();

                        // Transition for standard to dst timezone
                        let std_to_dst = create_timezone_transitions(
                            IcalTransition::Standard,
                            alt_time.std().time_zone_designation().to_string(),
                            std_offset_min,
                            dst_offset_min,
                            dst_start_day,
                        );
                        tz_spec.transitions.push(std_to_dst);

                        // Transition for dst timezone back to standard
                        let dst_to_std = create_timezone_transitions(
                            IcalTransition::Daylight,
                            alt_time.dst().time_zone_designation().to_string(),
                            dst_offset_min,
                            std_offset_min,
                            dst_end_day,
                        );
                        tz_spec.transitions.push(dst_to_std);
                    }
                    _ => (),
                }
            }

            ical_calendar.timezones.push(tz_spec);
        }

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

        let dtstart = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .ok_or(Error::new(ErrorKind::EventMissingKey, "No DTSTART found"))?;

        let dtend = event.properties.iter().find(|p| p.name == "DTEND");
        // Check if DURATION is set
        let duration = event.properties.iter().find(|p| p.name == "DURATION");

        // Required (if METHOD not set)
        let dtstart_spec = IcalDateTime::try_from(dtstart)?;

        // Set TZ id based on start spec
        let tz = if let IcalDateTime::Local(dt) = dtstart_spec {
            dt.timezone()
        } else {
            chrono_tz::UTC
        };

        // DTEND does not HAVE to be specified...
        let mut occurrence = if let Some(dt) = dtend {
            // ...but if set it must be parseable
            let dtend_spec = IcalDateTime::try_from(dt)?;
            match &dtend_spec {
                IcalDateTime::Date(date) => {
                    if let IcalDateTime::Date(bdate) = dtstart_spec {
                        OccurrenceRule::Onetime(TimeSpan::allday_until(bdate, *date, tz))
                    } else {
                        return Err(Error::new(
                            ErrorKind::DateParse,
                            "DTEND must also be of type 'DATE' if DTSTART is",
                        ));
                    }
                }
                dt @ _ => OccurrenceRule::Onetime(TimeSpan::from_start_and_end(
                    dtstart_spec.as_datetime(&tz),
                    dt.as_datetime(&tz),
                )),
            }
        } else if let Some(duration) = duration {
            let dur_spec = IcalDuration::try_from(duration)?;
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
                IcalDateTime::Date(d) => OccurrenceRule::Onetime(TimeSpan::allday(d, tz)),
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
                let tz = occurrence.timezone();
                occurrence = occurrence
                    .with_recurring(ruleset.build(start.with_timezone(&rrule::Tz::Tz(tz)))?);
            }
        }

        // TODO: VTIMEZONE
        // TODO: Check for exdate

        Ok(Event {
            path: path.into(),
            occurrence,
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
