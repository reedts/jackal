mod app;
mod args;
mod calendar;
mod cmds;
mod config;
mod ctrl;
mod ctx;
mod events;
mod ical;
mod ui;

use std::io;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::backend::TermionBackend;
use tui::Terminal;

use app::App;
use args::Args;
use calendar::Calendar;
use config::Config;
use events::{Dispatcher, Event};

fn main() -> Result<(), io::Error> {
    let args = Args::from_args();

    let config = Config::default();

    let dispatcher = Dispatcher::from_config(config.clone());

    let calendar = if let Some(path) = args.input {
        Calendar::new(&path)?
    } else if let Some(calendar_params) = config.calendar_params() {
        // TODO: Handle multiple calendars here. To be thought through...
        Calendar::new(&(calendar_params[0].path))?
    } else {
        // Not one calendar found
        println!("Nothing to do.");
        return Ok(());
    };

    let mut app = App::new(&config, calendar);

    if args.show {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        terminal.draw(|mut f| {
            app::draw(&mut f, &mut app);
        })?;
    } else {
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        loop {
            // Draw
            terminal.draw(|mut f| {
                app::draw(&mut f, &mut app);
            })?;

            // Handle events
            let result = match dispatcher.next().unwrap() {
                Event::Tick => app.handle(Event::Tick),
                Event::Input(key) => app.handle(Event::Input(key)),
                _ => Ok(cmds::Cmd::Noop),
            };

            //if app.quit {
            //    break;
            //}
        }
    }

    Ok(())
}
