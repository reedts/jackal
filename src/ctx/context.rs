use crate::calendar::{Calendar, EventsOfDay, Month};
use chrono::{DateTime, Datelike, FixedOffset, Local, TimeZone};
use tui::widgets::ListState;

pub struct Context {
    pub calendar: Calendar,
    pub evtlist_context: ListState,
    pub now: DateTime<Local>,
    pub cursor: DateTime<Local>,
}

impl Context {
    pub fn new(calendar: Calendar) -> Self {
        Context {
            calendar,
            evtlist_context: ListState::default(),
            now: Local::now(),
            cursor: Local::now(),
        }
    }

    pub fn with_today(mut self) -> Self {
        self.select_today();
        self
    }

    pub fn select_today(&mut self) {
        self.cursor = Local::now();
    }

    pub fn get_events_of_day(&self) -> EventsOfDay<FixedOffset> {
        let tz = FixedOffset::from_offset(self.cursor.offset());

        self.calendar
            .events_of_day(&self.cursor.with_timezone(&tz).date())
    }

    pub fn get_selected_day(&self) -> u32 {
        self.cursor.day()
    }

    pub fn get_selected_month(&self) -> Month {
        Month::from(self.cursor.month())
    }

    pub fn get_selected_year(&self) -> i32 {
        self.cursor.year()
    }

    pub fn now(&self) -> &DateTime<Local> {
        &self.now
    }

    pub fn update(&mut self) {
        self.now = Local::now();
    }
}
