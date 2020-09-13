pub mod calview;
pub mod evtview;
pub mod evtlistview;

pub use calview::CalendarView;
pub use evtview::EventView;

pub trait Selection {
    fn move_left(&mut self);
    fn move_right(&mut self);
    fn move_up(&mut self);
    fn move_down(&mut self);

    fn move_n_left(&mut self, n: u32);
    fn move_n_right(&mut self, n: u32);
    fn move_n_up(&mut self, n: u32);
    fn move_n_down(&mut self, n: u32);
}
