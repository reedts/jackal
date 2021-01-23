use crate::calendar::Month;

pub struct CalendarContext {
    pub day: u32,
    pub month: Month,
    pub year: i32,
}

impl Default for CalendarContext {
    fn default() -> Self {
        CalendarContext {
            day: 1,
            month: Month::from(1),
            year: 0,
        }
    }
}
