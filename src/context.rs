use chrono::prelude::*;
use num_traits::FromPrimitive;
use tui::widgets::ListState;

use crate::calendar::{Calendar, EventsOfDay};

pub struct Context {
    calendar: Calendar,
    pub evtlist_context: ListState,
    pub monthview_context: ListState,
    now: DateTime<Local>,
    pub cursor: DateTime<Local>,
}

impl Context {
    pub fn new(calendar: Calendar) -> Self {
        Context {
            calendar,
            evtlist_context: ListState::default(),
            monthview_context: ListState::default(),
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

    pub fn events_of_day(&self) -> EventsOfDay<FixedOffset> {
        let tz = FixedOffset::from_offset(self.cursor.offset());

        self.calendar
            .events_of_day(&self.cursor.with_timezone(&tz).date())
    }

    pub fn selected_day(&self) -> u32 {
        self.cursor.day()
    }

    pub fn selected_month(&self) -> Month {
        Month::from_u32(self.cursor.month()).unwrap()
    }

    pub fn selected_year(&self) -> i32 {
        self.cursor.year()
    }

    pub fn now(&self) -> &DateTime<Local> {
        &self.now
    }

    pub fn update(&mut self) {
        self.now = Local::now();
    }
}
