use crate::cmds;
use crate::config;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use std::thread;

use termion::event::Key;
use termion::input::TermRead;

use config::Config;

pub enum Event {
    Input(Key),
    Cmd(cmds::Cmd),
    Tick,
}

pub struct Dispatcher {
    rx: mpsc::Receiver<Event>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}

impl Default for Dispatcher {
    fn default() -> Dispatcher {
        Dispatcher::from_config(Config::default())
    }
}

impl Dispatcher {
    pub fn from_config(config: Config) -> Dispatcher {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::Input(key)) {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };
        let tick_handle = {
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        Dispatcher {
            rx,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
