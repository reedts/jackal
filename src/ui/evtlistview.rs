use chrono::Utc;
use std::cell::RefCell;
use std::rc::Rc;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::widgets::{List, ListState, Widget};
use crate::calendar::{Day, Calendar};

pub struct EvtListView<'a> {
    calendar: Rc<RefCell<Calendar>>,
    day: Option<&'a Day<Utc>>,
    state: ListState
}

impl<'a> EvtListView<'a> {
    pub fn new(calendar: Rc<RefCell<Calendar>>) -> Self{
        EvtListView { calendar, day: None, state: ListState::default() }
    }

    pub fn for_day(&mut self, day: &'a Day<Utc>) {
        self.day = Some(day);

        let events = self.day.unwrap().events();
    }
}

impl<'a> Widget for EvtListView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {

    }
}
