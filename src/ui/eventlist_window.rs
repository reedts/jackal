use chrono::{DateTime, Local};
use std::fmt::{Display, Write};
use unsegen::base::*;
use unsegen::widget::*;

use crate::ical::{Event, OccurrenceSpec};
use crate::ui::Context;

enum Entry<'a> {
    Event(&'a Event),
    Time(DateTime<Local>),
    Cursor(DateTime<Local>),
}

impl Entry<'_> {
    pub fn datetime(&self) -> DateTime<Local> {
        match self {
            &Entry::Event(evt) => evt.occurrence().begin(&Local {}),
            &Entry::Cursor(dt) | &Entry::Time(dt) => dt,
        }
    }
}

impl Display for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Event(event) => {
                let time = match event.occurrence() {
                    OccurrenceSpec::Allday(_) => "Allday".to_owned(),
                    OccurrenceSpec::Onetime(start, end) => format!(
                        "{} - {}",
                        start.as_datetime(&Local {}).time().format("%H:%M"),
                        end.as_datetime(&Local {}).time().format("%H:%M")
                    ),
                };
                write!(f, "{}: {}", time, event.summary())
            }
            Self::Time(dt) => write!(f, " -> {}", dt.time().format("%H:%M")),
            Self::Cursor(dt) => write!(f, " -* {}", dt.time().format("%H:%M")),
        }
    }
}

pub struct EventWindow<'a> {
    context: &'a Context<'a>,
}

impl<'a> EventWindow<'a> {
    pub fn new(context: &'a Context<'a>) -> Self {
        EventWindow { context }
    }
}

impl Widget for EventWindow<'_> {
    fn space_demand(&self) -> Demand2D {
        Demand2D {
            width: ColDemand::at_least(10),
            height: RowDemand::at_least(10),
        }
    }

    fn draw(&self, mut window: unsegen::base::Window, hints: RenderingHints) {
        let mut events = self
            .context
            .agenda()
            .events_of_day(&self.context.cursor().date())
            .map(|ev| Entry::Event(ev))
            .chain([Entry::Cursor(self.context.cursor().clone())])
            .collect::<Vec<Entry>>();

        if self.context.today() == self.context.cursor().date() {
            events.push(Entry::Time(self.context.now().clone()))
        }

        events.sort_unstable_by_key(|entry| entry.datetime());

        let mut cursor = Cursor::new(&mut window);
        for ev in events {
            writeln!(&mut cursor, "{}", ev).unwrap();
        }
    }
}
