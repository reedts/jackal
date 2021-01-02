use crate::calendar::{Calendar, Day, Month};
use crate::ctx::CalendarContext;
use chrono::{Datelike, FixedOffset};
use tui::widgets::ListState;

pub struct Context {
    pub calendar: Calendar,
    pub calendar_context: CalendarContext,
    pub evtlist_context: ListState,
}

impl Context {
    pub fn new(calendar: Calendar) -> Self {
        Context {
            calendar,
            calendar_context: CalendarContext::default(),
            evtlist_context: ListState::default(),
        }
    }

    pub fn with_today(mut self) -> Self {
        self.select_today();
        self
    }

    pub fn select_today(&mut self) {
        let today = chrono::Utc::today();

        self.calendar_context.day = today.naive_utc().day();
        self.calendar_context.month = Month::from(today.naive_utc().month());
        self.calendar_context.year = today.naive_utc().year();
    }

    pub fn get_day(&self) -> Day<FixedOffset> {
        self.calendar.events_of_day(
            self.calendar_context.day,
            self.calendar_context.month,
            self.calendar_context.year,
        )
    }

    pub fn get_month(&self) -> Month {
        Month::from(self.calendar_context.month)
    }

    pub fn get_year(&self) -> i32 {
        self.calendar_context.year
    }
}
