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

pub struct CommandParser<'a, 'c> {
    context: &'a mut Context<'c>,
    config: &'a Config,
}

fn match_one_of<'a, 's, T: ?Sized, Act: 's>(c: &'a T) -> impl Fn(&str) -> IResult<&str, &str> + 'a
where
    &'a T: IntoIterator<Item = &'s (&'s str, Act)>,
{
    move |input| {
        for (name, _) in c {
            match all_consuming(tag(name.as_bytes()))(input) {
                done @ IResult::Ok(..) => return done,
                _ => (),
            }
        }

        Err(Err::Failure(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )))
    }
}

impl<'a, 'c> CommandParser<'a, 'c> {
    pub fn new(context: &'a mut Context<'c>, config: &'a Config) -> Self {
        CommandParser { context, config }
    }

    fn run_repeatable_command(&mut self, repeat: &str, cmd: &str) -> Result<(), Error<String>> {
        let repeats = if repeat.is_empty() {
            1
        } else {
            u32::from_str_radix(repeat, 10)
                .or_else::<Error<String>, _>(|_| {
                    return Err(ParseError::from_error_kind(repeat.into(), ErrorKind::Digit));
                })
                .unwrap()
        };

        let (_, act) = COMMANDS
            .iter()
            .find(|(c, _)| c == &cmd)
            .ok_or(ParseError::from_error_kind(cmd.into(), ErrorKind::OneOf))?;

        if let Action::Repeatable(a) = act {
            a(self.context, repeats)
        } else {
            Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag))
        }
    }

    fn run_nonargs_command(&mut self, cmd: &str) -> Result<(), Error<String>> {
        let (_, act) = COMMANDS
            .iter()
            .find(|(c, _)| c == &cmd)
            .ok_or(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag))?;

        match act {
            Action::NoArg(a) => a(self.context),
            Action::Repeatable(a) => a(self.context, 1),
            _ => Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag)),
        }
    }

    fn run_arg_command(&mut self, cmd: &str, arg: &str) -> Result<(), Error<String>> {
        let (_, act) = COMMANDS
            .iter()
            .find(|(c, _)| c == &cmd)
            .ok_or(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag))?;

        if let Action::Arg(a) = act {
            a(self.context, arg.to_owned())
        } else {
            Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag))
        }
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<(), Error<String>> {
        let res = all_consuming(tuple((digit1, Self::parse_command_code)))(cmd);

        if let Ok((_, (repeats, code))) = res {
            return self.run_repeatable_command(repeats, code);
        };

        let res = all_consuming(separated_pair(Self::parse_command_code, space1, rest))(cmd);

        if let Ok((_, (code, arg))) = res {
            return self.run_arg_command(code, arg);
        }

        let (_, cmd) = all_consuming(Self::parse_command_code)(cmd)
            .or_else(|_| Err(ParseError::from_error_kind(cmd.into(), ErrorKind::Tag)))?;

        self.run_nonargs_command(cmd)
    }

    fn report_error(&mut self, error: Error<String>) {
        self.context.last_error_message = Some(format!("{}", error));
    }

    fn parse_command_code(input: &str) -> IResult<&str, &str> {
        match_one_of(COMMANDS)(input)
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
