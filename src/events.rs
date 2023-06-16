use crate::config;
use nix::sys::signal;
use std::io;
use std::sync::mpsc;
use std::thread;

use unsegen::input::Input;

use config::Config;

pub enum Event {
    Input(Input),
    Signal(signal::Signal),
    Update,
    ExternalModification,
}

pub struct Dispatcher {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    _input_handle: thread::JoinHandle<()>,
    _update_handle: thread::JoinHandle<()>,
    _signal_handle: thread::JoinHandle<()>,
}

impl Default for Dispatcher {
    fn default() -> Dispatcher {
        Dispatcher::from_config(&Config::default(), signal::SigSet::empty())
    }
}

impl Dispatcher {
    pub fn from_config(config: &Config, signals_to_wait: signal::SigSet) -> Dispatcher {
        signals_to_wait.thread_block().unwrap();

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
        let tx_upd = tx.clone();
        let update_handle = {
            thread::spawn(move || loop {
                tx_upd.send(Event::Update).unwrap();
                thread::sleep(tick_rate);
            })
        };

        let signal_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if let Ok(sig) = signals_to_wait.wait() {
                    tx.send(Event::Signal(sig)).unwrap()
                }
            })
        };

        Dispatcher {
            rx,
            tx,
            _input_handle: input_handle,
            _update_handle: update_handle,
            _signal_handle: signal_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn event_sink(&self) -> &mpsc::Sender<Event> {
        &self.tx
    }
}
