use crate::calendar::{EventsOfDay, Month};
use crate::ctx::Context;

use std::convert::{From, Into};

use chrono::{Datelike, FixedOffset, NaiveDate, TimeZone, Weekday};

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, Widget};

pub struct DayCell {
    day_num: u8,
    selected: bool,
    is_today: bool,
    style: Style,
    focus_style: Style,
    today_style: Style,
    focus_symbol: Option<char>,
    today_symbol: Option<char>,
}

pub struct MonthView {
    month: Month,
    year: i32,
    selected: bool,
    header_style: Style,
    header_focus_style: Style,
    header_bottom_margin: u16,
    label_style: Style,
    label_focus_style: Style,
    cell_style: Style,
    cell_focus_style: Style,
    cell_today_style: Style,
    today_symbol: Option<char>,
    focus_symbol: Option<char>,
    horizontal_padding: u16,
    vertical_padding: u16,
}

pub struct CalendarView {
    header_style: Style,
}

impl DayCell {
    pub fn new(day_num: u8) -> Self {
        DayCell {
            day_num,
            selected: false,
            is_today: false,
            style: Style::default(),
            focus_style: Style::default().fg(Color::Red),
            today_style: Style::default(),
            focus_symbol: None,
            today_symbol: None,
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

    pub fn today_style(mut self, style: Style) -> Self {
        self.today_style = style;
        self
    }

    pub fn focus_symbol(mut self, symbol: char) -> Self {
        self.focus_symbol = Some(symbol);
        self
    }

    pub fn focus_symbol_opt(mut self, symbol_opt: Option<char>) -> Self {
        self.focus_symbol = symbol_opt;
        self
    }

    pub fn today_symbol(mut self, symbol: char) -> Self {
        self.today_symbol = Some(symbol);
        self
    }

    pub fn today_symbol_opt(mut self, symbol_opt: Option<char>) -> Self {
        self.today_symbol = symbol_opt;
        self
    }

    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn is_today(&mut self, is_today: bool) {
        self.is_today = is_today;
    }

    pub fn today(mut self, is_today: bool) -> Self {
        self.is_today(is_today);
        self
    }

    pub fn day_num(&self) -> u8 {
        self.day_num
    }
}

impl<'a> Into<Cell<'a>> for DayCell {
    fn into(self) -> Cell<'a> {
        let spans = Text::from(vec![
            if self.is_today {
                Spans::from(vec![
                    if let Some(symbol) = self.today_symbol {
                        Span::styled(symbol.to_string(), self.today_style)
                    } else {
                        Span::from("")
                    },
                    Span::styled(self.day_num.to_string(), self.today_style),
                ])
            } else {
                Spans::from(Span::styled(self.day_num.to_string(), self.style))
            },
            if self.selected {
                if let Some(symbol) = self.focus_symbol {
                    Spans::from(Span::styled(symbol.to_string(), self.focus_style))
                } else {
                    Spans::from(Span::from(""))
                }
            } else {
                Spans::from(Span::from(""))
            },
        ]);

        Cell::from(spans).style(if self.selected {
            self.focus_style
        } else {
            self.style
        })
    }
}

impl MonthView {
    pub fn new(month: Month, year: i32) -> Self {
        MonthView {
            month,
            year,
            selected: false,
            header_style: Style::default().fg(Color::Yellow),
            header_focus_style: Style::default().fg(Color::Yellow),
            header_bottom_margin: 1,
            label_style: Style::default(),
            label_focus_style: Style::default(),
            cell_style: Style::default(),
            cell_focus_style: Style::default().bg(Color::Blue),
            cell_today_style: Style::default(),
            today_symbol: Some('*'),
            focus_symbol: None,
            horizontal_padding: 10,
            vertical_padding: 5,
        }
    }

    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    pub fn header_focus_style(mut self, style: Style) -> Self {
        self.header_focus_style = style;
        self
    }

    pub fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    pub fn label_focus_style(mut self, style: Style) -> Self {
        self.label_focus_style = style;
        self
    }

    pub fn cell_style(mut self, style: Style) -> Self {
        self.cell_style = style;
        self
    }

    pub fn cell_focus_style(mut self, style: Style) -> Self {
        self.cell_focus_style = style;
        self
    }

