use std::fmt::{Display, Write};
use unsegen::base::*;
use unsegen::widget::*;

use crate::ical::Event;
use crate::ui::Context;

struct EventEntry<'a> {
    event: &'a Event,
}

impl<'a> EventEntry<'a> {
    pub fn new(event: &'a Event) -> Self {
        EventEntry { event }
    }
}

impl Display for EventEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "X - X: {}", self.event.summary())
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
        let events = self
            .context
            .agenda()
            .events_of_day(&self.context.cursor().date());

        let mut cursor = Cursor::new(&mut window);
        for ev in events {
            writeln!(&mut cursor, "{}", EventEntry::new(ev));
        }
    }
}
