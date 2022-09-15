use std::collections::BTreeMap;
use std::result::Result;
use std::str::FromStr;
use unsegen::input::*;
use unsegen::widget::builtin::PromptLine;

use chrono::{Duration, NaiveDateTime};

use nom::{
    branch::alt,
    bytes::complete::*,
    character::complete::*,
    combinator::*,
    error::{Error, ErrorKind, ParseError},
    sequence::{pair, separated_pair, tuple},
    Err, IResult,
};

use super::context::{Context, Mode};
use crate::config::Config;

pub struct CommandParser<'a> {
    context: &'a mut Context,
    config: &'a Config,
}

pub fn match_action<'a, 's, T: ?Sized, Act: 's>(
    c: &'a T,
) -> impl Fn(&str) -> IResult<&str, (&'s str, &'s Act)> + 'a
where
    &'a T: IntoIterator<Item = &'s (&'s str, Act)>,
{
    move |input| {
        if let Some((name, act)) = c.into_iter().find(|(name, _)| name == &input) {
            Ok(("", (name, act)))
        } else {
            Err(Err::Failure(ParseError::from_error_kind(
                input,
                ErrorKind::Tag,
            )))
        }
    }
}

impl<'a> CommandParser<'a> {
    pub fn new(context: &'a mut Context, config: &'a Config) -> Self {
        CommandParser { context, config }
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<(), Error<String>> {
        let res = all_consuming(tuple((digit1, match_action(COMMANDS))))(cmd);

        if let Ok((_, (repeat, (_, act)))) = res {
            let repeats = if repeat.is_empty() {
                1
            } else {
                u32::from_str_radix(repeat, 10)
                    .or_else::<Error<String>, _>(|_| {
                        return Err(ParseError::from_error_kind(repeat.into(), ErrorKind::Digit));
                    })
                    .unwrap()
            };

            if let Action::Repeatable(a) = act {
                return a(self.context, repeats);
            }
        };

        let res = all_consuming(separated_pair(match_action(COMMANDS), space1, rest))(cmd);

        if let Ok((_, ((_, act), arg))) = res {
            if let Action::Arg(a) = act {
                return a(self.context, arg.to_owned());
            } else {
                return Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag));
            }
        };

        let (_, (cmd, act)) = all_consuming(match_action(COMMANDS))(cmd)
            .or_else(|_| Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag)))?;

        match act {
            Action::NoArg(a) => a(self.context),
            Action::Repeatable(a) => a(self.context, 1),
            _ => Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag)),
        }
    }

    fn report_error(&mut self, error: Error<String>) {
        self.context.last_error_message = Some(format!("{}", error));
    }
}

impl Behavior for CommandParser<'_> {
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
