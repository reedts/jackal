mod agenda;
mod app;
mod cmds;
mod config;
mod events;
mod ical;
mod ui;

use agenda::Agenda;
use app::App;
use config::Config;
use events::Dispatcher;
use std::io::stdout;
use std::path::PathBuf;
use structopt::StructOpt;
use unsegen::base::Terminal;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "jk",
    author = "Julian Bigge <j.reedts@gmail.com>",
    about = "Jackal - A TUI calendar application."
)]
pub struct Args {
    #[structopt(help = "input folder containing *.ics files", parse(from_os_str))]
    pub input: Option<PathBuf>,

    #[structopt(
        name = "CONFIG",
        short = "c",
        long = "config",
        help = "path to config file",
        parse(from_os_str)
    )]
    pub configfile: Option<PathBuf>,

    #[structopt(
        short = "s",
        long = "show",
        help = "only show calendar non-interactively"
    )]
    pub show: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::from_args();
    let config = Config::default();
    let dispatcher = Dispatcher::from_config(config.clone());
    // Setup unsegen terminal
    let stdout = stdout();
    let mut term = Terminal::new(stdout.lock())?;

    let calendar = if let Some(path) = args.input {
        Agenda::new(&path)?
    } else if let Some(calendar_params) = config.calendar_params() {
        // TODO: Handle multiple calendars here. To be thought through...
        Agenda::new(&(calendar_params[0].path))?
    } else {
        // Not one calendar found
        println!("Nothing to do.");
        return Ok(());
    };

    let mut app = App::new(&config, calendar);

    app.run(dispatcher, term)
}
