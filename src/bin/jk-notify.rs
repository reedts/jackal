extern crate jackal as lib;

use chrono::{Duration, Utc};
use flexi_logger::{Duplicate, FileSpec, Logger};
use lib::{agenda::Agenda, events::Dispatcher};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "jk-notify",
    author = "Julian Bigge <j.reedts@gmail.com>",
    about = "Notification deamon of the jackal calendar suite."
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

    let _dispatcher = Dispatcher::from_config(&config);

    let calendar = Agenda::from_config(&config)?;

    let headsup_time = Duration::minutes(config.notification_headsup_minutes.into());
    let check_window = Duration::days(1);
    assert!(
        check_window > headsup_time,
        "Check window is too small for headsup time"
    );
    loop {
        let begin = Utc::now().naive_utc();
        let end = begin + check_window;

        let mut next_events = calendar.events_in(begin..end).collect::<Vec<_>>();
        next_events.sort_unstable_by_key(|(begin, _)| *begin);

        for (begin, event) in next_events {
            let begin_utc = begin.naive_utc();
            let headsup_begin = begin_utc - headsup_time;
            let now = Utc::now().naive_utc();
            let to_sleep = headsup_begin - now;
            log::info!(
                "Sleeping {} until headsup time of next event {}",
                to_sleep,
                event.summary()
            );

            // Chrono duration may be negative, in which case we do not want to sleep
            std::thread::sleep(to_sleep.to_std().unwrap_or(std::time::Duration::ZERO));

            let end = *begin + event.duration();
            let summary = format!("Upcoming event '{}'", event.title());
            let with_dates = begin.date() != end.date();
            let time_str = if with_dates {
                format!("{}-{}", begin.naive_local(), end.naive_local())
            } else {
                format!(
                    "{}-{}",
                    begin.naive_local().time(),
                    end.naive_local().time()
                )
            };
            let body = format!("{}\n{}", time_str, event.summary());

            let now = Utc::now().naive_utc();
            let timeout = end.naive_utc() - now;
            notify_rust::Notification::new()
                .summary(&summary)
                .body(&body)
                .timeout(notify_rust::Timeout::Milliseconds(
                    timeout.num_milliseconds() as u32,
                ))
                .show()
                .unwrap();
            log::info!("After notification");
        }

        let now = Utc::now().naive_utc();
        let end = end - headsup_time;
        let to_sleep = end - now;
        eprintln!("No more events in batch. Sleeping for {}", to_sleep);
        std::thread::sleep(to_sleep.to_std().unwrap_or(std::time::Duration::ZERO));
    }
}
