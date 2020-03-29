mod calendar;
mod cmds;
mod config;
mod events;
mod ui;

use chrono::{Datelike, NaiveDate, Utc};
use std::io;
use std::path::Path;
use tui::Terminal;
use tui::backend::TermionBackend;
use termion::{
    event::Key,
    raw::IntoRawMode,
    screen::AlternateScreen
};

use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};

use calendar::Calendar;
use config::Config;
use events::{Event, Events};
use ui::calendar_view::CalendarView;

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = Config::default();

    let events = Events::from_config(config.clone());

    let now = Utc::now();

    let mut calendar = Calendar::new(Path::new("/home/reedts/.calendars/google/j.reedts@gmail.com/"), now.date().naive_utc().year())?;
    let mut calendar_view = CalendarView::new(&mut calendar);
    
    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50)
                ].as_ref()
                )
                .split(f.size());
            calendar_view.render(&mut f, layout[0]);
            Block::default().title("Events").borders(Borders::ALL).render(&mut f, layout[1]);
        })?;

        match events.next().unwrap() {
            Event::Tick => {
            },
            Event::Cmd(_) => {},
            Event::Input(key) => match key {
                Key::Char(_) => {
                    break;
                },
                _ => {}
            }
        }
    }

    Ok(())
}
