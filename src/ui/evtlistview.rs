use crate::ctx::Context;
use crate::ui::evtview::EventView;
use chrono::{NaiveTime, Utc};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget};

pub struct EvtListView {
    style: Style,
    focus_style: Style,
}

struct EvtListCursor {
    style: Style,
    time: NaiveTime,
}

impl EvtListView {
    pub fn default() -> Self {
        EvtListView {
            style: Style::default(),
            focus_style: Style::default(),
        }
    }
}

impl StatefulWidget for EvtListView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = {
            let day = state.get_day();

            // FIXME: worthy?
            let evts = day.events();

            let now = state.now();

            let mut items: Vec<ListItem> = evts
                .iter()
                .map(|ev| ListItem::new(EventView::with(ev)))
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
                    ListItem::new(EvtListCursor::new(None, now.time())),
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
            StatefulWidget::render(
                List::new(items)
                    .block(Block::default().title("Events").borders(Borders::ALL))
                    .highlight_symbol(">"),
                area,
                buf,
                &mut state.evtlist_context,
            );
        }
    }
}

impl EvtListCursor {
    fn new(style: Option<Style>, time: NaiveTime) -> Self {
        EvtListCursor {
            style: style.unwrap_or_default(),
            time,
        }
    }
}

impl<'a> Into<Text<'a>> for EvtListCursor {
    fn into(self) -> Text<'a> {
        Text::from(Spans::from(vec![
            Span::styled(" <-  ", self.style),
            Span::styled(self.time.format("%H:%M").to_string(), self.style),
        ]))
    }
}
