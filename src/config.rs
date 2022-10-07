use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use toml;

const DEFAULT_RECURRENCE_LOOKAHEAD: u32 = 356;
const CONFIG_PATH_ENV_VAR: &str = "JACKAL_CONFIG_FILE";

pub(crate) fn find_configfile() -> io::Result<PathBuf> {
    if let Ok(path) = env::var(CONFIG_PATH_ENV_VAR) {
        return Ok(PathBuf::from(path));
    }

    if let Some(config_dir) = dirs::config_dir() {
        let config_file = config_dir.join("jackal.toml");
        if config_file.is_file() {
            return Ok(config_file);
        }

        let config_file = config_dir.join("jackal/config.toml");
        if config_file.is_file() {
            return Ok(config_file);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not find config file",
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSpec {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSpec {
    pub name: String,
    pub provider: String,
    pub path: PathBuf,
    pub calendars: Vec<CalendarSpec>,
}

fn default_tick_rate() -> Duration {
    Duration::from_secs(60)
}

fn default_recurrence_lookahead() -> u32 {
    DEFAULT_RECURRENCE_LOOKAHEAD
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip, default = "default_tick_rate")]
    pub tick_rate: Duration,
    // Us days.
    // TODO: Implement as chrono::Duration with serde
    #[serde(default = "default_recurrence_lookahead")]
    pub recurrence_lookahead: u32,
    pub collections: Vec<CollectionSpec>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            path: if let Some(path) = dirs::config_dir() {
                path.join("jackal/config.toml")
            } else {
                PathBuf::from("jackal.toml")
            },
            tick_rate: Duration::from_secs(60),
            recurrence_lookahead: 356,
            collections: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
        let mut config: Config = toml::from_str(&fs::read_to_string(path)?)?;
        config.path = path.to_owned();
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(&self.path, toml::to_string(&self)?)?;
        Ok(())
    }

    pub fn collection_config_for(&self, id: &str) -> Option<&CollectionSpec> {
        self.collections.iter().find(|c| &c.name == id)
    }
}
