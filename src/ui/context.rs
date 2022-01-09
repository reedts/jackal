use chrono::prelude::*;
use num_traits::FromPrimitive;

use crate::agenda::Agenda;

use unsegen::base::style::*;
use unsegen::widget::builtin::PromptLine;

#[derive(Clone, Debug)]
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

pub struct TuiContext {
    pub mode: Mode,
    pub theme: Theme,
    pub cursor: DateTime<Local>,
    pub command_line: PromptLine,
    pub eventlist_index: usize,
}

impl Default for TuiContext {
    fn default() -> Self {
        TuiContext {
            mode: Mode::Normal,
            theme: Theme::default(),
            cursor: Local::now(),
            command_line: PromptLine::with_prompt(":".to_owned()),
            eventlist_index: 0,
        }
    }
}

impl TuiContext {
    pub fn new(cursor: DateTime<Local>) -> Self {
        TuiContext {
            mode: Mode::Normal,
            theme: Theme::default(),
            cursor,
            command_line: PromptLine::with_prompt(":".to_owned()),
            eventlist_index: 0,
        }
    }
}

impl TuiContext {
    pub fn with_today(mut self) -> Self {
        self.select_today();
        self
    }

    pub fn select_today(&mut self) {
        self.cursor = Local::now();
    }

    pub fn selected_day(&self) -> u32 {
        self.cursor.day()
    }

    pub fn selected_month(&self) -> Month {
        Month::from_u32(self.cursor.month()).unwrap()
    }

    pub fn selected_year(&self) -> i32 {
        self.cursor.year()
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }
}

pub struct Context<'a> {
    tui_context: TuiContext,
    calendar: Agenda<'a>,
    now: DateTime<Local>,
}

impl<'a> Context<'a> {
    pub fn new<'b: 'a>(calendar: Agenda<'b>) -> Self {
        Context {
            tui_context: TuiContext::default(),
            calendar,
            now: Local::now(),
        }
    }

    pub fn tui_context(&self) -> &TuiContext {
        &self.tui_context
    }

    pub fn tui_context_mut(&mut self) -> &mut TuiContext {
        &mut self.tui_context
    }

    pub fn agenda(&self) -> &Agenda {
        &self.calendar
    }

    pub fn now(&self) -> &DateTime<Local> {
        &self.now
    }

    pub fn today(&self) -> Date<Local> {
        self.now.date()
    }

    pub fn cursor(&self) -> &DateTime<Local> {
        &self.tui_context.cursor
    }

    pub fn update(&mut self) {
        self.now = Local::now();
    }

    pub fn current_day(&self) -> u32 {
        self.now().day()
    }

    pub fn current_month(&self) -> Month {
        Month::from_u32(self.now().month()).unwrap()
    }

    pub fn current_year(&self) -> i32 {
        self.now().year()
    }
}
