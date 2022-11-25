use crate::agenda::Agenda;
use crate::config::Config;
use crate::events::{Dispatcher, Event};
use crate::provider::NewEvent;

use super::{CalendarWindow, Context, EventWindow, EventWindowBehaviour, Mode};

use unsegen::base::{GraphemeCluster, Terminal};
use unsegen::input::{
    EditBehavior, Key, Navigatable, NavigateBehavior, OperationResult, ScrollBehavior,
};
use unsegen::widget::*;

use super::command::CommandParser;
use super::insert::InsertParser;

pub struct App<'a> {
    config: &'a Config,
    context: Context,
}

impl<'a> App<'a> {
    pub fn new(config: &'a Config, agenda: Agenda) -> App<'a> {
        let context = Context::new(agenda);
        App { config, context }
    }

    fn bottom_bar<'w>(&'w self) -> impl Widget + 'w {
        let spacer = " ".with_demand(|_| Demand2D {
            width: ColDemand::exact(1),
            height: RowDemand::exact(1),
        });

        let mut layout = HLayout::new()
            .separator(GraphemeCluster::try_from(' ').unwrap())
            .widget(spacer);
        if let mode @ (Mode::Command | Mode::Insert) = self.context.mode {
            layout = layout.widget(self.context.input_sink(mode).as_widget());
        }

        layout
    }

    fn as_widget<'w>(&'w self) -> impl Widget + 'w
    where
        'a: 'w,
    {
        let layout = VLayout::new()
            .widget(
                HLayout::new()
                    .widget(CalendarWindow::new(&self.context))
                    .widget(EventWindow::new(&self.context)),
            )
            .widget(self.bottom_bar());

        layout
    }

    pub fn run<'r>(
        &'r mut self,
        dispatcher: Dispatcher,
        mut term: Terminal,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        'a: 'r,
    {
        let mut run = true;

        while run {
            // Handle events
            if let Ok(event) = dispatcher.next() {
                match event {
                    Event::Update => self.context.update(),
                    Event::ExternalModification => {
                        self.context.agenda_mut().process_external_modifications();
                        self.context.update();
                    }
                    Event::Input(input) => {
                        let num_events_of_current_day = self
                            .context
                            .agenda()
                            .events_of_day(&self.context.cursor().date_naive())
                            .count();

                        if input.matches(Key::Esc) {
                            self.context.mode = Mode::Normal;
                        } else {
                            match self.context.mode {
                                Mode::Normal => {
                                    let _leftover = input
                                        .chain((Key::Char('q'), || run = false))
                                        .chain((Key::Char(':'), || {
                                            self.context.mode = Mode::Command
                                        }))
                                        .chain((Key::Char('i'), || {
                                            self.context.mode = Mode::Insert
                                        }))
                                        .chain(
                                            NavigateBehavior::new(&mut CursorBehaviour(
                                                &mut self.context,
                                            ))
                                            .down_on(Key::Char('j'))
                                            .up_on(Key::Char('k'))
                                            .left_on(Key::Char('h'))
                                            .right_on(Key::Char('l')),
                                        )
                                        .chain(
                                            ScrollBehavior::new(&mut EventWindowBehaviour(
                                                &mut self.context,
                                                num_events_of_current_day,
                                            ))
                                            .forwards_on(Key::Char(']'))
                                            .backwards_on(Key::Char('[')),
                                        )
                                        .finish();
                                }
                                mode @ Mode::Insert => {
                                    let begin = self
                                        .context
                                        .cursor()
                                        .with_timezone(&chrono_tz::Europe::Berlin);

                                    input
                                        .chain(
                                            EditBehavior::new(self.context.input_sink_mut(mode))
                                                .delete_forwards_on(Key::Delete)
                                                .delete_backwards_on(Key::Backspace)
                                                .left_on(Key::Left)
                                                .right_on(Key::Right),
                                        )
                                        .chain(
                                            ScrollBehavior::new(self.context.input_sink_mut(mode))
                                                .backwards_on(Key::Up)
                                                .forwards_on(Key::Down),
                                        )
                                        .chain(InsertParser::new(
                                            &mut self.context,
                                            &self.config,
                                            NewEvent::new(begin),
                                        ))
                                        .finish();
                                }
                                mode @ Mode::Command => {
                                    input
                                        .chain(
                                            EditBehavior::new(self.context.input_sink_mut(mode))
                                                .delete_forwards_on(Key::Delete)
                                                .delete_backwards_on(Key::Backspace)
                                                .left_on(Key::Left)
                                                .right_on(Key::Right),
                                        )
                                        .chain(
                                            ScrollBehavior::new(self.context.input_sink_mut(mode))
                                                .backwards_on(Key::Up)
                                                .forwards_on(Key::Down),
                                        )
                                        .chain(CommandParser::new(&mut self.context, &self.config))
                                        .finish();
                                }
                            }
                        }
                    }
                }
            }

            // Draw
            let root = term.create_root_window();

            let _ = self.as_widget().draw(root, RenderingHints::new());

            term.present();
        }

        Ok(())
    }
}

struct CursorBehaviour<'a>(&'a mut Context);

impl Navigatable for CursorBehaviour<'_> {
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
