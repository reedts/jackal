use crate::calendar::Day;
use crate::ctx::Context;

use chrono::{Datelike, FixedOffset, Weekday};

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};

pub struct DayBlock<'a> {
    day_num: u32,
    selected: bool,
    day: Day<'a, FixedOffset>,
    style: Style,
    focus_style: Style,
}

pub struct CalendarView {
    header_style: Style,
}

impl<'a> DayBlock<'a> {
    pub fn new(day_num: u32, day: Day<'a, FixedOffset>) -> Self {
        DayBlock {
            day_num,
            selected: false,
            day,
            style: Style::default(),
            focus_style: Style::default().fg(Color::Red),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus_style = style;
        self
    }

    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn day(&self) -> &Day<'a, FixedOffset> {
        &self.day
    }
}

impl<'a> Widget for DayBlock<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = match self.selected {
            true => self.focus_style,
            false => self.style,
        };

        Paragraph::new(Text::styled(format!("{}", self.day_num), style))
            .alignment(Alignment::Right)
            .render(area, buf);
    }
}

impl Default for CalendarView {
    fn default() -> Self {
        CalendarView {
            header_style: Style::default().fg(Color::Yellow),
        }
    }
}

impl CalendarView {
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }
}

impl StatefulWidget for CalendarView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let day = state.calendar_context.day;
        let month = state.calendar_context.month;
        let year = state.calendar_context.year;

        Block::default()
            .borders(Borders::ALL)
            .title(format!("{} {}", month.name(), year))
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

        for (col, header) in rows.first_mut().unwrap().iter_mut().zip(header.iter()) {
            Paragraph::new(Text::styled(*header, self.header_style))
                .alignment(Alignment::Right)
                .render(*col, buf);
        }

        let mut day_blocks: Vec<DayBlock> = (1..month.days(year) as u32)
            .map(|day| DayBlock::new(day, state.calendar.events_of_day(day, month, year)))
            .collect();

        // Mark selected day
        day_blocks[(day - 1) as usize].select();

        let mut row: usize = 1;
        for day in day_blocks.drain(..) {
            let col = day.day().date().weekday().num_days_from_monday() as usize;
            let weekday = day.day().date().weekday();
            day.render(rows[row][col], buf);

            // If day was 'Sunday' switch to next week
            if weekday == Weekday::Sun {
                row += 1;
            }
        }
    }
}
