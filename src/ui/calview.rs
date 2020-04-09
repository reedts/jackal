use std::cell::RefCell;
use std::rc::Rc;
use crate::calendar::{Calendar, Day};
use crate::cmds::{Cmd, Result};
use crate::control::Control;

use chrono::{Utc, Weekday};

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Text, Widget};

use crate::ui::Selection;

pub struct DayBlock<'a> {
    day: &'a Day<Utc>,
    selected: bool,
}

pub struct CalendarView {
    calendar: Rc<RefCell<Calendar>>,
    selected_month_idx: usize,
    selected_day_idx: usize,
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

impl CalendarView {
    pub fn new(calendar: Rc<RefCell<Calendar>>) -> Self {
        let curr_month = calendar.borrow().curr_month().ord();
        let curr_day = calendar.borrow().curr_day().day_num();

        CalendarView {
            calendar: calendar.clone(),
            selected_month_idx: curr_month,
            selected_day_idx: (curr_day - 1) as usize,
        }
    }

    // pub fn selected_month(&self) -> &Month<Utc> {
    //     self.calendar.borrow().month_from_idx(self.selected_month_idx).unwrap()
    // }

    // pub fn selected_day(&self) -> &Day<Utc> {
    //     &self.calendar.borrow().month_from_idx(self.selected_month_idx).unwrap().days()[self.selected_day_idx]
    // }

    fn checked_select_n_next(&mut self, n: usize) {
        self.selected_day_idx = if let Some(i) = self.selected_day_idx.checked_add(n) {
            if i < self.calendar.borrow().month_from_idx(self.selected_month_idx).unwrap().days().len() {
                i
            } else {
                self.selected_day_idx
            }
        } else {
            self.selected_day_idx
        };
    }

    fn checked_select_n_prev(&mut self, n: usize) {
        self.selected_day_idx = if let Some(i) = self.selected_day_idx.checked_sub(n) {
            i
        } else {
            self.selected_day_idx
        };
    }
}

impl Control for CalendarView {
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

impl Selection for CalendarView {
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

    fn move_n_left(&mut self, n: usize) {
        self.checked_select_n_prev(n);
    }

    fn move_n_right(&mut self, n: usize) {
        self.checked_select_n_next(n);
    }

    fn move_n_up(&mut self, n: usize) {
        self.checked_select_n_prev(n * 7);
    }

    fn move_n_down(&mut self, n: usize) {
        self.checked_select_n_next(n * 7);
    }
}

impl<'a> Widget for DayBlock<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let style = match self.selected {
            true => Style::default().fg(Color::Red),
            false => Style::default(),
        };

        Paragraph::new([Text::styled(format!("{}", self.day.day_num()), style)].iter())
            .alignment(Alignment::Right)
            .draw(area, buf);
    }
}

impl Widget for CalendarView {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let cal = self.calendar.borrow();
        let selected_month = cal.month_from_idx(self.selected_month_idx).unwrap_or(cal.curr_month());
        Block::default()
            .borders(Borders::ALL)
            .title(&format!(
                "{} {}",
                selected_month.name().name(),
                cal.year().num()
            ))
            .draw(area, buf);

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
                .draw(*col, buf);
        }

        let mut day_blocks: Vec<DayBlock> = cal.month_from_idx(self.selected_month_idx).unwrap().days().iter()
            .map(|day| DayBlock {day, selected: false})
            .collect();

        day_blocks[self.selected_day_idx].select();

        let mut row: usize = 1;
        for day in day_blocks.iter_mut() {
            let col = day.day().weekday().num_days_from_monday() as usize;
            day.draw(rows[row][col], buf);

            // If day was 'Sunday' switch to next week
            if day.day().weekday() == Weekday::Sun {
                row += 1;
            }
        }
    }
}
