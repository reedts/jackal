pub struct CalendarContext {
    pub selected_day: u32,
    pub selected_month: u32
}

impl CalendarContext {
    pub fn default() -> Self {
        CalendarContext {
            selected_day: 0,
            selected_month: 0
        }
    }
}
