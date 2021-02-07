use crate::cmds::{Cmd, CmdResult};
use crate::ctrl::{Control, Selection};
use crate::ctx::Context;

pub struct CalendarController {}

impl Default for CalendarController {
    fn default() -> Self {
        CalendarController {}
    }
}

impl Control for CalendarController {
    fn send_cmd(&mut self, cmd: &Cmd, context: &mut Context) -> CmdResult {
        match cmd {
            Cmd::NextDay => {
                self.move_right(context);
                Ok(Cmd::Noop)
            }
            Cmd::PrevDay => {
                self.move_left(context);
                Ok(Cmd::Noop)
            }
            Cmd::NextWeek => {
                self.move_down(context);
                Ok(Cmd::Noop)
            }
            Cmd::PrevWeek => {
                self.move_up(context);
                Ok(Cmd::Noop)
            }
            _ => Ok(*cmd),
        }
    }
}

impl Selection for CalendarController {
    fn move_left(&mut self, context: &mut Context) {
        self.move_n_left(1, context);
    }

    fn move_right(&mut self, context: &mut Context) {
        self.move_n_right(1, context);
    }

    fn move_up(&mut self, context: &mut Context) {
        self.move_n_up(1, context);
    }

    fn move_down(&mut self, context: &mut Context) {
        self.move_n_down(1, context);
    }

    fn move_n_left(&mut self, n: u32, context: &mut Context) {
        context.cursor = context.cursor - chrono::Duration::days(n as i64);
    }

    fn move_n_right(&mut self, n: u32, context: &mut Context) {
        context.cursor = context.cursor + chrono::Duration::days(n as i64);
    }

    fn move_n_up(&mut self, n: u32, context: &mut Context) {
        context.cursor = context.cursor - chrono::Duration::days(7 * n as i64);
    }

    fn move_n_down(&mut self, n: u32, context: &mut Context) {
        context.cursor = context.cursor + chrono::Duration::days(7 * n as i64);
    }
}
