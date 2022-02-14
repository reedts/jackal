use crate::config;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread;

use unsegen::input::Input;

use config::Config;

pub enum Event {
    Input(Input),
    Update,
}

pub struct Dispatcher {
    rx: mpsc::Receiver<Event>,
    _input_handle: thread::JoinHandle<()>,
    _update_handle: thread::JoinHandle<()>,
}

impl Default for Dispatcher {
    fn default() -> Dispatcher {
        Dispatcher::from_config(&Config::default())
    }
}

impl Dispatcher {
    pub fn from_config(config: &Config) -> Dispatcher {
        let tick_rate = config.tick_rate.clone();
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                let stdin = stdin.lock();
                for evt in Input::read_all(stdin) {
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
        let update_handle = {
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Update).unwrap();
                    thread::sleep(tick_rate);
                }
            })
        };
        Dispatcher {
            rx,
            _input_handle: input_handle,
            _update_handle: update_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
