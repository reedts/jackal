use crate::ctx::Context;
use crate::ui::evtview::EventView;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget};

pub struct EvtListView {
    style: Style,
    focus_style: Style,
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
            day.events()
                .iter()
                .map(|ev| ListItem::new(EventView::with(ev)))
                .collect()
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
