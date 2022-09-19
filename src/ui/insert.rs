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

use chrono::Duration;

use super::command::ActionResult;
use super::context::Context;
use super::match_action;
use crate::config::Config;
use crate::provider::ical::EventBuilder;
use crate::provider::ical::calendar::IcalDateTime;

type InsertAction = fn(&mut EventBuilder, &str) -> ActionResult;
const INSERT_ACTIONS: &'static [(&'static str, InsertAction)] = &[
    ("description", |b, v| {
        b.set_description(v.to_owned());
        Ok(())
    }),
    ("begin", |b, v| {
        // let start = IcalDateTime::from_str(v)
        //     .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        // b.set_start(start);
        Ok(())
    }),
    ("duration", |b, v| {
        // let duration = Duration::from_str(v)
        //     .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        // b.set_duration(duration);
        Ok(())
    }),
    ("end", |b, v| {
        // let end = IcalDateTime::from_str(v)
        //     .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        // b.set_end(end);
        Ok(())
    }),
    ("location", |b, v| {
        b.set_location(v.to_owned());
        Ok(())
    }),
];

pub struct InsertParser<'a> {
    context: &'a mut Context,
    config: &'a Config,
    builder: EventBuilder,
}

impl<'a> InsertParser<'a> {
    pub fn new(context: &'a mut Context, config: &'a Config, builder: EventBuilder) -> Self {
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

impl Behavior for InsertParser<'_> {
    fn input(mut self, input: Input) -> Option<Input> {
        if let Event::Key(key) = input.event {
            match key {
                Key::Char('\n') => {
                    let line = self
                        .context
                        .input_sink_mut(super::Mode::Insert)
                        .finish_line()
                        .to_owned();

                let res = self.parse_line(&line);
                    if let Err(e) = res {
                        self.context.last_error_message = Some(format!("{}", e));
                    } else {
                        let event = self.builder.finish();
                        // actually write & save event
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
