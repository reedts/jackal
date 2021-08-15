use crate::ical::{Event, OccurrenceSpec};
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView {
    pub style: Style,
    pub event: Event,
    indent: u16,
}

impl EventView {
    pub fn with(event: &Event) -> Self {
        EventView {
            style: Style::default(),
            event: event.clone(),
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
        use OccurrenceSpec::*;
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
                Span::styled(self.event.summary(), self.style),
            ]),
            Spans::default(),
        ])
    }
}
