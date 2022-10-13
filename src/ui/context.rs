use chrono::prelude::*;
use std::collections::BTreeMap;

use crate::agenda::Agenda;

use unsegen::base::style::*;
use unsegen::widget::builtin::PromptLine;

#[derive(Clone, Copy, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub day_style: StyleModifier,
    pub day_text_style: TextFormatModifier,
    pub focus_day_style: StyleModifier,
    pub focus_day_text_style: TextFormatModifier,
    pub focus_day_char: Option<char>,
    pub today_day_style: StyleModifier,
    pub today_day_text_style: TextFormatModifier,
    pub today_day_char: Option<char>,
    pub month_header_style: StyleModifier,
    pub month_header_text_style: TextFormatModifier,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            day_style: StyleModifier::default(),
            day_text_style: TextFormatModifier::default(),
            focus_day_style: StyleModifier::default().bg_color(Color::Blue),
            focus_day_text_style: TextFormatModifier::default(),
            focus_day_char: None,
            today_day_style: StyleModifier::default().invert(true),
            today_day_text_style: TextFormatModifier::default().italic(true),
            today_day_char: Some('*'),
            month_header_style: StyleModifier::default().fg_color(Color::Yellow),
            month_header_text_style: TextFormatModifier::default(),
        }
    }
}

pub struct Context {
    pub mode: Mode,
    pub theme: Theme,
    pub cursor: DateTime<Local>,
    pub eventlist_index: usize,
    pub last_error_message: Option<String>,
    input_sinks: BTreeMap<Mode, PromptLine>,
    agenda: Agenda,
    now: DateTime<Local>,
}

impl Context {
    pub fn new(calendar: Agenda) -> Self {
        Context {
            mode: Mode::Normal,
            theme: Theme::default(),
            cursor: Local::now(),
            last_error_message: None,
            input_sinks: BTreeMap::from([
                (Mode::Insert, PromptLine::with_prompt(">".to_owned())),
                (Mode::Command, PromptLine::with_prompt(":".to_owned())),
            ]),
            eventlist_index: 0,
            agenda: calendar,
            now: Local::now(),
        }
    }

    pub fn input_sink(&self, mode: Mode) -> &PromptLine {
        self.input_sinks.get(&mode).unwrap()
    }
    pub fn input_sink_mut(&mut self, mode: Mode) -> &mut PromptLine {
        self.input_sinks.get_mut(&mode).unwrap()
    }

    pub fn agenda(&self) -> &Agenda {
        &self.agenda
    }

    pub fn agenda_mut(&mut self) -> &mut Agenda {
        &mut self.agenda
    }

    pub fn now(&self) -> &DateTime<Local> {
        &self.now
    }

    pub fn today(&self) -> Date<Local> {
        self.now.date()
    }

    pub fn cursor(&self) -> &DateTime<Local> {
        &self.cursor
    }

    pub fn update(&mut self) {
        self.now = Local::now();
    }
}
