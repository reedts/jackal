use crate::calendar::Calendar;

use tui::buffer::Buffer;
use tui::layout::{Constraint, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Table, Row, Widget};

pub struct CalendarView<'a> {
    calendar: &'a mut Calendar<'a>
}

impl<'a> CalendarView<'a> {
    pub fn new(calendar: &'a mut Calendar<'a>) -> CalendarView<'a> {
        CalendarView {
            calendar
        }
    }
}

impl<'a> Widget for CalendarView<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let row_style = Style::default().fg(Color::White);
        let table = Table::new(
                ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter(),
                vec![
                    Row::StyledData(self.calendar.curr_month().days()[0..=7].iter(), row_style),
                    Row::StyledData(self.calendar.curr_month().days()[8..=14].iter(), row_style),
                    Row::StyledData(self.calendar.curr_month().days()[15..=22].iter(), row_style),
                    Row::StyledData(self.calendar.curr_month().days()[22..].iter(), row_style)
                ].into_iter()
            )
            .block(Block::default().title("2020").borders(Borders::ALL))
            .header_style(Style::default().fg(Color::Yellow))
            .widths(&[
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14)
            ])
            .style(Style::default().fg(Color::White))
            .column_spacing(1)
            .draw(area, buf);
    }
}
