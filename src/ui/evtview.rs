use chrono::Utc;
use std::borrow::Cow;
use std::convert::Into;
use crate::ical::Event;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::text::Text;
use tui::widgets::{
    Block,
    Borders,
    Paragraph,
    Widget
};

pub struct EventView<'a> {
    style: Style,
    event: &'a Event<Utc>
}

impl<'a> EventView<'a> {
    pub fn with(event: &'a Event<Utc>) -> Self {
        EventView { style: Style::default(), event }
    }
}

impl<'a> Widget for EventView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Text::raw(self.event.summary()))
            .block(Block::default().borders(Borders::ALL))
            .render(area, buf);
    }
}

impl<'a> Into<Text<'a>> for EventView<'a> {
    fn into(self) -> Text<'a> {
        Text::styled(self.event.summary(), self.style)
    }
}
