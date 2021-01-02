use crate::ical::Event;
use chrono::FixedOffset;
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView<'a> {
    style: Style,
    event: &'a Event<FixedOffset>,
}

impl<'a> EventView<'a> {
    pub fn with(event: &'a Event<FixedOffset>) -> Self {
        EventView {
            style: Style::default(),
            event,
        }
    }
}

impl<'a> Into<Text<'a>> for EventView<'a> {
    fn into(self) -> Text<'a> {
        Text::from(vec![
            Spans::from(vec![Span::raw(
                self.event.begin().format("%Y-%m-%d").to_string(),
            )]),
            Spans::from(vec![Span::raw(self.event.summary())]),
        ])
    }
}
