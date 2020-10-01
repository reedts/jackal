use crate::calendar::Day;
use crate::cmds::{Cmd, Result};
use crate::control::Control;
use crate::context::Context;

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

pub struct CalendarView {
}

pub struct CalendarViewState {
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


impl CalendarView {
    pub fn default() -> Self {
        CalendarView {
        }
    }
}

impl StatefulWidget for CalendarView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let day_idx        = state.selected_day_idx;
        let month_idx      = state.selected_month_idx;
        let selected_month = state.calendar.month_from_idx(month_idx).unwrap_or(state.calendar.curr_month());

        Block::default()
            .borders(Borders::ALL)
            .title(&format!(
                "{} {}",
                selected_month.name().name(),
                state.calendar.year().num()
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

        let mut day_blocks: Vec<DayBlock> = state.calendar.month_from_idx(month_idx).unwrap().days().iter()
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
    pub fn default() -> Self {
        CalendarViewState {
        }
    }

    fn checked_select_n_next(&mut self, n: u32, context: &mut Context) {
        context.selected_day_idx = if let Some(i) = context.selected_day_idx.checked_add(n) {
            if i < context.calendar.month_from_idx(context.selected_month_idx).unwrap().days().len() as u32 {
                i
            } else {
                context.selected_day_idx
            }
        } else {
            context.selected_day_idx
        };
    }

    fn checked_select_n_prev(&mut self, n: u32, context: &mut Context) {
        context.selected_day_idx = if let Some(i) = context.selected_day_idx.checked_sub(n) {
            i
        } else {
            context.selected_day_idx
        };
    }
}

impl Control for CalendarViewState {
    fn send_cmd(&mut self, cmd: Cmd, context: &mut Context) -> Result {
        match cmd {
            Cmd::NextDay => {
                self.move_right(context);
                Ok(Cmd::Noop)
            }
            Cmd::PrevDay => {
                self.move_left(context);
                Ok(Cmd::Noop)
            }
            Cmd::NextWeek => {
                self.move_down(context);
                Ok(Cmd::Noop)
            }
            Cmd::PrevWeek => {
                self.move_up(context);
                Ok(Cmd::Noop)
            }
            _ => Ok(cmd),
        }
    }
}

impl Selection for CalendarViewState {
    fn move_left(&mut self, context: &mut Context) {
        self.checked_select_n_prev(1, context);
    }

    fn move_right(&mut self, context: &mut Context) {
        self.checked_select_n_next(1, context);
    }

    fn move_up(&mut self, context: &mut Context) {
        self.checked_select_n_prev(7, context);
    }

    fn move_down(&mut self, context: &mut Context) {
        self.checked_select_n_next(7, context);
    }

    fn move_n_left(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_prev(n, context);
    }

    fn move_n_right(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_next(n, context);
    }

    fn move_n_up(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_prev(n * 7, context);
    }

    fn move_n_down(&mut self, n: u32, context: &mut Context) {
        self.checked_select_n_next(n * 7, context);
    }
}

