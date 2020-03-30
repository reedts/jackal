mod calendar;
mod cmds;
mod config;
mod events;
mod ui;

use chrono::{Datelike, Utc};
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
use cmds::Receiver;
use config::Config;
use events::{Event, Events};
use ui::calendar_view::CalendarView;

enum Mode {
    Calendar,
    Events,
    Input
}

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut active_mode = Mode::Calendar;

    let config = Config::default();

    let events = Events::from_config(config.clone());

    let now = Utc::now();

    let mut calendar = Calendar::new(Path::new("/home/reedts/.calendars/google/j.reedts@gmail.com/"), now.date().naive_utc().year())?;
    let mut calendar_view = Box::new(CalendarView::new(&mut calendar));

    loop {
        // Draw
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
        
        // Handle events
        match events.next().unwrap() {
            Event::Tick => {}
            Event::Input(key) => {
                match active_mode {
                    Mode::Calendar => match config.key_map.get(&key) {
                        Some(cmd) => {
                            calendar_view.recv(*cmd);
                        },
                        None => {}
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    Ok(())
}
