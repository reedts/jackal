pub mod calendar;
pub mod event;
pub mod eventlist;
pub mod util;

pub use self::calendar::{CalendarView, CalendarViewState};
pub use self::event::EventView;
pub use self::eventlist::EventListView;

pub trait WidgetSize {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

pub trait EstimatedWidgetSize {
    fn est_size() -> (u16, u16) {
        (Self::est_width(), Self::est_height())
    }
    fn est_width() -> u16;
    fn est_height() -> u16;
}
