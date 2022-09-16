mod agenda;
mod config;
mod events;
mod provider;
mod ui;

use agenda::Agenda;
use config::Config;
use events::Dispatcher;
use flexi_logger::{Duplicate, FileSpec, Logger};
use std::convert::TryFrom;
use std::io::stdout;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use ui::app::App;
use unsegen::base::Terminal;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "jk",
    author = "Julian Bigge <j.reedts@gmail.com>",
    about = "Jackal - A TUI calendar application."
)]
pub struct Args {
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
    Logger::try_with_env_or_str("info")?
        .log_to_file(FileSpec::default())
        .print_message()
        .duplicate_to_stderr(Duplicate::Warn)
        .start()?;

    let args = Args::from_args();
    let config = if let Some(path) = args.configfile {
        Config::load(&path)?
    } else if let Ok(path) = config::find_configfile() {
        Config::load(&path)?
    } else {
        Config::default()
    };

    // print!("{:?}", config);
    // return Ok(())

    let dispatcher = Dispatcher::from_config(&config);
    // Setup unsegen terminal
    let stdout = stdout();
    let mut term = Terminal::new(stdout.lock())?;

    let calendar = Agenda::from_config(&config)?;

    let mut app = App::new(&config, calendar);

    app.run(dispatcher, term)
}
