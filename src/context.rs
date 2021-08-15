use chrono::prelude::*;
use num_traits::FromPrimitive;
use tui::widgets::ListState;

use crate::agenda::{Agenda, EventsOfDay};
use crate::ui::CalendarViewState;

pub struct Context<'a> {
    calendar: Agenda<'a>,
    pub eventlist_context: ListState,
    pub calendarview_context: CalendarViewState,
    now: DateTime<Local>,
    pub cursor: DateTime<Local>,
}

impl Context<'_> {
    pub fn new(calendar: Agenda) -> Self {
        Context {
            calendar,
            eventlist_context: ListState::default(),
            calendarview_context: CalendarViewState::new(Local::now(), 1),
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
