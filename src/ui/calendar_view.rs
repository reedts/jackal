use crate::calendar::{Calendar, Day, Month};
use crate::cmds::{Cmd, Result};
use crate::control::Control;

use chrono::{Utc, Weekday};

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Text, Paragraph, Widget};

use crate::ui::Selection;

struct DayBlock<'a> {
    day: &'a Day<'a, Utc>,
    selected: bool
}

pub struct CalendarView<'a> {
    calendar: &'a Calendar<'a>,
    selected_month: &'a Month<'a, Utc>,
    day_blocks: Vec<DayBlock<'a>>,
    selected_day_idx: usize
}

impl<'a> CalendarView<'a> {
    pub fn new(calendar: &'a mut Calendar<'a>) -> CalendarView<'a> {
        let curr_month = calendar.curr_month();
        let curr_day = calendar.curr_day().day_num();
        let mut view = CalendarView {
            calendar,
            selected_month: curr_month,
            day_blocks: curr_month.days().iter()
                .map(|d| DayBlock { day: d, selected: false })
                .collect(),
            selected_day_idx: (curr_day - 1) as usize
        };

        view.day_blocks[view.selected_day_idx].select();

        view
    }

    fn selected_block(&self) -> &DayBlock<'a>{
        &self.day_blocks[self.selected_day_idx]
    }
    
    fn selected_block_mut(&mut self) -> &mut DayBlock<'a>{
        &mut self.day_blocks[self.selected_day_idx]
    }

    pub fn selected_day(&self) -> &Day<'a, Utc> {
        self.day_blocks[self.selected_day_idx].day()
    }

    fn checked_select_n_next(&mut self, n: usize) {
        self.selected_block_mut().unselect();
        self.selected_day_idx = if let Some(i) = self.selected_day_idx.checked_add(n) {
            if i < self.day_blocks.len() {
                i
            } else {
                self.selected_day_idx
            }
        } else {
            self.selected_day_idx
        };
        self.selected_block_mut().select();
    }
    
    fn checked_select_n_prev(&mut self, n: usize) {
        self.selected_block_mut().unselect();
        self.selected_day_idx = if let Some(i) = self.selected_day_idx.checked_sub(n) {
            i
        } else {
            self.selected_day_idx
        };
        self.selected_block_mut().select();
    }
}

impl<'a> Control for CalendarView<'a> {
    fn send_cmd(&mut self, cmd: Cmd) -> Result {
        match cmd {
            Cmd::NextDay => {
                self.move_right();
                Ok(Cmd::Noop)
            },
            Cmd::PrevDay => {
                self.move_left();
                Ok(Cmd::Noop)
            },
            Cmd::NextWeek => {
                self.move_down();
                Ok(Cmd::Noop)
            },
            Cmd::PrevWeek => {
                self.move_up();
                Ok(Cmd::Noop)
            }
            _ => Ok(cmd)
        }
    }
}

impl<'a> Selection for CalendarView<'a> {
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

impl<'a> DayBlock<'a> {
    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn day(&self) -> &Day<'a, Utc> {
        self.day
    }
}

impl<'a> Widget for DayBlock<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let style = match self.selected {
            true => Style::default().fg(Color::Red),
            false => Style::default()
        };

        Paragraph::new([Text::styled(format!("{}", self.day.day_num()), style)].iter())
            .alignment(Alignment::Right)
            .draw(area, buf);
    }
}

impl<'a> Widget for CalendarView<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        Block::default().borders(Borders::ALL)
            .title(&format!("{} {}", self.selected_month.name(), self.calendar.year().num()))
            .draw(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ].as_ref())
            .split(Rect {
                x: area.x + (area.width / 2) - 35 / 2,
                y: area.y + 2,
                width: 35,
                height: 30
            });
        
        let mut rows: Vec<Vec<Rect>> = rows.iter().map(|r| {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5)
                ].as_ref())
                .split(*r)
            }).collect();

        let header = [
            "Mon",
            "Tue",
            "Wed",
            "Thu",
            "Fri",
            "Sat",
            "Sun"
        ];

        let header_style = Style::default().fg(Color::Yellow);

        for (col, header) in rows.first_mut().unwrap().iter_mut().zip(header.iter()) {
            Paragraph::new([Text::styled(*header, header_style)].iter())
                .alignment(Alignment::Right)
                .draw(*col, buf);
        }
        
        let mut row: usize = 1;
        for day in self.day_blocks.iter_mut() {
            let col = day.day().weekday().num_days_from_monday() as usize;
            day.draw(rows[row][col], buf);

            // If day was 'Sunday' switch to next week
            if day.day().weekday() == Weekday::Sun {
                row += 1;
            }
        }
    }
}
