use std::cell::RefCell;
use std::rc::Rc;
use crate::calendar::{Calendar, Day};
use crate::cmds::{Cmd, Result};
use crate::control::Control;

use chrono::{Utc, Weekday};

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{
    Block,
    Borders,
    Paragraph,
    StatefulWidget,
    Text,
    Widget
};

use crate::ui::Selection;

pub struct DayBlock<'a> {
    day: &'a Day<Utc>,
    selected: bool,
}

pub struct CalendarView<'a> {
    calendar: &'a Calendar,
}

pub struct CalendarViewState {
    calendar: Rc<RefCell<Calendar>>,
    month_idx: u32,
    day_idx: u32,
}

impl<'a> DayBlock<'a> {
    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn day(&self) -> &Day<Utc> {
        self.day
    }
}

impl<'a> Widget for DayBlock<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = match self.selected {
            true => Style::default().fg(Color::Red),
            false => Style::default(),
        };

        Paragraph::new([Text::styled(format!("{}", self.day.day_num()), style)].iter())
            .alignment(Alignment::Right)
            .render(area, buf);
    }
}


impl<'a> CalendarView<'a> {
    pub fn new(calendar: &'a Calendar) -> Self {
        CalendarView {
            calendar
        }
    }
}

impl<'a> StatefulWidget for CalendarView<'a> {
    type State = CalendarViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let day_idx        = state.day_idx();
        let month_idx      = state.month_idx();
        let selected_month = self.calendar.month_from_idx(month_idx).unwrap_or(self.calendar.curr_month());

        Block::default()
            .borders(Borders::ALL)
            .title(&format!(
                "{} {}",
                selected_month.name().name(),
                self.calendar.year().num()
            ))
            .render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(Rect {
                x: area.x + (area.width / 2) - 35 / 2,
                y: area.y + 2,
                width: 35,
                height: 30,
            });

        let mut rows: Vec<Vec<Rect>> = rows
            .iter()
            .map(|r| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Length(5),
                        ]
                        .as_ref(),
                    )
                    .split(*r)
            })
            .collect();

        let header = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        let header_style = Style::default().fg(Color::Yellow);

        for (col, header) in rows.first_mut().unwrap().iter_mut().zip(header.iter()) {
            Paragraph::new([Text::styled(*header, header_style)].iter())
                .alignment(Alignment::Right)
                .render(*col, buf);
        }

        let mut day_blocks: Vec<DayBlock> = self.calendar.month_from_idx(month_idx).unwrap().days().iter()
            .map(|day| DayBlock {day, selected: false})
            .collect();

        // Mark selected day
        day_blocks[day_idx as usize].select();

        let mut row: usize = 1;
        for day in day_blocks.drain(..) {
            let col = day.day().weekday().num_days_from_monday() as usize;
            let weekday = day.day().weekday();
            day.render(rows[row][col], buf);

            // If day was 'Sunday' switch to next week
            if weekday == Weekday::Sun {
                row += 1;
            }
        }
    }
}

impl CalendarViewState {
    pub fn new(calendar: Rc<RefCell<Calendar>>) -> Self {
        let curr_month = calendar.borrow().curr_month().ord();
        let curr_day = calendar.borrow().curr_day().day_num();

        CalendarViewState {
            calendar,
            month_idx: curr_month,
            day_idx:   curr_day
        }
    }

    fn checked_select_n_next(&mut self, n: u32) {
        self.day_idx = if let Some(i) = self.day_idx.checked_add(n) {
            if i < self.calendar.borrow().month_from_idx(self.month_idx).unwrap().days().len() as u32 {
                i
            } else {
                self.day_idx
            }
        } else {
            self.day_idx
        };
    }

    fn checked_select_n_prev(&mut self, n: u32) {
        self.day_idx = if let Some(i) = self.day_idx.checked_sub(n) {
            i
        } else {
            self.day_idx
        };
    }

    pub fn day_idx(&self) -> u32 {
        self.day_idx
    }

    pub fn month_idx(&self) -> u32 {
        self.month_idx
    }
}

impl Control for CalendarViewState {
    fn send_cmd(&mut self, cmd: Cmd) -> Result {
        match cmd {
            Cmd::NextDay => {
                self.move_right();
                Ok(Cmd::Noop)
            }
            Cmd::PrevDay => {
                self.move_left();
                Ok(Cmd::Noop)
            }
            Cmd::NextWeek => {
                self.move_down();
                Ok(Cmd::Noop)
            }
            Cmd::PrevWeek => {
                self.move_up();
                Ok(Cmd::Noop)
            }
            _ => Ok(cmd),
        }
    }
}

impl Selection for CalendarViewState {
    fn move_left(&mut self) {
        self.checked_select_n_prev(1);
    }

    fn move_right(&mut self) {
        self.checked_select_n_next(1);
    }

    fn move_up(&mut self) {
        self.checked_select_n_prev(7);
    }

    fn move_down(&mut self) {
        self.checked_select_n_next(7);
    }

    fn move_n_left(&mut self, n: u32) {
        self.checked_select_n_prev(n);
    }

    fn move_n_right(&mut self, n: u32) {
        self.checked_select_n_next(n);
    }

    fn move_n_up(&mut self, n: u32) {
        self.checked_select_n_prev(n * 7);
    }

    fn move_n_down(&mut self, n: u32) {
        self.checked_select_n_next(n * 7);
    }
}

