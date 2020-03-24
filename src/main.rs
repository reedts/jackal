mod calendar;

use chrono::{Datelike, NaiveDate, Utc};
use std::io;
use std::path::Path;
use tui::Terminal;
use tui::backend::TermionBackend;
use termion::raw::IntoRawMode;

use tui::widgets::{Widget, Block, Borders};
use tui::layout::{Layout, Constraint, Direction};

use calendar::Calendar;

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let now = Utc::now();

    let mut calendar = Calendar::new(Path::new("/home/reedts/.calendar/google/j.reedts@gmail.com/"), now.date().naive_utc().year());

    terminal.draw(|mut f| {
        let size = f.size();
        Block::default()
            .title("Test Block")
            .borders(Borders::ALL)
            .render(&mut f, size);
    })
}
