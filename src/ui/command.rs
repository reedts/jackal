use std::collections::BTreeMap;
use std::result::Result;
use std::str::FromStr;
use unsegen::input::*;
use unsegen::widget::builtin::PromptLine;

use chrono::{Duration, NaiveDateTime};
use nom::{branch::alt, combinator::all_consuming, error::Error, IResult};

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
                    } else {
                        self.context.mode = Mode::Normal;
                    }
                    None
                }
                _ => Some(input),
            }
        } else {
            Some(input)
        }
    }
}

pub type ActionResult = Result<(), Error<String>>;

pub enum Action {
    Arg(fn(&mut Context, String) -> ActionResult),
    NoArg(fn(&mut Context) -> ActionResult),
    Repeatable(fn(&mut Context, u32) -> ActionResult),
}

const COMMANDS: &[(&'static str, Action)] = &[
    (
        "gy",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor + chrono::Duration::days(p as i64 * 365);
            Ok(())
        }),
    ),
    (
        "gY",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor - chrono::Duration::days(p as i64 * 365);
            Ok(())
        }),
    ),
    (
        "gw",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor + chrono::Duration::weeks(p as i64);
            Ok(())
        }),
    ),
    (
        "gW",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor - chrono::Duration::weeks(p as i64);
            Ok(())
        }),
    ),
    (
        "gd",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor + chrono::Duration::days(p as i64);
            Ok(())
        }),
    ),
    (
        "gD",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor - chrono::Duration::days(p as i64);
            Ok(())
        }),
    ),
    (
        "gh",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor + chrono::Duration::hours(p as i64);
            Ok(())
        }),
    ),
    (
        "gH",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor - chrono::Duration::hours(p as i64);
            Ok(())
        }),
    ),
    (
        "gm",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor + chrono::Duration::minutes(p as i64);
            Ok(())
        }),
    ),
    (
        "gM",
        Action::Repeatable(|c, p| {
            c.cursor = c.cursor - chrono::Duration::minutes(p as i64);
            Ok(())
        }),
    ),
];
