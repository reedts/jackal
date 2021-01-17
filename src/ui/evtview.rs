use crate::ical::{Event, Occurrence};
use chrono::FixedOffset;
use std::convert::Into;
use tui::style::Style;
use tui::text::{Span, Spans, Text};

pub struct EventView {
    pub style: Style,
    pub date: Occurrence<FixedOffset>,
    pub summary: String,
}

impl<'a> EventView {
    pub fn with(event: &Event<FixedOffset>) -> Self {
        EventView {
            style: Style::default(),
            date: event.begin().clone(),
            summary: event.summary().to_owned(),
        }
    }
}

impl<'a> Into<Text<'a>> for EventView {
    fn into(self) -> Text<'a> {
        use Occurrence::*;
        Text::from(vec![
            Spans::from(vec![Span::raw(match self.date {
                Allday(_) => "Allday".to_owned(),
                _ => self.date.inner_as_datetime().format("%H:%M").to_string(),
            })]),
            Spans::from(vec![Span::raw("  "), Span::raw(self.summary)]),
        ])
    }
}
