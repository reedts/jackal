use crate::provider::datetime::days_of_month;
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
    context: &'a Context,
}

impl<'a> MonthPane<'a> {
    const COLUMNS: usize = 7;
    const ROWS: usize = 6;
    const HEADER_ROWS: usize = 2;

    const HEADER: &'static [&'static str] = &["Mon", "Tue", "Wen", "Thu", "Fri", "Sat", "Sun"];

    const WIDTH: usize = Self::COLUMNS * DayCell::CELL_WIDTH;
    const HEIGHT: usize = (Self::ROWS + Self::HEADER_ROWS) * DayCell::CELL_HEIGHT;

    pub fn new(month: Month, year: i32, context: &'a Context) -> Self {
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

    pub fn from_month_index(index: MonthIndex, context: &'a Context) -> Self {
        Self::new(index.index, index.year, context)
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
        let theme = &self.context.theme;

        let mut cursor = Cursor::new(&mut window)
            .wrapping_mode(WrappingMode::Wrap)
            .style_modifier(
                theme
                    .month_header_style
                    .format(theme.month_header_text_style),
            );

        // print Header first
        writeln!(&mut cursor, "{} {}", &self.month.name(), self.year).unwrap();

        for &head in Self::HEADER {
            write!(
                &mut cursor,
                "{:>width$}",
                &head,
                width = DayCell::CELL_WIDTH
            )
            .unwrap();
        }
        cursor.fill_and_wrap_line();

        // set offset for first row and set modifier
        cursor.set_style_modifier(theme.day_style.format(theme.day_text_style));
        cursor.move_by(
            ColDiff::new((DayCell::CELL_WIDTH * self.offset as usize) as i32),
            RowDiff::new(0),
        );

        let is_current_month = (self.context.now().month() == self.month.number_from_month())
            && (self.context.now().year() == self.year);
        let is_selected_month = (self.context.cursor().month() == self.month.number_from_month())
            && (self.context.cursor().year() == self.year);

        for (idx, cell) in (1..=self.num_days).map(|idx| (idx, DayCell::new(idx, &theme))) {
            let is_today = is_current_month && (idx as u32 == self.context.now().day());
            let is_selected = is_selected_month && (idx as u32 == self.context.cursor().day());

            let saved_style = if is_today || is_selected {
                Some(cursor.get_style_modifier())
            } else {
                None
            };

            if is_today {
                cursor
                    .apply_style_modifier(theme.today_day_style.format(theme.today_day_text_style));
            }

            if is_selected {
                cursor
                    .apply_style_modifier(theme.focus_day_style.format(theme.focus_day_text_style));
            }

            write!(&mut cursor, "{}", cell.select(is_selected).today(is_today)).unwrap();

            if let Some(style) = saved_style {
                cursor.set_style_modifier(style);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthIndex {
    pub index: Month,
    pub year: i32,
}

impl MonthIndex {
    pub fn new(index: Month, year: i32) -> Self {
        MonthIndex { index, year }
    }

    pub fn _next(&self) -> Self {
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

    pub fn _prev(&self) -> Self {
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
            let new_month = month_sum - year_diff * 12;

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
            let year_diff = 1 + month_diff.abs() / 12;
            let new_month = 12 - (month_diff.abs() - (year_diff - 1) * 12);

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
pub struct CalendarWindow<'a> {
    context: &'a Context,
}

impl<'a> CalendarWindow<'a> {
    pub fn new(context: &'a Context) -> Self {
        CalendarWindow { context }
    }
}

impl Widget for CalendarWindow<'_> {
    fn space_demand(&self) -> Demand2D {
        Demand2D {
            width: ColDemand::at_least(MonthPane::WIDTH),
            height: RowDemand::at_least(MonthPane::HEIGHT),
        }
    }

    fn draw(&self, mut window: Window, hints: RenderingHints) {
        // Calculate number of fitting month panes and prepare
        // subwindows accordingly
        let num_fitting_months = window.get_height() / MonthPane::HEIGHT;

        let offset: MonthIndex = MonthIndex::from(self.context.cursor.clone())
            - (num_fitting_months.raw_value() / 2) as u32;

        let (subwindow_x, subwindow_y) = (
            (window.get_width().raw_value() - MonthPane::WIDTH as i32) / 2,
            0,
        );
        let pane = window.create_subwindow(
            ColIndex::new(subwindow_x)..ColIndex::new(subwindow_x + MonthPane::WIDTH as i32),
            RowIndex::new(subwindow_y)..RowIndex::new(window.get_height().raw_value()),
        );

        // Check for correct offset
        //
        //
        let mut layout = VLayout::new();

        for i in 0..num_fitting_months.raw_value() {
            layout = layout.widget(MonthPane::from_month_index(
                offset + i as u32,
                &self.context,
            ));
        }

        layout.draw(pane, hints);
    }
}
