use chrono::{Datelike, Utc};
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

    pub fn with_today(mut self) -> Self {
        let today = chrono::Utc::today();

        self.selected_day_idx = today.naive_utc().day0();
        self.selected_month_idx = today.naive_utc().month0();
        self
    }

    pub fn select_today(&mut self) {
        let today = chrono::Utc::today();

        self.selected_day_idx = today.naive_utc().day0();
        self.selected_month_idx = today.naive_utc().month0();
    }
}
