use crate::calendar::{Calendar, Day, Month};

use chrono::Utc;

use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Text, Table, Row, Paragraph, Widget};

struct DayBlock<'a> {
    day: &'a Day<'a, Utc>
}

pub struct CalendarView<'a> {
    calendar: &'a Calendar<'a>,
    selection: &'a Month<'a, Utc>,
    day_blocks: Vec<DayBlock<'a>>
}

impl<'a> CalendarView<'a> {
    pub fn new(calendar: &'a mut Calendar<'a>) -> CalendarView<'a> {
        let curr_month = calendar.curr_month();
        CalendarView {
            calendar,
            selection: curr_month,
            day_blocks: curr_month.days().iter()
                .map(|d| DayBlock { day: d })
                .collect()
        }
    }
}

impl<'a> Widget for DayBlock<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new([Text::raw("test")].iter())
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .draw(area, buf);
    }
}

impl<'a> Widget for CalendarView<'a> {
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
            ].as_ref())
            .margin(0)
            .split(Rect {
                x: area.x,
                y: area.y,
                width: 70,
                height:27
            });
        
        let mut rows: Vec<Vec<Rect>> = rows.iter().map(|r| {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10)
                ].as_ref())
                .margin(0)
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
                .alignment(Alignment::Left)
                .draw(*col, buf);
        }

        for r in &rows[1..] {
            for c in r {
                self.day_blocks.first_mut().unwrap().draw(*c, buf);
            }
        }
    }
}
