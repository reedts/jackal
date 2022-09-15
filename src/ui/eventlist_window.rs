use chrono::{DateTime, Local};
use std::fmt::{Display, Write};
use unsegen::base::*;
use unsegen::input::Scrollable;
use unsegen::widget::*;

use crate::provider::Occurrence;
use crate::provider::ical::calendar::Event;
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
                    Occurrence::Allday(_) => "Allday".to_owned(),
                    Occurrence::Onetime(timespan) => format!(
                        "{} - {}",
                        timespan
                            .begin()
                            .as_datetime(&Local {})
                            .time()
                            .format("%H:%M"),
                        timespan.end().as_datetime(&Local {}).time().format("%H:%M")
                    ),
                    Occurrence::Instant(dt) => {
                        format!("{}", dt.as_datetime(&Local {}).time().format("%H:%M"))
                    }
                };
                write!(f, "{}: {}", time, event.summary())
            }
            Self::Time(dt) => write!(f, " -> {}", dt.time().format("%H:%M")),
            Self::Cursor(dt) => write!(f, " -* {}", dt.time().format("%H:%M")),
        }
    }
}

pub struct EventWindow<'a> {
    context: &'a Context,
}

impl<'a> EventWindow<'a> {
    pub fn new(context: &'a Context) -> Self {
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

        // Append current time if cursor's date is today
        if self.context.today() == self.context.cursor().date() {
            events.push(Entry::Time(self.context.now().clone()))
        }

        events.sort_unstable_by_key(|entry| entry.datetime());

        let mut cursor = Cursor::new(&mut window);

        // Only count the real events (no cursor/clock)
        let mut idx: usize = 0;
        for ev in events {
            match ev {
                ev @ Entry::Event(_) => {
                    let saved_style = cursor.get_style_modifier();

                    if idx == self.context.eventlist_index {
                        cursor.apply_style_modifier(StyleModifier::new().invert(true));
                    }

                    write!(&mut cursor, "{}", ev).unwrap();
                    cursor.fill_and_wrap_line();

                    cursor.set_style_modifier(saved_style);
                    idx += 1;
                }
                entry => writeln!(&mut cursor, "{}", entry).unwrap(),
            }
        }
    }
}

pub struct EventWindowBehaviour<'a>(pub &'a mut Context, pub usize);

impl Scrollable for EventWindowBehaviour<'_> {
    fn scroll_backwards(&mut self) -> unsegen::input::OperationResult {
        if self.0.eventlist_index > 0 {
            self.0.eventlist_index -= 1;
            Ok(())
        } else {
            Err(())
        }
    }

    fn scroll_forwards(&mut self) -> unsegen::input::OperationResult {
        if self.0.eventlist_index + 1 < self.1 {
            self.0.eventlist_index += 1;
            Ok(())
        } else {
            Err(())
        }
    }
}
