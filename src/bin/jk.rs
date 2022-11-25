extern crate jackal as lib;

use flexi_logger::{Duplicate, FileSpec, Logger};
use lib::agenda::Agenda;
use lib::events::Dispatcher;
use lib::ui::app::App;
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

    #[structopt(long = "log-file", help = "path to log file", parse(from_os_str))]
    pub log_file: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::from_args();

    let mut logger = Logger::try_with_env_or_str("info")?.duplicate_to_stderr(Duplicate::Warn);

    if let Some(log_file) = args.log_file {
        logger = logger
            .log_to_file(FileSpec::try_from(log_file)?)
            .print_message();
    }

    logger.start()?;

    let config = lib::config::load_suitable_config(args.configfile.as_deref())?;

    let dispatcher = Dispatcher::from_config(&config);
    // Setup unsegen terminal
    let stdout = stdout();
    let term = Terminal::new(stdout.lock())?;

    let calendar = Agenda::from_config(&config, dispatcher.event_sink())?;

    let mut app = App::new(&config, calendar);

    app.run(dispatcher, term)
}
