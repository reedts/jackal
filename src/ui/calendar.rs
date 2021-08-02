use chrono::prelude::*;
use num_traits::FromPrimitive;
use std::cmp::Ordering;
use std::convert::{From, Into};
use std::ops::{Add, Sub};
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, Widget};

use crate::calendar;
use crate::context::Context;
use crate::ui::{util, EstimatedWidgetSize, WidgetSize};

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

pub struct MonthView {
    month: Month,
    year: i32,
    selected: bool,
    header_style: Style,
    header_focus_style: Style,
    label_style: Style,
    label_focus_style: Style,
    cell_style: Style,
    cell_focus_style: Style,
    cell_today_style: Style,
    today_symbol: Option<char>,
    focus_symbol: Option<char>,
}

impl MonthView {
    const COLUMNS: u16 = 7;
    const ROWS: u16 = 6;
    const LABEL_ROWS: u16 = 1;

    pub fn new(month: Month, year: i32) -> Self {
        MonthView {
            month,
            year,
            selected: false,
            header_style: Style::default().fg(Color::Yellow),
            header_focus_style: Style::default().fg(Color::Yellow),
            label_style: Style::default(),
            label_focus_style: Style::default(),
            cell_style: Style::default(),
            cell_focus_style: Style::default().bg(Color::Blue),
            cell_today_style: Style::default(),
            today_symbol: Some('*'),
            focus_symbol: None,
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
}

impl From<MonthIndex> for MonthView {
    fn from(v: MonthIndex) -> Self {
        Self::new(v.index, v.year)
    }
}

impl StatefulWidget for MonthView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let header = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let centered_area = area; //util::center_in(&self, &area).unwrap_or(area);

        let sel_day = state.cursor.day0();
        let sel_month = Month::from_u32(state.cursor.month()).unwrap();
        let sel_year = state.cursor.year();
        let tz = FixedOffset::from_offset(state.cursor.offset());

        // set styles
        let (label_style, header_style) = if self.selected {
            (self.label_focus_style, self.header_focus_style)
        } else {
            (self.label_style, self.header_style)
        };

        let offset = NaiveDate::from_ymd(self.year, self.month.number_from_month(), 1)
            .weekday()
            .num_days_from_monday() as usize;

        let mut cells: Vec<DayCell> = (1..(calendar::days_of_month(&self.month, self.year) + 1)
            as usize)
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

        let cur_day = state.now().day0();
        let cur_month = state.now().month();
        let cur_year = state.now().year();
        if cur_month == self.month.number_from_month() && cur_year == self.year {
            cells[cur_day as usize].is_today(true);
        }

        let rows: Vec<Row> = std::iter::repeat_with(|| Cell::from(""))
            .take(offset)
            .chain(cells.drain(..).map(|day_cell| day_cell.into()))
            .collect::<Vec<Cell<'_>>>()
            .chunks(7)
            .map(|row| Row::new(row.to_vec()))
            .collect();

        Block::default()
            .borders(Borders::NONE)
            .title(Span::styled(
                format!("{} {}", self.month.name(), self.year),
                label_style,
            ))
            .render(centered_area, buf);

        Widget::render(
            Table::new(rows)
                .header(Row::new(header.to_vec()).style(header_style))
                .widths(&[
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(5),
                ]),
            Rect::new(
                centered_area.x,
                centered_area.y + Self::LABEL_ROWS,
                centered_area.width,
                centered_area.height,
            ),
            buf,
        );
    }
}

impl WidgetSize for MonthView {
    fn width(&self) -> u16 {
        // 7 days, length of 6 + column spacing of 1
        MonthView::COLUMNS * 6
    }

    fn height(&self) -> u16 {
        MonthView::ROWS + MonthView::LABEL_ROWS
    }
}

impl EstimatedWidgetSize for MonthView {
    fn est_width() -> u16 {
        MonthView::COLUMNS * 6
    }

