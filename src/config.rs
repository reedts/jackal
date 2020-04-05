use crate::cmds;
use cmds::Cmd;
use std::collections::HashMap;
use std::time::Duration;

use termion::event::Key;

pub type KeyMap = HashMap<Key, Cmd>;

#[derive(Debug, Clone)]
pub struct Config {
    pub key_map: KeyMap,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        let mut config = Config {
            key_map: HashMap::new(),
            tick_rate: Duration::from_millis(500),
        };

        config.key_map.insert(Key::Char('l'), Cmd::NextDay);
        config.key_map.insert(Key::Char('h'), Cmd::PrevDay);
        config.key_map.insert(Key::Char('j'), Cmd::NextWeek);
        config.key_map.insert(Key::Char('k'), Cmd::PrevWeek);
        config.key_map.insert(Key::Char('q'), Cmd::Exit);

        config
    }
}
