use crate::ical::Event;
use chrono::FixedOffset;
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView {
    style: Style,
    date: String,
    summary: String,
}

impl<'a> EventView {
    pub fn with(event: &Event<FixedOffset>) -> Self {
        EventView {
            style: Style::default(),
            date: if event.is_allday() {
                "Allday".to_owned()
            } else {
                event
                    .begin()
                    .inner_as_datetime()
                    .format("%H:%m")
                    .to_string()
            },
            summary: event.summary().to_owned(),
        }
    }
}

impl<'a> Into<Text<'a>> for EventView {
    fn into(self) -> Text<'a> {
        Text::from(vec![
            Spans::from(vec![Span::raw(self.date)]),
            Spans::from(vec![Span::raw("  "), Span::raw(self.summary)]),
        ])
    }
}
