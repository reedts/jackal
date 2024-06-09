use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone};
use std::fmt::{Display, Write};
use unsegen::base::*;
use unsegen::input::Scrollable;
use unsegen::widget::*;

use crate::provider::{Occurrence, TimeSpan};
use crate::ui::Context;

#[allow(dead_code)]
#[derive(Debug)]
enum Entry {
    Event(TimeSpan<Local>, String),
    DaySeparator(NaiveDate),
    Time(DateTime<Local>),
    Cursor(DateTime<Local>),
}

impl Entry {
    pub fn datetime(&self) -> DateTime<Local> {
        match self {
            Entry::Event(span, _) => span.begin(),
            Entry::DaySeparator(date) => Local
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .earliest()
                .unwrap(),
            Entry::Cursor(dt) | Entry::Time(dt) => dt.clone(),
        }
    }
}

impl From<Occurrence<'_>> for Entry {
    fn from(value: Occurrence) -> Self {
        let Occurrence { span, event } = value;
        Entry::Event(span.with_tz(&Local), event.title().to_owned())
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event(span, title) => {
                let local_span = span.clone().with_tz(&Local);

                let time = if span.is_allday() {
                    "Allday".to_owned()
                } else if span.is_instant() {
                    format!("{}", local_span.begin().time().format("%H:%M"))
                } else {
                    format!(
                        "{} - {}",
                        local_span.begin().time().format("%H:%M"),
                        local_span.end().time().format("%H:%M")
                    )
                };

                write!(f, "\t{}: {}", time, title)
            }
            Self::DaySeparator(date) => write!(f, "{}", date.format("%a, %b %d")),
            Self::Time(dt) => f.pad(&format!("[{}]", dt.time().format("%H:%M"))),
            Self::Cursor(dt) => write!(f, " * {}", dt.time().format("%H:%M")),
        }
    }
}

pub struct EventWindow<'win> {
    context: &'win Context,
    lookahead: Duration,
}

impl<'win> EventWindow<'win> {
    pub fn new(context: &'win Context, lookahead: Duration) -> Self {
        EventWindow { context, lookahead }
    }
}

impl Widget for EventWindow<'_> {
    fn space_demand(&self) -> Demand2D {
        Demand2D {
            width: ColDemand::at_least(10),
            height: RowDemand::at_least(10),
        }
    }

    fn draw(&self, mut window: unsegen::base::Window, _hints: RenderingHints) {
        let date = self.context.cursor().date_naive();

        let mut entries = self
            .context
            .agenda()
            .events_in(
                date.and_hms_opt(0, 0, 0).unwrap()
                    ..(date + self.lookahead).and_hms_opt(23, 59, 59).unwrap(),
            )
            .map(Entry::from)
            .collect::<Vec<Entry>>();

        // Append current time if cursor's date is today
        if self.context.today() == date {
            entries.push(Entry::Time(self.context.now().clone()))
        }

        if !entries.is_empty() {
            entries.sort_unstable_by_key(|entry| entry.datetime());
        }

        log::debug!("{:#?}", entries);

        let width = window.get_width().raw_value() as usize;

        let mut cursor = Cursor::new(&mut window);

        // Only count the real events (no cursor/clock)
        let mut idx: usize = 0;
        let mut date_it = NaiveDate::MIN;

        for ev in entries {
            // Check if iteration entry has new date. If yes write DaySeparator
            let ev_date = ev.datetime().date_naive();
            if ev_date != date_it {
                let saved_style = cursor.get_style_modifier();
                cursor.apply_style_modifier(StyleModifier::new().bold(true).underline(true));
                writeln!(&mut cursor, "{}", Entry::DaySeparator(ev_date)).unwrap();
                cursor.set_style_modifier(saved_style);

                date_it = ev_date;
            }

            match ev {
                ev @ Entry::Event(..) => {
                    let saved_style = cursor.get_style_modifier();

                    if idx == self.context.eventlist_index {
                        cursor.apply_style_modifier(StyleModifier::new().invert(true));
                    }

                    if let Err(err) = write!(&mut cursor, "{}", ev) {
                        log::warn!("Error while writing event: {}", err);
                    }

                    cursor.fill_and_wrap_line();

                    cursor.set_style_modifier(saved_style);
                    idx += 1;
                }
                time @ Entry::Time(_) => {
                    let save_style = cursor.get_style_modifier();

                    cursor.apply_style_modifier(StyleModifier::new().fg_color(Color::LightRed));
                    writeln!(&mut cursor, "{:─^width$}", time).unwrap();
                    cursor.set_style_modifier(save_style);
                }

                entry => writeln!(&mut cursor, "{}", entry).unwrap(),
            }
        }
    }
}

pub struct EventWindowBehaviour<'beh>(pub &'beh mut Context, pub usize);

impl Scrollable for EventWindowBehaviour<'_> {
    fn scroll_backwards(&mut self) -> unsegen::input::OperationResult {
        if self.0.eventlist_index > 0 {
            self.0.eventlist_index -= 1;
            Ok(())
        } else {
            Err(())
        }
    }

    fn scroll_forwards(&mut self) -> unsegen::input::OperationResult {
        if self.0.eventlist_index + 1 < self.1 {
            self.0.eventlist_index += 1;
            Ok(())
        } else {
            Err(())
        }
    }
}
