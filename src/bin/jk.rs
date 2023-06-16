extern crate jackal as lib;

use flexi_logger::{FileSpec, Logger};
use lib::agenda::Agenda;
use lib::events::Dispatcher;
use lib::ui::app::App;
use nix::sys::{signal, termios};
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

    const DEFAULT_LOG_LEVEL: &'static str = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };

    let mut logger = Logger::try_with_env_or_str(DEFAULT_LOG_LEVEL)?;

    if let Some(log_file) = args.log_file {
        logger = logger
            .log_to_file(FileSpec::try_from(log_file)?)
            .print_message();
    }

    logger.start()?;

    const STDOUT: std::os::unix::io::RawFd = 0;
    let orig_attr = std::sync::Mutex::new(
        termios::tcgetattr(STDOUT).expect("Failed to get terminal attributes"),
    );

    std::panic::set_hook(Box::new(move |info| {
        // Switch to main terminal screen
        println!("{}{}", termion::screen::ToMainScreen, termion::cursor::Show);

        let _ = termios::tcsetattr(STDOUT, termios::SetArg::TCSANOW, &orig_attr.lock().unwrap());

        println!("Jackal ran into a fatal error!");
        println!(
            "Consider filing an issue with a log file and the backtrace below at {}",
            env!("CARGO_PKG_REPOSITORY")
        );

        println!("{}", info);
        println!("{:?}", backtrace::Backtrace::new());
    }));

    let mut signals_to_wait = signal::SigSet::empty();
    signals_to_wait.add(signal::SIGWINCH);

    let config = lib::config::load_suitable_config(args.configfile.as_deref())?;

    let dispatcher = Dispatcher::from_config(&config, signals_to_wait);
    // Setup unsegen terminal
    let stdout = stdout();
    let term = Terminal::new(stdout.lock())?;

    let calendar = Agenda::from_config(&config, dispatcher.event_sink())?;

    let mut app = App::new(&config, calendar);

    app.run(dispatcher, term)
}
