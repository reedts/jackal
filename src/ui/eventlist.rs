use crate::context::Context;
use crate::ui::EventView;
use chrono::{NaiveTime, Utc};
use tui::buffer::Buffer;
use tui::layout::{Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget};

pub struct EvtListContext {
    event: u32,
}

impl Default for EvtListContext {
    fn default() -> Self {
        EvtListContext { event: 0 }
    }
}

impl EvtListContext {
    pub fn new(idx: u32) -> Self {
        EvtListContext { event: idx }
    }
}

pub struct EventListView {
    style: Style,
    focus_style: Style,
    vertical_padding: u16,
    horizontal_padding: u16,
    item_indent: u16,
    cursor_indent: u16,
}

impl Default for EventListView {
    fn default() -> Self {
        EventListView {
            style: Style::default(),
            focus_style: Style::default(),
            vertical_padding: 5,
            horizontal_padding: 10,
            item_indent: 10,
            cursor_indent: 0,
        }
    }
}

impl EventListView {
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = style;
        self
    }

    pub fn vertical_padding(mut self, padding: u16) -> Self {
        self.vertical_padding = padding;
        self
    }

    pub fn horizontal_padding(mut self, padding: u16) -> Self {
        self.horizontal_padding = padding;
        self
    }

    pub fn item_indent(mut self, indent: u16) -> Self {
        self.item_indent = indent;
        self
    }

    pub fn cursor_indent(mut self, indent: u16) -> Self {
        self.cursor_indent = indent;
        self
    }
}

impl StatefulWidget for EventListView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = {
            let day = state.events_of_day();

            // FIXME: worthy?
            let evts = day.events();

            let now = state.now();

            let mut items: Vec<ListItem> = evts
                .iter()
                .map(|ev| ListItem::new(EventView::with(ev).indent(self.item_indent)))
                .collect();

            if day.date().with_timezone(&Utc) == now.date() {
                let pos = evts.binary_search_by(|&ev| {
                    ev.begin()
                        .inner_as_datetime()
                        .with_timezone(&Utc)
                        .time()
                        .cmp(&now.time())
                });
                // FIXME: Ther must be a better way to unwrap
                items.insert(
                    pos.unwrap_or_else(std::convert::identity),
                    ListItem::new(EventListCursor::new(now.time()).indent(self.cursor_indent)),
                );
            }

            items
        };

        if items.is_empty() {
            Paragraph::new(Text::styled(
                "No events",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))
            .block(Block::default().title("Events").borders(Borders::ALL))
            .render(area, buf);
        } else {
            Block::default()
                .title("Events")
                .borders(Borders::ALL)
                .render(area, buf);
            StatefulWidget::render(
                List::new(items).highlight_symbol(">"),
                Rect::new(
                    area.x + self.horizontal_padding,
                    area.y + self.vertical_padding,
                    area.width - 2 * self.vertical_padding,
                    area.height - 2 * self.horizontal_padding,
                ),
                buf,
                &mut state.eventlist_context,
            );
        }
    }
}

struct EventListCursor {
    style: Style,
    time: NaiveTime,
    indent: u16,
}

impl EventListCursor {
    fn new(time: NaiveTime) -> Self {
        EventListCursor {
            style: Style::default(),
            time,
            indent: 0,
        }
    }

    fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    fn indent(mut self, indent: u16) -> Self {
        self.indent = indent;
        self
    }
}

impl<'a> Into<Text<'a>> for EventListCursor {
    fn into(self) -> Text<'a> {
        Text::from(Spans::from(vec![
            Span::raw(" ".repeat(self.indent as usize)),
            Span::styled(self.time.format("%H:%M").to_string(), self.style),
            Span::styled(" -> ", self.style),
        ]))
    }
}
