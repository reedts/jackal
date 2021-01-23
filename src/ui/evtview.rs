use crate::ical::{Event, Occurrence};
use chrono::FixedOffset;
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView {
    pub style: Style,
    pub date: Occurrence<FixedOffset>,
    pub summary: String,
    indent: u16,
}

impl<'a> EventView {
    pub fn with(event: &Event<FixedOffset>) -> Self {
        EventView {
            style: Style::default(),
            date: event.begin().clone(),
            summary: event.summary().to_owned(),
            indent: 0,
        }
    }

    pub fn indent(mut self, indent: u16) -> Self {
        self.indent = indent;
        self
    }
}

impl<'a> Into<Text<'a>> for EventView {
    fn into(self) -> Text<'a> {
        use Occurrence::*;
        Text::from(vec![
            Spans::from(vec![
                Span::raw(" ".repeat(self.indent as usize)),
                Span::raw(match self.date {
                    Allday(_) => "Allday".to_owned(),
                    _ => self.date.inner_as_datetime().format("%H:%M").to_string(),
                }),
            ]),
            Spans::from(vec![
                Span::raw(" ".repeat(self.indent as usize)),
                Span::raw("  "),
                Span::raw(self.summary),
            ]),
        ])
    }
}
