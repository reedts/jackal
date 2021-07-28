use crate::ical::{Event, Occurrence};
use chrono::FixedOffset;
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView {
    pub style: Style,
    pub begin: Occurrence<FixedOffset>,
    pub end: Occurrence<FixedOffset>,
    pub summary: String,
    indent: u16,
}

impl<'a> EventView {
    pub fn with(event: &Event<FixedOffset>) -> Self {
        EventView {
            style: Style::default(),
            begin: event.begin().clone(),
            end: event.end().clone(),
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
                Span::styled(
                    match self.begin {
                        Allday(_) => "Allday".to_owned(),
                        _ => self.begin.inner_as_datetime().format("%H:%M").to_string(),
                    },
                    self.style,
                ),
                Span::styled(if self.begin.is_allday() { "" } else { " - " }, self.style),
                Span::styled(
                    match self.end {
                        Allday(_) => "".to_owned(),
                        _ => self.end.inner_as_datetime().format("%H:%M").to_string(),
                    },
                    self.style,
                ),
            ]),
            Spans::from(vec![
                Span::raw(" ".repeat(self.indent as usize)),
                Span::raw("  "),
                Span::styled(self.summary, self.style),
            ]),
            Spans::default(),
        ])
    }
}
