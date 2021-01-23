use crate::cmds::{Cmd, CmdError, CmdResult};
use crate::config::KeyMap;
use crate::ctx::Context;
use crate::events::Event;

pub trait Control {
    fn send_cmd(&mut self, cmd: &Cmd, context: &mut Context) -> CmdResult;
}

pub enum Mode {
    Normal,
    Visual,
}

pub struct Controller<'a, C: Control> {
    mode: Mode,
    key_map: &'a KeyMap,
    recvr: C,
}

impl<'a, C: Control> Controller<'a, C> {
    pub fn new(key_map: &'a KeyMap, recvr: C) -> Controller<'a, C> {
        Controller {
            mode: Mode::Normal,
            key_map,
            recvr,
        }
    }

    pub fn handle(&mut self, event: Event, context: &mut Context) -> CmdResult {
        match event {
            Event::Input(key) => match self.key_map.get(&key) {
                Some(cmd) => self.recvr.send_cmd(cmd, context),
                None => Err(CmdError::new(format!("Could not find map for '{:?}'", key))),
            },
            _ => Ok(Cmd::Noop),
        }
    }

    pub fn inner(&self) -> &C {
        &self.recvr
    }

    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.recvr
    }
}
