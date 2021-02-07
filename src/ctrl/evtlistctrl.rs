use crate::cmds::{Cmd, CmdResult};
use crate::ctrl::{Control, Selection};
use crate::ctx::Context;

pub struct EvtListController {}

impl Default for EvtListController {
    fn default() -> Self {
        EvtListController {}
    }
}

impl Control for EvtListController {
    fn send_cmd(&mut self, cmd: &Cmd, context: &mut Context) -> CmdResult {
        use Cmd::*;
        match cmd {
            NextEvent => self.move_down(context),
            PrevEvent => self.move_up(context),
            _ => {}
        }

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
            item.saturating_sub(n as usize)
        } else {
            0
        };
        context.evtlist_context.select(Some(sel_evt));
    }

    fn move_n_down(&mut self, n: u32, context: &mut Context) {
        let sel_evt = if let Some(item) = context.evtlist_context.selected() {
            std::cmp::min(
                item + n as usize,
                context.get_events_of_day().events().len() - 1,
            )
        } else {
            0
        };
        context.evtlist_context.select(Some(sel_evt));
    }
}
