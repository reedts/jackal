use chrono::{Datelike, Utc};
use crate::calendar::{Calendar, Day, Month};
use crate::ctx::CalendarContext;
use crate::ctx::EvtListContext;

pub struct Context {
    pub calendar: Calendar,
    pub calendar_context: CalendarContext,
    pub evtlist_context: EvtListContext
}

impl Context {
    pub fn new(calendar: Calendar) -> Self {
        Context {
            calendar,
            calendar_context: CalendarContext::default(),
            evtlist_context: EvtListContext::default()
        }
    }

    pub fn get_selected_day(&self) -> &Day<Utc> {
        &self.calendar.month_from_idx(self.calendar_context.selected_month).unwrap().day(self.calendar_context.selected_day as usize)
    }

    pub fn get_selected_month(&self) -> &Month<Utc> {
        &self.calendar.month_from_idx(self.calendar_context.selected_month).unwrap()
    }

    pub fn with_today(mut self) -> Self {
        self.select_today();
        self
    }

    pub fn select_today(&mut self) {
        let today = chrono::Utc::today();

        self.calendar_context.selected_day = today.naive_utc().day0();
        self.calendar_context.selected_month = today.naive_utc().month0();
    }
}
