use chrono::Utc;
use crate::calendar::{Calendar, Day};

pub struct Context {
    pub selected_day_idx: u32,
    pub selected_month_idx: u32,
    pub calendar: Calendar
}

impl Context {
    pub fn new(calendar: Calendar) -> Self {
        Context {
            selected_day_idx: 0,
            selected_month_idx: 0,
            calendar 
        }
    }

    pub fn get_selected_day(&self) -> &Day<Utc> {
        &self.calendar.month_from_idx(self.selected_month_idx).unwrap().day(self.selected_day_idx as usize)
    }
}
