use chrono::{DateTime, Local};
use std::fmt::{Display, Write};
use unsegen::base::*;
use unsegen::input::Scrollable;
use unsegen::widget::*;

use crate::provider::Eventlike;
use crate::ui::Context;

enum Entry<'a> {
    Event(DateTime<Local>, &'a dyn Eventlike),
    Time(DateTime<Local>),
    Cursor(DateTime<Local>),
}

impl Entry<'_> {
    pub fn datetime(&self) -> DateTime<Local> {
        match self {
            &Entry::Event(dt, _) | &Entry::Cursor(dt) | &Entry::Time(dt) => dt,
        }
    }
}

impl Display for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Event(_, event) => {
                let occur = event.occurrence_rule();

                let time = if occur.is_allday() {
                    "Allday".to_owned()
                } else {
                    format!(
                        "{} - {}",
                        occur.begin().time().format("%H:%M"),
                        occur.end().time().format("%H:%M")
                    )
                };
                write!(f, "{}: {}", time, event.summary())
            }
            Self::Time(dt) => f.pad(&format!("[{}]", dt.time().format("%H:%M"))),
            Self::Cursor(dt) => write!(f, " * {}", dt.time().format("%H:%M")),
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

    fn draw(&self, mut window: unsegen::base::Window, _hints: RenderingHints) {
        let mut events = self
            .context
            .agenda()
            .events_of_day(&self.context.cursor().date_naive())
            .map(|(dt, ev)| Entry::Event(dt.with_timezone(&Local), ev))
            .chain([Entry::Cursor(self.context.cursor().clone())])
            .collect::<Vec<Entry>>();

        // Append current time if cursor's date is today
        if self.context.today() == self.context.cursor().date() {
            events.push(Entry::Time(self.context.now().clone()))
        }

        events.sort_unstable_by_key(|entry| entry.datetime());

        let width = window.get_width().raw_value() as usize;

        let mut cursor = Cursor::new(&mut window);

        // Only count the real events (no cursor/clock)
        let mut idx: usize = 0;
        for ev in events {
            match ev {
                ev @ Entry::Event(..) => {
                    let saved_style = cursor.get_style_modifier();

                    if idx == self.context.eventlist_index {
                        cursor.apply_style_modifier(StyleModifier::new().invert(true));
                    }

                    if let Err(err) = write!(&mut cursor, "{}", ev) {
                        log::warn!("Error while writing event: {}", err);
                    }

                    cursor.fill_and_wrap_line();

                    cursor.set_style_modifier(saved_style);
                    idx += 1;
                }
                time @ Entry::Time(_) => {
                    let save_style = cursor.get_style_modifier();

                    cursor.apply_style_modifier(StyleModifier::new().fg_color(Color::LightRed));
                    writeln!(&mut cursor, "{:─^width$}", time).unwrap();
                    cursor.set_style_modifier(save_style);
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
