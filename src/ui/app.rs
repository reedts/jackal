
use crate::agenda::Agenda;
use crate::config::Config;
use crate::events::{Dispatcher, Event};

use super::{Context, MonthPane};

use unsegen::base::Terminal;
use unsegen::input::{Input, Key};
use unsegen::widget::*;

pub struct App<'a> {
    config: &'a Config,
    context: Context<'a>,
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, agenda: Agenda<'a>) -> App<'a> {
        let context = Context::new(agenda);
        App { config, context }
    }

    fn as_widget<'w>(&'w self) -> impl Widget + 'w
    where
        'a: 'w,
    {
        let mut layout = HLayout::new().widget(MonthPane::new(
            self.context.current_month(),
            self.context.current_year(),
            &self.context,
        ));

        layout
    }

    pub fn run(
        &mut self,
        dispatcher: Dispatcher,
        mut term: Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut run = true;

        while run {
            // Handle events
            if let Ok(event) = dispatcher.next() {
                match event {
                    Event::Update => self.context.update(),
                    Event::Input(input) => {
                        input.chain((Key::Char('q'), || run = false));
                    },
                    _ => {},
                }
            }

            // Draw
            let root = term.create_root_window();

            term.present();
        }

        Ok(())
    }
}
