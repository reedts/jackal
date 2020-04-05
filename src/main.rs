mod app;
mod calendar;
mod cmds;
mod config;
mod control;
mod events;
mod ical;
mod ui;

use chrono::{Datelike, Utc};
use std::io;
use std::path::Path;
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::backend::TermionBackend;
use tui::widgets::Widget;
use tui::Terminal;

use app::App;
use calendar::Calendar;
use config::Config;
use events::{Dispatcher, Event};

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let config = Config::default();

    let dispatcher = Dispatcher::from_config(config.clone());

    let now = Utc::now();
    let calendar = Calendar::new(
        Path::new("/home/reedts/.calendars/google/j.reedts@gmail.com/"),
        now.date().naive_utc().year(),
    )?;

    let mut app = App::new(&config, calendar);

    loop {
        // Draw
        terminal.draw(|mut f| {
            let size = f.size();
            app.render(&mut f, size);
        })?;

        // Handle events
        match dispatcher.next().unwrap() {
            Event::Tick => {}
            Event::Input(key) => {
                app.handle(Event::Input(key));
            }
            _ => {}
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}
