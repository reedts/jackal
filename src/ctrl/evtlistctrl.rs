use crate::cmds::{Cmd, Result};
use crate::ctrl::{Control, Selection};
use crate::ctx::Context;

pub struct EvtListController {}

impl EvtListController {
    pub fn default() -> Self {
        EvtListController {}
    }
}

impl Control for EvtListController {
    fn send_cmd(&mut self, cmd: Cmd, context: &mut Context) -> Result {
        Ok(Cmd::Noop)
    }
}

impl Selection for EvtListController {
    fn move_left(&mut self, _context: &mut Context) {}

    fn move_right(&mut self, _context: &mut Context) {}

    fn move_up(&mut self, context: &mut Context) {
        self.move_n_up(1, context);
    }

    fn move_down(&mut self, context: &mut Context) {
        self.move_n_down(1, context);
    }

    fn move_n_left(&mut self, _n: u32, _context: &mut Context) {}

    fn move_n_right(&mut self, _n: u32, _context: &mut Context) {}

    fn move_n_up(&mut self, n: u32, context: &mut Context) {
        let sel_evt = if let Some(item) = context.evtlist_context.selected() {
            item
        } else {
            0
        };
        context
            .evtlist_context
            .select(sel_evt.checked_sub(n as usize).map(|v| v as usize));
    }

    fn move_n_down(&mut self, n: u32, context: &mut Context) {
        let sel_evt = if let Some(item) = context.evtlist_context.selected() {
            item
        } else {
            0
        };
        context.evtlist_context.select(Some(std::cmp::min(
            context.get_day().events().len() - 1,
            sel_evt.checked_add(n as usize).unwrap_or(sel_evt),
        )));
    }
}
