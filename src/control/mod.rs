pub mod calendar;
pub mod control;
pub mod eventlist;

pub use calendar::CalendarController;
pub use control::{Control, Controller};
pub use eventlist::EventListController;

use crate::context::Context;

pub trait Selection {
    fn move_left(&mut self, context: &mut Context);
    fn move_right(&mut self, context: &mut Context);
    fn move_up(&mut self, context: &mut Context);
    fn move_down(&mut self, context: &mut Context);

    fn move_n_left(&mut self, n: u32, context: &mut Context);
    fn move_n_right(&mut self, n: u32, context: &mut Context);
    fn move_n_up(&mut self, n: u32, context: &mut Context);
    fn move_n_down(&mut self, n: u32, context: &mut Context);
}
