pub mod calview;
pub mod evtlistview;
pub mod evtview;

pub(crate) mod util;

pub use calview::CalendarView;
pub use evtview::EventView;

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
