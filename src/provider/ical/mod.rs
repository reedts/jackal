pub mod calendar;
pub mod datetime;
pub mod event;
pub mod ser;

use ::ical::parser::Component;
pub use calendar::Calendar;
pub use event::Event;
use nom::bytes::complete::take;
use nom::character::complete::one_of;
use nom::combinator::{map_res, opt};
use nom::sequence::tuple;

use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use crate::config::CalendarConfig;
use crate::provider::ical::datetime::IcalDateTime;
use crate::provider::*;

use ::ical::parser::ical::component::{IcalTimeZone, IcalTimeZoneTransitionType};
use ::ical::property::Property;

type PropertyList = Vec<Property>;

const JACKAL_PRODID: &'static str = "-//JACKAL//NONSGML Calendar//EN";
const JACKAL_CALENDAR_VERSION: &'static str = "2.0";

const ISO8601_2004_LOCAL_FORMAT: &'static str = "%Y%m%dT%H%M%S";
const ISO8601_2004_UTC_FORMAT: &'static str = "%Y%m%dT%H%M%SZ";
const ISO8601_2004_LOCAL_FORMAT_DATE: &'static str = "%Y%m%d";

const ICAL_FILE_EXT: &'static str = "ics";

pub fn from_dir(
    path: &Path,
    config: &[CalendarConfig],
    tz_transition_cache: &'static TzTransitionCache,
    event_sink: &std::sync::mpsc::Sender<crate::events::Event>,
) -> Result<Vec<ProviderCalendar>> {
    if !path.is_dir() {
        return Err(Error::new(
            ErrorKind::CalendarParse,
            &format!("'{}' is not a directory", path.display()),
        ));
    }

    let calendars = config
        .iter()
        .map(|c| {
            calendar::from_dir(
                path.join(PathBuf::from(&c.id)).as_ref(),
                c,
                tz_transition_cache,
                event_sink,
            )
        })
        .inspect(|res| {
            if let Err(err) = res {
                log::error!("Could not load calendar: {}", err)
            }
        })
        .filter_map(Result::ok)
        .map(ProviderCalendar::Ical)
        .collect();

    Ok(calendars)
}
pub fn tz_from_ical(
    ical_tz: &IcalTimeZone,
    tz_transition_cache: &'static TzTransitionCache,
) -> Result<Tz> {
    fn parse_offset(s: &str) -> Result<i32> {
        let (_, (sign, hours, minutes, seconds)) = tuple((
            map_res(
                one_of::<_, _, (&str, nom::error::ErrorKind)>("+-"),
                |s: char| (s.to_string() + "1").parse::<i32>(),
            ),
            map_res(take(2usize), |s: &str| s.parse::<i32>()),
            map_res(take(2usize), |s: &str| s.parse::<i32>()),
            map_res(opt(take(2usize)), |s: Option<&str>| {
                s.unwrap_or("0").parse::<i32>()
            }),
        ))(s)?;

        Ok(sign * (hours * 3600 + minutes * 60 + seconds))
    }

    let id = ical_tz
        .get_property("TZID")
        .ok_or(Error::new(ErrorKind::TimezoneError, "Timezone has no id"))?
        .value
        .as_deref()
        .unwrap();

    // First try for "well-known" (IANA) TZID
    if let tz @ Ok(_) = id.parse::<Tz>() {
        return tz;
    }

    // If not well-known we build a Tz from custom transitions
    let mut transitions = Vec::<TransitionSet>::with_capacity(ical_tz.transitions.len());

    for ical_transition in ical_tz.transitions.iter() {
        let (utc_offset_secs, dst_offset_secs) = match ical_transition.transition {
            IcalTimeZoneTransitionType::STANDARD => (
                parse_offset(
                    ical_transition
                        .get_property("TZOFFSETTO")
                        .unwrap()
                        .value
                        .as_ref()
                        .unwrap(),
                )
                .expect("TZOFFSETTO not convertible"),
                0,
            ),
            IcalTimeZoneTransitionType::DAYLIGHT => {
                let utc_offset = parse_offset(
                    ical_transition
                        .get_property("TZOFFSETFROM")
                        .unwrap()
                        .value
                        .as_ref()
                        .unwrap(),
                )
                .expect("TZOFFSETFROM not convertible");
                (
                    utc_offset,
                    parse_offset(
                        ical_transition
                            .get_property("TZOFFSETTO")
                            .unwrap()
                            .value
                            .as_ref()
                            .unwrap(),
                    )
                    .expect("TZOFFSETTO not convertible")
                        - utc_offset,
                )
            }
        };

        let name = ical_transition
            .get_property("TZNAME")
            .and_then(|prop| prop.value.to_owned());

        // build RRULE for custom timezone
        // There is no need to provide a Tz here as DTSTART in VTIMEZONE
        // must contain a local (or 'floating') ical_tz
        let dtstart = IcalDateTime::from_property(
            ical_transition.get_property("DTSTART").ok_or(Error::new(
                ErrorKind::TimezoneError,
                &format!(
                    "Missing DTSTART for timezone '{}'",
                    name.as_deref().unwrap_or(&id)
                ),
            ))?,
            None,
        )?;

        let rule = if let Some(rrule_str) = ical_transition
            .get_property("RRULE")
            .and_then(|prop| prop.value.as_deref())
        {
            let rrule = rrule_str
                .parse::<RRule<rrule::Unvalidated>>()
                .expect("Could not parse RRULE of timezone");
            let rrule_set = rrule.build(dtstart.as_datetime(&rrule::Tz::LOCAL))?;
            let transitions = tz_transition_cache.lookup(&rrule_set);
            TransitionRule::Recurring(rrule_set, transitions)
        } else {
            TransitionRule::Single(dtstart.as_naive_local())
        };

        transitions.push(TransitionSet {
            utc_offset_secs,
            dst_offset_secs,
            id: id.to_string(),
            name,
            rule,
        });
    }

    Ok(Tz::from_iter(transitions))
}
