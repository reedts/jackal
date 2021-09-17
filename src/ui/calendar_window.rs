use crate::ical::days_of_month;
use chrono::{Datelike, Local, Month, NaiveDate};
use num_traits::FromPrimitive;
use std::fmt::Display;
use std::fmt::Write;
use std::ops::{Add, Sub};
use unsegen::base::*;
use unsegen::widget::*;

use super::{Context, Theme};

pub struct DayCell<'a> {
    day_num: u8,
    selected: bool,
    is_today: bool,
    theme: &'a Theme,
}

impl<'a> DayCell<'a> {
    const CELL_HEIGHT: usize = 1;
    const CELL_WIDTH: usize = 4;

    fn new(day_num: u8, theme: &'a Theme) -> Self {
        DayCell {
            day_num,
            selected: false,
            is_today: false,
            theme,
        }
    }

    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    fn select(mut self, selected: bool) -> Self {
        self.set_selected(selected);
        self
    }

    fn set_today(&mut self, is_today: bool) {
        self.is_today = is_today;
    }

    fn today(mut self, is_today: bool) -> Self {
        self.set_today(is_today);
        self
    }
}

impl Display for DayCell<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arg_today = if self.is_today {
            self.theme.today_day_char.unwrap_or(' ')
        } else {
            ' '
        };

        let arg_focus = if self.selected {
            self.theme.focus_day_char.unwrap_or(' ')
        } else {
            ' '
        };

        write!(f, "{}{}{:>2}", arg_today, arg_focus, self.day_num)
    }
}

#[derive(Clone)]
pub struct MonthPane<'a> {
    month: Month,
    year: i32,
    num_days: u8,
    offset: u8,
    context: &'a Context<'a>,
}

impl<'a> MonthPane<'a> {
    const COLUMNS: usize = 7;
    const ROWS: usize = 6;
    const HEADER_ROWS: usize = 1;

    const HEADER: &'static [&'static str] = &["Mon", "Tue", "Wen", "Thu", "Fri", "Sat", "Sun"];

    pub fn new(month: Month, year: i32, context: &'a Context<'a>) -> Self {
        let num_days = days_of_month(&month, year);
        let offset = NaiveDate::from_ymd(year, month.number_from_month(), 1)
            .weekday()
            .num_days_from_monday() as u8;

        MonthPane {
            month,
            year,
            num_days: num_days as u8,
            offset,
            context,
        }
    }
}

impl Widget for MonthPane<'_> {
    fn space_demand(&self) -> Demand2D {
        Demand2D {
            width: ColDemand::exact(Self::COLUMNS * DayCell::CELL_WIDTH),
            height: RowDemand::exact(Self::HEADER_ROWS + Self::ROWS * DayCell::CELL_HEIGHT),
        }
    }

    fn draw(&self, mut window: Window, _hints: RenderingHints) {
        let theme = &self.context.tui_context().theme;

        let mut cursor = Cursor::new(&mut window)
            .wrapping_mode(WrappingMode::Wrap)
            .style_modifier(
                theme
                    .month_header_style
                    .format(theme.month_header_text_style),
            );

        // print Header first
        for &head in Self::HEADER {
            write!(
                &mut cursor,
                "{:>width$}",
                &head,
                width = DayCell::CELL_WIDTH
            )
            .unwrap();
        }

        // set offset for first row and set modifier
        cursor.set_style_modifier(theme.day_style.format(theme.day_text_style));
        cursor.move_by(
            ColDiff::new((DayCell::CELL_WIDTH * self.offset as usize) as i32),
            RowDiff::new(0),
        );

        for (idx, cell) in (1..self.num_days)
            .into_iter()
            .map(|idx| DayCell::new(idx, &theme))
            .into_iter()
            .enumerate()
        {
            write!(
                &mut cursor,
                "{}",
                cell.select(false).today(
                    &self.context.now().month() == &self.month.number_from_month()
                        && self.context.now().day() == idx as u32 + 1
                )
            )
            .unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MonthIndex {
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
struct CalendarWindow<'a> {
    context: &'a Context<'a>,
    offset: MonthIndex,
    scrolloff: u32,
}

impl<'a> CalendarWindow<'a> {
    pub fn new<T>(context: &'a Context<'a>, selected: T, scrolloff: u32) -> Self
    where
        MonthIndex: From<T>,
    {
        CalendarWindow {
            context,
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
