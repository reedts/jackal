use crate::ical::days_of_month;
use chrono::{Datelike, Month, NaiveDate};
use std::fmt::Display;
use std::fmt::Write;
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
            &self.theme.today_day_char.unwrap_or(' ')
        } else {
            &' '
        };

        let arg_focus = if self.selected {
            &self.theme.focus_day_char.unwrap_or(' ')
        } else {
            &' '
        };

        write!(f, "{}{}{:>2}", arg_today, arg_today, self.day_num)
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

    fn draw(&self, window: Window, hints: RenderingHints) {
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
            write!(&cursor, "{:>width$}", &head, width = DayCell::CELL_WIDTH);
        }

        // set offset for first row and set modifier
        cursor
            .style_modifier(theme.day_style.format(theme.day_text_style))
            .move_by(
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
                &cursor,
                "{}",
                cell.select(false).today(
                    &self.context.now().month() == &self.month.number_from_month()
                        && self.context.now().day() == idx as u32 + 1
                )
            );
        }
    }
}
