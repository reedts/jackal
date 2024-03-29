extern crate jackal as lib;

use chrono::{DateTime, Duration, Local, Utc};
use flexi_logger::{Duplicate, FileSpec, Logger};
use lib::{agenda::Agenda, provider::Occurrence};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};
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

fn open_url(url: &str) {
    let open_prog = "xdg-open";
    let _join_handle = std::process::Command::new(open_prog)
        .arg(url)
        .spawn()
        .unwrap();
}

fn notify(
    title: String,
    body: String,
    begin: DateTime<Utc>,
    end: DateTime<Utc>,
    url: Option<String>,
    _guard: NotificationGuard,
) {
    let mut dismissed = false;

    while !dismissed {
        let now = Utc::now().naive_utc();

        let mut n = notify_rust::Notification::new();

        n.action("dismiss", "Dismiss");
        if url.is_some() {
            n.action("open_url", "Open URL");
        }

        let timeout;
        let summary;
        if now < begin.naive_utc() {
            timeout = begin.naive_utc() - now;
            summary = format!("Upcoming event '{}'", title);
            n.action("snooze", "Snooze");
        } else if now < end.naive_utc() {
            timeout = end.naive_utc() - now;
            summary = format!("Current event '{}'", title);
        } else {
            return;
        }

        n.summary(&summary)
            .body(&body)
            .timeout(notify_rust::Timeout::Milliseconds(
                timeout.num_milliseconds() as u32,
            ))
            .hint(notify_rust::Hint::Resident(true));

        n.show().unwrap().wait_for_action(|action| match action {
            "dismiss" => dismissed = true,
            "snooze" => {
                let now = Utc::now().naive_utc();
                let to_sleep = begin.naive_utc() - now;
                log::info!("Sleeping {} until begin notification time", to_sleep,);
                std::thread::sleep(to_sleep.to_std().unwrap_or(std::time::Duration::ZERO));
            }
            "open_url" => open_url(url.as_ref().unwrap()),
            "__closed" => dismissed = true,
            _ => {}
        });
    }
}

struct NotificationGuard {
    map: Arc<Mutex<HashSet<String>>>,
    uid: String,
}

impl NotificationGuard {
    fn new(uid: String, map: &Arc<Mutex<HashSet<String>>>) -> Option<Self> {
        {
            let mut r = map.lock().unwrap();
            if r.contains(&uid) {
                return None;
            }
            r.insert(uid.clone());
        }

        Some(NotificationGuard {
            map: map.clone(),
            uid,
        })
    }
}

impl Drop for NotificationGuard {
    fn drop(&mut self) {
        let mut r = self.map.lock().unwrap();
        r.remove(&self.uid);
    }
}

fn spawn_notify(occurence: Occurrence, running_notifications: &Arc<Mutex<HashSet<String>>>) {
    use linkify::{LinkFinder, LinkKind};
    let guard = if let Some(guard) =
        NotificationGuard::new(occurence.event.uid().to_owned(), running_notifications)
    {
        guard
    } else {
        log::info!(
            "Not rescheduling running notification for event {}",
            occurence.event.title()
        );
        return;
    };

    let begin = occurence.span.begin();
    let end = occurence.span.end();

    let begin_display = begin.with_timezone(&Local);
    let end_display = end.with_timezone(&Local);

    let with_dates = begin_display.date_naive() != end_display.date_naive();
    let time_str = if with_dates {
        format!("{}-{}", begin_display, end_display)
    } else {
        format!("{}-{}", begin_display.time(), end_display.time())
    };
    let mut body = time_str;
    if let Some(description) = occurence.event.description() {
        body += "\n";
        body += description;
    }
    let title = occurence.event.title().to_owned();

    // TODO: We probably want to look for urls in other fields like location or URL, too.
    let url = occurence.event.description().and_then(|description| {
        let mut finder = LinkFinder::new();
        let mut links = finder.kinds(&[LinkKind::Url]).links(description);
        links.next().map(|l| l.as_str().to_owned())
    });

    let _ = std::thread::Builder::new()
        .name("jackal-notify-notification".to_owned())
        .spawn(move || notify(title, body, begin, end, url, guard))
        .unwrap();
}

enum ControlFlow {
    Continue,
    Restart,
}

fn wait(
    events: &std::sync::mpsc::Receiver<lib::events::Event>,
    until: DateTime<Utc>,
    info: &str,
) -> ControlFlow {
    loop {
        let now = Utc::now().naive_utc();
        let to_sleep = until.naive_utc() - now;

        log::info!("Sleeping {} {}", to_sleep, info);

        match events.recv_timeout(to_sleep.to_std().unwrap_or(std::time::Duration::ZERO)) {
            Ok(lib::events::Event::ExternalModification) => return ControlFlow::Restart,
            Ok(lib::events::Event::Update | lib::events::Event::Input(_)) => {
                panic!("No dispatcher was started so where do those come from?!")
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return ControlFlow::Continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                panic!("Event senders are disconnected")
            }
        }
    }
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

    let (tx, mod_rx) = std::sync::mpsc::channel();
    let headsup_time = Duration::minutes(config.notification_headsup_minutes.into());
    let check_window = Duration::days(1);
    assert!(
        check_window > headsup_time,
        "Check window is too small for headsup time"
    );

    let running_notifications = Arc::new(Mutex::new(HashSet::new()));

    let mut calendar = Agenda::from_config(&config, &tx)?;
    'outer: loop {
        calendar.process_external_modifications();

        loop {
            let begin = Utc::now();
            let end = begin + check_window;

            let mut next_events = calendar
                .events_in(begin.naive_utc()..end.naive_utc())
                .collect::<Vec<_>>();
            next_events.sort_unstable_by_key(|occurrence| occurrence.begin());

            for occurrence in next_events {
                let begin_utc = occurrence.begin();
                let headsup_begin = begin_utc - headsup_time;

                match wait(&mod_rx, headsup_begin, "until headsup time of next event") {
                    ControlFlow::Restart => continue 'outer,
                    ControlFlow::Continue => {}
                }

                spawn_notify(occurrence, &running_notifications);
            }

            let end = end - headsup_time;

            match wait(&mod_rx, end, " until end of window. No more events!") {
                ControlFlow::Restart => continue 'outer,
                ControlFlow::Continue => {}
            }
        }
    }
}
