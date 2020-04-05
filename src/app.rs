use crate::calendar::Calendar;
use crate::cmds::{Cmd, Result};
use crate::config::Config;
use crate::control::{Control, Controller};
use crate::events::Event;
use crate::ui::calview::CalendarView;

use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{Block, Borders, Widget};

pub enum View<'a> {
    Calendar(Controller<'a, CalendarView<'a>>),
}

pub struct App<'a> {
    pub quit: bool,
    views: [View<'a>; 1],
    calendar: &'a mut Calendar<'a>,
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
    pub fn new(config: &'a Config, calendar: &'a mut Calendar<'a>) -> App<'a> {
        App {
            quit: false,
            views: [View::Calendar(Controller::new(&config.key_map, CalendarView::new(calendar)))],
            calendar,
            active_view: 0,
            config,
        }
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

impl<'a> Widget for App<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        for view in self.views.iter_mut() {
            match view {
                View::Calendar(ctrlr) => ctrlr.inner_mut().draw(layout[0], buf),
            }
        }

        Block::default()
            .title("Events")
            .borders(Borders::ALL)
            .draw(layout[1], buf);
    }
}
