use std::fmt::Write;

use crate::agenda::Agenda;
use crate::config::Config;
use crate::events::{Dispatcher, Event};

use super::{CalendarWindow, Context, MonthPane, TuiContext};

use unsegen::base::{Cursor, Terminal};
use unsegen::input::{Input, Key, Navigatable, NavigateBehavior, OperationResult};
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
                        input.chain((Key::Char('q'), || run = false)).chain(
                            NavigateBehavior::new(&mut DtCursorBehaviour(
                                self.context.tui_context_mut(),
                            ))
                            .down_on(Key::Char('j'))
                            .up_on(Key::Char('k'))
                            .left_on(Key::Char('h'))
                            .right_on(Key::Char('l')),
                        );
                    }
                    _ => {}
                }
            }

            // Draw
            let mut root = term.create_root_window();

            let mut layout = HLayout::new()
                .widget(CalendarWindow::new(&self.context))
                .draw(root, RenderingHints::new());

            term.present();
        }

        Ok(())
    }
}

struct DtCursorBehaviour<'a>(&'a mut TuiContext);

impl Navigatable for DtCursorBehaviour<'_> {
    fn move_down(&mut self) -> OperationResult {
        self.0.cursor = self.0.cursor + chrono::Duration::weeks(1);
        Ok(())
    }

    fn move_left(&mut self) -> OperationResult {
        self.0.cursor = self.0.cursor - chrono::Duration::days(1);
        Ok(())
    }

    fn move_right(&mut self) -> OperationResult {
        self.0.cursor = self.0.cursor + chrono::Duration::days(1);
        Ok(())
    }

    fn move_up(&mut self) -> OperationResult {
        self.0.cursor = self.0.cursor - chrono::Duration::weeks(1);
        Ok(())
    }
}
