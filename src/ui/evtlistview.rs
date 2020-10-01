use tui::buffer::Buffer;
use tui::layout::{Layout, Direction, Rect, Constraint};
use tui::style::Style;
use tui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget};
use crate::cmds::{Cmd, Result};
use crate::context::Context;
use crate::control::Control;
use crate::ui::Selection;
use crate::ui::evtview::EventView;

pub struct EvtListView {
    style: Style,
    focus_style: Style,
}

pub struct EvtListViewState {
    selected: u32
}

impl EvtListView {
    pub fn default() -> Self {
        EvtListView { style: Style::default(), focus_style: Style::default() }
    }
}

impl StatefulWidget for EvtListView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let day = state.get_selected_day();

        let items: Vec<ListItem> = day.events().iter().map(|ev| ListItem::new(EventView::with(ev))).collect();

        List::new(items)
            .block(Block::default().title("Events").borders(Borders::ALL))
            .highlight_symbol(">")
            .render(area, buf, &mut ListState::default());
    }
}


impl EvtListViewState {
    pub fn new(idx: u32) -> Self {
        EvtListViewState { selected: idx }
    }

    pub fn default() -> Self {
        EvtListViewState { selected: 0 }
    }

    fn checked_select_n_prev(&mut self, n: u32, context: &mut Context) {
        self.selected = std::cmp::max(0, self.selected - n)
    }

    fn checked_select_n_next(&mut self, n: u32, context: &mut Context) {
        self.selected = std::cmp::min(context.get_selected_day().events().len() as u32, self.selected - n)
    }
}

impl Control for EvtListViewState {
    fn send_cmd(&mut self, cmd: Cmd, context: &mut Context) -> Result {
        Ok(Cmd::Noop)
    }
}

impl Selection for EvtListViewState {
    fn move_left(&mut self, context: &mut Context) {
        self.checked_select_n_prev(1, context);
    }

    fn move_right(&mut self, context: &mut Context) {
        self.checked_select_n_next(1, context);
    }

    fn move_up(&mut self, context: &mut Context) {
        self.checked_select_n_prev(7, context);
    }

    fn move_down(&mut self, context: &mut Context) {
        self.checked_select_n_next(7, context);
    }

    fn move_n_left(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_prev(n, context);
    }

    fn move_n_right(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_next(n, context);
    }

    fn move_n_up(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_prev(n * 7, context);
    }

    fn move_n_down(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_next(n * 7, context);
    }
}
