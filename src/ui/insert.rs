use chrono::NaiveDateTime;
use chrono_tz::Tz;
use nom::{
    branch::alt,
    bytes::complete::take_until1,
    character::complete::{alpha1, alphanumeric1, char, space0},
    combinator::all_consuming,
    error::*,
    multi::many1,
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use phf::phf_map;
use unsegen::input::*;

use super::command::ActionResult;
use super::context::Context;
use crate::config::Config;
use crate::provider::NewEvent;

type InsertAction = fn(&mut NewEvent<Tz>, &str) -> ActionResult;

const DATETIME_FORMAT: &'static str = "%Y-%m-%dT%H:%M";

const INSERT_ACTIONS: phf::Map<&'static str, InsertAction> = phf_map! {
    "title" => |b, v| {
        b.set_title(v);
        Ok(())
    },
    "description" => |b, v| {
        b.set_description(v);
        Ok(())
    },
    "begin" => |b, v| {
        let dt = NaiveDateTime::parse_from_str(v, DATETIME_FORMAT).or_else(|_| Err(nom::error::ParseError::from_error_kind(v.to_string(), nom::error::ErrorKind::Tag)))?;
        b.set_begin(dt);
        Ok(())
    },
    "duration" => |_b, _v| {
        // let duration = Duration::from_str(v)
        //     .or_else(|_| Err(ParseError::from_error_kind(v.into(), ErrorKind::Tag)))?;
        // b.set_duration(duration);
        Ok(())
    },
    "end" => |b, v| {
        let dt = NaiveDateTime::parse_from_str(v, DATETIME_FORMAT).or_else(|_| Err(nom::error::ParseError::from_error_kind(v.to_string(), nom::error::ErrorKind::Tag)))?;
        b.set_end(dt);
        Ok(())
    },
};

pub struct InsertParser<'a> {
    context: &'a mut Context,
    _config: &'a Config,
    new_event: Option<NewEvent<Tz>>,
}

impl<'a> InsertParser<'a> {
    pub fn new(context: &'a mut Context, config: &'a Config, new_event: NewEvent<Tz>) -> Self {
        InsertParser {
            context,
            _config: config,
            new_event: Some(new_event),
        }
    }

    fn parse_key_value(key_value: &str) -> IResult<&str, ((&str, &InsertAction), &str)> {
        let (rest, (key, value)) = separated_pair(
            preceded(space0, alpha1),
            char(':'),
            alt((
                delimited(char('"'), take_until1("\""), char('"')),
                take_until1(" "),
            )),
        )(key_value)?;

        let action = INSERT_ACTIONS.get(key);

        if let Some(action) = action {
            Ok((rest, ((key, action), value)))
        } else {
            Err(nom::Err::Error(nom::error::Error::from_error_kind(
                key,
                nom::error::ErrorKind::Tag,
            )))
        }
    }

    fn parse_line(&mut self, line: &str) -> Result<(), Error<String>> {
        let (rest, found_key_values) = many1(Self::parse_key_value)(line)
            .or_else(|_| Err(ParseError::from_error_kind(line.into(), ErrorKind::Many1)))?;

        for ((_, action), input) in found_key_values.into_iter() {
            action(self.new_event.as_mut().unwrap(), input)?;
        }

        let (_, name): (&str, &str) = all_consuming(delimited(space0, alphanumeric1, space0))(rest)
            .or_else(|_: nom::Err<nom::error::Error<&str>>| {
                Err(ParseError::from_error_kind(
                    rest.to_string(),
                    ErrorKind::Tag,
                ))
            })?;

        if let Some(calendar) = self.context.agenda_mut().calendar_by_name_mut(name) {
            calendar
                .add_event(self.new_event.take().unwrap())
                .or_else(|e| {
                    Err(Error::from_error_kind(
                        format!("Could not add event: {}", e),
                        ErrorKind::Fail,
                    ))
                })
        } else {
            Err(ParseError::from_error_kind(
                format!("Calendar '{}' not found", name),
                ErrorKind::Tag,
            ))
        }
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
                        log::error!("{}", e);
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
