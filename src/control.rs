use crate::cmds::{Cmd, CmdFailed, Result};
use crate::config::KeyMap;
use crate::events::Event;

pub trait Control {
    fn send_cmd(&mut self, cmd: Cmd) -> Result;
}

pub enum Mode {
    Normal,
    Visual
}

pub struct Controller<'a, C: Control> {
    mode: Mode,
    key_map: &'a KeyMap,
    recvr: C
}

impl<'a, C: Control> Controller<'a, C> {
    pub fn new(key_map: &'a KeyMap, recvr: C) -> Controller<'a, C> {
        Controller {
            mode: Mode::Normal,
            key_map,
            recvr
        }
    }

    pub fn handle(&mut self, event: Event) -> Result {
        match event {
            Event::Input(key) => {
                match self.key_map.get(&key) {
                    Some(cmd) => {
                        self.recvr.send_cmd(*cmd)
                    },
                    None => Err(CmdFailed {})
                }
            },
            _ => Err(CmdFailed {})
        }
    }

    pub fn inner(&self) -> &C {
        &self.recvr
    }

    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.recvr
    }
}
