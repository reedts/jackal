use crate::calendar::Calendar;
use crate::cmds::{Cmd, Result};
use crate::config::Config;
use crate::ctrl::{CalendarController, Control, Controller, EvtListController};
use crate::ctx::{CalendarContext, Context, EvtListContext};
use crate::events::Event;
use crate::ui::calview::CalendarView;
use crate::ui::evtlistview::EvtListView;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use tui::backend::Backend;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{Block, Borders, Widget};
use tui::Frame;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    for view in app.views.iter_mut() {
        match view {
            View::Calendar(_) => {
                f.render_stateful_widget(CalendarView::default(), layout[0], &mut app.global_ctx)
            }
            View::Events(_) => {
                f.render_stateful_widget(EvtListView::default(), layout[1], &mut app.global_ctx)
            }
        }
    }
}

pub enum View<'a> {
    Calendar(Controller<'a, CalendarController>),
    Events(Controller<'a, EvtListController>),
}

pub struct App<'a> {
    pub quit: bool,
    views: [View<'a>; 2],
    active_view: usize,
    config: &'a Config,
    global_ctx: Context,
}

impl<'a> Control for View<'a> {
    fn send_cmd(&mut self, cmd: Cmd, context: &mut Context) -> Result {
        match self {
            Self::Calendar(ctrlr) => ctrlr.inner_mut().send_cmd(cmd, context),
            Self::Events(ctrlr) => ctrlr.inner_mut().send_cmd(cmd, context),
        }
    }
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, calendar: Calendar) -> App<'a> {
        let global_ctx = Context::new(calendar).with_today();
        App {
            quit: false,
            views: [
                View::Calendar(Controller::new(
                    &config.key_map,
                    CalendarController::default(),
                )),
                View::Events(Controller::new(
                    &config.key_map,
                    EvtListController::default(),
                )),
            ],
            active_view: 0,
            config,
            global_ctx,
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
                    match &mut self.views[self.active_view] {
                        View::Calendar(ctrlr) => ctrlr.handle(event, &mut self.global_ctx),
                        View::Events(ctrlr) => ctrlr.handle(event, &mut self.global_ctx),
                    }
                }
            }
            _ => Ok(Cmd::Noop),
        }
    }
}
