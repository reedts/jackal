use crate::agenda::Agenda;
use crate::cmds::{Cmd, CmdResult};
use crate::config::Config;
use crate::context::Context;
use crate::control::{CalendarController, Control, Controller, EventListController};
use crate::events::Event;
use crate::ui::{CalendarView, EventListView};

use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout};
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
                f.render_stateful_widget(EventListView::default(), layout[1], &mut app.global_ctx)
            }
        }
    }
}

pub enum View<'a> {
    Calendar(Controller<'a, CalendarController>),
    Events(Controller<'a, EventListController>),
}

pub struct App<'a> {
    pub quit: bool,
    views: [View<'a>; 2],
    config: &'a Config,
    global_ctx: Context,
}

impl<'a> Control for View<'a> {
    fn send_cmd(&mut self, cmd: &Cmd, context: &mut Context) -> CmdResult {
        match self {
            Self::Calendar(ctrlr) => ctrlr.inner_mut().send_cmd(cmd, context),
            Self::Events(ctrlr) => ctrlr.inner_mut().send_cmd(cmd, context),
        }
    }
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, calendar: Agenda) -> App<'a> {
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
                    EventListController::default(),
                )),
            ],
            config,
            global_ctx,
        }
    }

    pub fn handle(&mut self, event: Event) -> CmdResult {
        match event {
            Event::Tick => {
                self.global_ctx.update();
                Ok(Cmd::Noop)
            }
            Event::Input(key) => {
                if let Some(cmd) = self.config.key_map.get(&key) {
                    if let Cmd::Exit = cmd {
                        self.quit = true;
                        Ok(Cmd::Noop)
                    } else {
                        for view in self.views.iter_mut() {
                            view.send_cmd(cmd, &mut self.global_ctx)?;
                        }
                        Ok(Cmd::Noop)
                    }
                } else {
                    Ok(Cmd::Noop)
                }
            }
            _ => Ok(Cmd::Noop),
        }
    }
}
