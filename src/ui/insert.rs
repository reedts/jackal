use std::str::FromStr;
use unsegen::input::*;

use nom::{
    branch::alt,
    bytes::complete::take_until1,
    character::{
        complete::{char, space1},
        is_space,
    },
    combinator::all_consuming,
    error::*,
    multi::{many1, separated_list1},
    sequence::{delimited, separated_pair, terminated},
    IResult,
};

use super::command::ActionResult;
use super::context::Context;
use super::match_action;
use crate::config::Config;
use crate::ical::{DateTime, Duration, EventBuilder};

type InsertAction = fn(&mut EventBuilder, &str) -> ActionResult;
const INSERT_ACTIONS: &'static [(&'static str, InsertAction)] = &[
    ("description", |b, v| {
        b.set_description(v.to_owned());
        Ok(())
    }),
    ("begin", |b, v| {
        let start = DateTime::from_str(v)
            .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        b.set_start(start);
        Ok(())
    }),
    ("duration", |b, v| {
        let duration = Duration::from_str(v)
            .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        b.set_duration(duration);
        Ok(())
    }),
    ("end", |b, v| {
        let end = DateTime::from_str(v)
            .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        b.set_end(end);
        Ok(())
    }),
    ("location", |b, v| {
        b.set_location(v.to_owned());
        Ok(())
    }),
];

pub struct InsertParser<'a, 'c> {
    context: &'a mut Context<'c>,
    config: &'a Config,
    builder: &'a mut EventBuilder,
}

impl<'a, 'c> InsertParser<'a, 'c> {
    pub fn new(
        context: &'a mut Context<'c>,
        config: &'a Config,
        builder: &'a mut EventBuilder,
    ) -> Self {
        InsertParser {
            context,
            config,
            builder,
        }
    }

    fn parse_key_value(key_value: &str) -> IResult<&str, ((&str, &InsertAction), &str)> {
        separated_pair(
            match_action(INSERT_ACTIONS),
            char(':'),
            alt((
                terminated(take_until1(" "), space1),
                delimited(char('"'), take_until1("\""), char('"')),
            )),
        )(key_value)
    }

    fn parse_line(&mut self, line: &str) -> Result<(), Error<String>> {
        let (rest, found_key_values) = many1(Self::parse_key_value)(line)
            .or_else(|_| Err(ParseError::from_error_kind(line.into(), ErrorKind::Many1)))?;

        Ok(())
    }
}

impl Behavior for InsertParser<'_, '_> {
    fn input(self, input: Input) -> Option<Input> {
        if let Event::Key(key) = input.event {
            match key {
                Key::Char('\n') => {
                    let line = self
                        .context
                        .input_sink_mut(super::Mode::Insert)
                        .finish_line()
                        .to_owned();
                    None
                }
                _ => Some(input),
            }
        } else {
            Some(input)
        }
    }
}
