use std::cell::{Ref, RefCell};
use std::rc::Rc;
use crate::calendar::Calendar;
use crate::cmds::{Cmd, Result};
use crate::config::Config;
use crate::control::{Control, Controller};
use crate::events::Event;
use crate::ui::calview::{CalendarView, CalendarViewState};

use tui::Frame;
use tui::backend::Backend;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{Block, Borders, Widget};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    for view in app.views.iter_mut() {
        match view {
            View::Calendar(ctrlr) => f.render_stateful_widget(CalendarView::new(&app.calendar.borrow()), layout[0], ctrlr.inner_mut()),
        }
    }

    let block = Block::default()
        .title("Events")
        .borders(Borders::ALL);

    f.render_widget(block, layout[1]);
}

pub enum View<'a> {
    Calendar(Controller<'a, CalendarViewState>),
}

pub struct App<'a> {
    pub quit: bool,
    views: [View<'a>; 1],
    calendar: Rc<RefCell<Calendar>>,
    active_view: usize,
    config: &'a Config,
}

impl<'a> Control for View<'a> {
    fn send_cmd(&mut self, cmd: Cmd) -> Result {
        match self {
            Self::Calendar(ctrlr) => ctrlr.inner_mut().send_cmd(cmd),
        }
    }
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, calendar: Calendar) -> App<'a> {
        let calendar = Rc::new(RefCell::new(calendar));
        App {
            quit: false,
            views: [
                View::Calendar(Controller::new(&config.key_map, CalendarViewState::new(calendar.clone())))
            ],
            calendar,
            active_view: 0,
            config,
        }
    }

    pub fn calendar(&self) -> Ref<Calendar> {
        self.calendar.borrow()
    }

    fn active_view_mut(&mut self) -> &mut View<'a> {
        &mut self.views[self.active_view]
    }

    pub fn handle(&mut self, event: Event) -> Result {
        match event {
            Event::Input(key) => {
                if let Cmd::Exit = self.config.key_map.get(&key).unwrap() {
                    self.quit = true;
                    Ok(Cmd::Noop)
                } else {
                    match self.active_view_mut() {
                        View::Calendar(ctrlr) => ctrlr.handle(event),
                    }
                }
            }
            _ => Ok(Cmd::Noop),
        }
    }

}
