use crate::config;
use crate::cmds;
use std::io;
use std::sync::{
    mpsc,
    atomic::{AtomicBool, Ordering},
    Arc,
};

use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

use config::Config;

pub enum Event<T> {
    Input(T),
    Cmd(cmds::Cmd),
    Tick,
}

pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>
}

impl Events {
    pub fn new() -> Events {
        Events::from_config(Config::default())
    }

    pub fn from_config(config: Config) -> Events {
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
            let tx = tx.clone();
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        Events {
            rx,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
