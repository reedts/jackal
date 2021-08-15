use crate::agenda::Agenda;
use crate::cmds::*;
use crate::config::Config;
use crate::context::Context;
use crate::control::*;
use crate::events::{Dispatcher, Event};

use unsegen::base::Terminal;

pub enum View<'a> {
    Calendar(Controller<'a, CalendarController>),
    Events(Controller<'a, EventListController>),
}

pub struct App<'a> {
    pub quit: bool,
    views: [View<'a>; 2],
    config: &'a Config,
    global_ctx: Context<'a>,
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

    pub fn run(&mut self, dispatcher: Dispatcher, mut term: Terminal) -> Result<(), Box<dyn std::error::Error>>
    {
        loop {
            // Handle events
            let result = match dispatcher.next() {
                Ok(event) => match event {
                    Event::Tick => self.handle(Event::Tick),
                    Event::Input(key) => self.handle(Event::Input(key)),
                    _ => Ok(Cmd::Noop),
                },
                Err(e) => Err(CmdError::new(format!("{}", e))),
            }?;

            if self.quit {
                break;
            }
            
            // Draw
            let root = term.create_root_window();

            term.present();
        }

        Ok(())
    }
}
