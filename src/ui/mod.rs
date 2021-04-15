pub mod calview;
pub mod evtlistview;
pub mod evtview;

pub(crate) mod util;

pub use calview::CalendarView;
pub use evtview::EventView;

pub trait Measure {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}
