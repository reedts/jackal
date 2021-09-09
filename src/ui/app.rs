use std::collections::BTreeMap;

use crate::agenda::Agenda;
use crate::cmds::*;
use crate::config::Config;
use crate::events::{Dispatcher, Event};

use super::{Context, MonthPane};

use unsegen::{widget::*, base::Terminal};


pub struct App<'a> {
    config: &'a Config,
    global_ctx: Context<'a>,
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, agenda: Agenda<'a>) -> App<'a> {
        let global_ctx = Context::new(agenda);
        App {
            config,
            global_ctx,
        }
    }

    fn as_widget<'w>(&self) -> impl Widget + 'w where 'a: 'w {
        let mut layout = HLayout::new().widget(MonthPane::new(self.global_ctx.current_month(), self.global_ctx.current_year(), &self.global_ctx));

        layout
    }

    pub fn run(&mut self, dispatcher: Dispatcher, mut term: Terminal) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut run = true;

        while run {
            // Handle events
            // let result = match dispatcher.next() {
            //     Ok(event) => match event {
            //         Event::Tick => self.handle(Event::Tick),
            //         Event::Input(key) => self.handle(Event::Input(key)),
            //         _ => Ok(Cmd::Noop),
            //     },
            //     Err(e) => Err(CmdError::new(format!("{}", e))),
            // }?;

            // Draw
            let root = term.create_root_window();
                
            term.present();
        }

        Ok(())
    }
}
