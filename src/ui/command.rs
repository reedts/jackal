use nom::IResult;
use std::result::Result;
use unsegen::input::*;
use unsegen::widget::builtin::PromptLine;

use super::context::{Context, Mode};
use crate::config::Config;

pub struct CommandParser<'a, 'c> {
    context: &'a mut Context<'c>,
    config: &'a Config,
}

impl<'a, 'c> CommandParser<'a, 'c> {
    pub fn new(context: &'a mut Context<'c>, config: &'a Config) -> Self {
        CommandParser { context, config }
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn report_error(&mut self, error: Box<dyn std::error::Error>) {
        self.context.last_error_message = Some(format!("{}", error));
    }
}

impl Behavior for CommandParser<'_, '_> {
    fn input(mut self, input: Input) -> Option<Input> {
        if let Event::Key(key) = input.event {
            match key {
                Key::Char('\n') => {
                    let cmd = self
                        .context
                        .input_sink_mut(Mode::Command)
                        .finish_line()
                        .to_owned();
                    if let Err(e) = self.run_command(&cmd) {
                        self.report_error(e);
                        None
                    } else {
                        self.context.mode = Mode::Normal;
                        None
                    }
                }
                _ => Some(input),
            }
        } else {
            Some(input)
        }
    }
}