    fn est_height() -> u16 {
        MonthView::ROWS + MonthView::LABEL_ROWS
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthIndex {
    index: Month,
    year: i32,
}

impl MonthIndex {
    pub fn new(index: Month, year: i32) -> Self {
        MonthIndex { index, year }
    }

    pub fn next(&self) -> Self {
        let next_month = self.index.succ();

        MonthIndex {
            index: next_month,
            year: if next_month.number_from_month() == 1 {
                self.year + 1
            } else {
                self.year
            },
        }
    }

    pub fn prev(&self) -> Self {
        let prev_month = self.index.succ();

        MonthIndex {
            index: prev_month,
            year: if prev_month.number_from_month() == 12 {
                self.year - 1
            } else {
                self.year
            },
        }
    }
}

impl Default for MonthIndex {
    fn default() -> Self {
        MonthIndex {
            index: Month::from_u32(Local::now().month()).unwrap_or(Month::January),
            year: Local::now().year(),
        }
    }
}

impl<T: Datelike> From<T> for MonthIndex {
    fn from(m: T) -> Self {
        MonthIndex::new(Month::from_u32(m.month()).unwrap(), m.year())
    }
}

impl Add<u32> for MonthIndex {
    type Output = MonthIndex;
    fn add(self, rhs: u32) -> Self::Output {
        let month_sum = self.index.number_from_month() + rhs;
        if month_sum <= 12 {
            MonthIndex {
                index: Month::from_u32(month_sum).unwrap(),
                year: self.year,
            }
        } else {
            let year_diff = month_sum / 12;
            let new_month = month_sum % 12;

            MonthIndex {
                index: Month::from_u32(new_month).unwrap(),
                year: self.year + year_diff as i32,
            }
        }
    }
}

impl Sub<u32> for MonthIndex {
    type Output = MonthIndex;
    fn sub(self, rhs: u32) -> Self::Output {
        let month_number = self.index.number_from_month();
        if rhs < month_number {
            MonthIndex {
                index: Month::from_u32(month_number - rhs).unwrap(),
                year: self.year,
            }
        } else if rhs == month_number {
            MonthIndex {
                index: Month::December,
                year: self.year - 1,
            }
        } else {
            let month_diff = month_number as i32 - rhs as i32;
            let new_month = month_diff.rem_euclid(12);
            let year_diff = month_diff.abs() / 12;

            MonthIndex {
                index: Month::from_u32(new_month as u32).unwrap(),
                year: self.year - year_diff as i32,
            }
        }
    }
}

impl PartialOrd for MonthIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.year != other.year {
            self.year.partial_cmp(&other.year)
        } else {
            self.index
                .number_from_month()
                .partial_cmp(&other.index.number_from_month())
        }
    }
}

#[derive(Clone)]
pub struct CalendarViewState {
    offset: MonthIndex,
    scrolloff: u32,
}

impl Default for CalendarViewState {
    fn default() -> Self {
        CalendarViewState {
            offset: Local::today().naive_local().into(),
            scrolloff: 1,
        }
    }
}

impl CalendarViewState {
    pub fn new<T>(selected: T, scrolloff: u32) -> Self
    where
        MonthIndex: From<T>,
    {
        CalendarViewState {
            offset: MonthIndex::from(selected.into()),
            scrolloff,
        }
    }

    pub fn select<T>(&mut self, idx: T, max: u32)
    where
        MonthIndex: From<T>,
    {
        let m_idx = MonthIndex::from(idx);

        if m_idx >= ((self.offset + max) - self.scrolloff) {
            self.offset = self.offset + self.scrolloff;
        } else if m_idx < self.offset + self.scrolloff {
            self.offset = m_idx - self.scrolloff;
        }
    }
}

pub struct CalendarView {
    header_style: Style,
    horizontal_padding: u16,
    vertical_padding: u16,
    month_spacing: u16,
}

impl Default for CalendarView {
    fn default() -> Self {
        CalendarView {
            header_style: Style::default().fg(Color::Yellow),
            horizontal_padding: 2,
            vertical_padding: 2,
            month_spacing: 1,
        }
    }
}

impl CalendarView {
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
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

    pub fn month_spacing(mut self, spacing: u16) -> Self {
        self.month_spacing = spacing;
        self
    }
}

impl StatefulWidget for CalendarView {
    type State = Context;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let padded_area = Rect::new(
            area.x + self.horizontal_padding,
            area.y + self.vertical_padding,
            area.width - (2 * self.horizontal_padding),
            area.height - (2 * self.vertical_padding),
        );

        let num_fitting_months = util::estimate_num_fits::<MonthView>(
            Direction::Vertical,
            &padded_area,
            Some(self.month_spacing),
        );

        let month_views = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                std::iter::repeat(Constraint::Length(
                    MonthView::est_height() + self.month_spacing,
                ))
                .take(num_fitting_months as usize)
                .collect::<Vec<_>>()
                .as_ref(),
            )
            .split(padded_area);

        state
            .calendarview_context
            .select(state.cursor, num_fitting_months as u32);

        let month_iter = state.calendarview_context.offset;

        for (i, &space) in month_views.iter().enumerate() {
            MonthView::from(month_iter + i as u32).render(space, buf, state);
        }
    }
}