    pub fn cell_today_style(mut self, style: Style) -> Self {
        self.cell_today_style = style;
        self
    }

    pub fn today_symbol(mut self, symbol: char) -> Self {
        self.today_symbol = Some(symbol);
        self
    }

    pub fn no_today_symbol(mut self) -> Self {
        self.today_symbol = None;
        self
    }

    pub fn focus_symbol(mut self, symbol: char) -> Self {
        self.focus_symbol = Some(symbol);
        self
    }

    pub fn no_focus_symbol(mut self) -> Self {
        self.focus_symbol = None;
        self
    }

    pub fn horizontal_padding(mut self, padding: u16) -> Self {
        self.horizontal_padding = padding;
        self
    }

    pub fn vertical_padding(mut self, padding: u16) -> Self {
        self.vertical_padding = padding;
        self
    }
}

impl StatefulWidget for MonthView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let header = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

        let sel_day = state.cursor.day0();
        let sel_month = Month::from(state.cursor.month0());
        let sel_year = state.cursor.year();
        let tz = FixedOffset::from_offset(state.cursor.offset());

        Block::default()
            .borders(Borders::NONE)
            .title(Span::styled(
                format!("{} {}", self.month.name(), self.year),
                if self.selected {
                    self.label_focus_style
                } else {
                    self.label_style
                },
            ))
            .render(area, buf);

        let offset = NaiveDate::from_ymd(self.year, self.month.ord(), 1)
            .weekday()
            .num_days_from_monday() as usize;

        let mut cells: Vec<DayCell> = (1..self.month.days(self.year) as usize)
            .map(|day_num| {
                DayCell::new(day_num as u8)
                    .style(self.cell_style)
                    .focus_style(self.cell_focus_style)
                    .focus_symbol_opt(self.focus_symbol)
                    .today_symbol_opt(self.today_symbol)
            })
            .collect();

        if self.month == sel_month {
            cells[sel_day as usize].select();
        }

        let cur_day = state.now.day0();
        let cur_month = Month::from(state.now.month());
        let cur_year = state.now.year();
        if cur_month == self.month && cur_year == self.year {
            cells[cur_day as usize].is_today(true);
        }

        let rows: Vec<Row> = std::iter::repeat_with(|| Cell::from(""))
            .take(offset)
            .chain(cells.drain(..).map(|day_cell| day_cell.into()))
            .collect::<Vec<Cell<'_>>>()
            .chunks(7)
            .map(|row| Row::new(row.to_vec()))
            .collect();

        Widget::render(
            Table::new(rows).header(Row::new(header.to_vec())),
            area,
            buf,
        );

        // prepend padding for input

        // let rows = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints(
        //         [
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //             Constraint::Length(1),
        //         ]
        //         .as_ref(),
        //     )
        //     .split(Rect {
        //         x: area.x + (area.width / 2) - 35 / 2,
        //         y: area.y + 2,
        //         width: 35,
        //         height: 30,
        //     });

        // let mut rows: Vec<Vec<Rect>> = rows
        //     .iter()
        //     .map(|r| {
        //         Layout::default()
        //             .direction(Direction::Horizontal)
        //             .constraints(
        //                 [
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                     Constraint::Length(5),
        //                 ]
        //                 .as_ref(),
        //             )
        //             .split(*r)
        //     })
        //     .collect();

        // for (col, header) in rows.first_mut().unwrap().iter_mut().zip(header.iter()) {
        //     Paragraph::new(Text::styled(*header, self.header_style))
        //         .alignment(Alignment::Right)
        //         .render(*col, buf);
        // }

        // let mut day_blocks: Vec<DayBlock> = (1..month.days(year) as u32)
        //     .map(|day| DayBlock::new(day))
        //     .collect();

        // Mark selected day
        // day_blocks[(day - 1) as usize].select();

        // let mut row: usize = 1;
        // for day in day_blocks.drain(..) {
        //     let col = day.day().date().weekday().num_days_from_monday() as usize;
        //     let weekday = day.day().date().weekday();
        //     day.render(rows[row][col], buf);

        //     // If day was 'Sunday' switch to next week
        //     if weekday == Weekday::Sun {
        //         row += 1;
        //     }
        // }
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

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {}
}
