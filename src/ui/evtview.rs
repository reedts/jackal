use chrono::Utc;
use crate::ical::Event;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::widgets::{
    Block,
    Borders,
    Paragraph,
    Text,
    Widget
};

pub struct EventView<'a> {
    event: &'a Event<Utc>,
}

impl<'a> EventView<'a> {
    pub fn with(event: &'a Event<Utc>) -> Self {
        EventView { event }
    }
}

impl<'a> Widget for EventView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new([Text::raw(self.event.summary())].iter())
            .block(Block::default().borders(Borders::ALL))
            .render(area, buf);
    }
}
