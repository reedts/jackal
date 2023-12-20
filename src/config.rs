use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
// FIXME: Use chrono::Duration once they support serde
use std::time::Duration;
use toml;

use crate::provider::tz::Tz;

const DEFAULT_NOTIFICATION_HEADSUP_MINUTES: u32 = 10;
const CONFIG_PATH_ENV_VAR: &str = "JACKAL_CONFIG_FILE";

fn find_configfile() -> io::Result<PathBuf> {
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

#[derive(Debug, Clone, Deserialize)]
pub struct CalendarConfig {
    pub id: String,
    pub name: String,
    pub override_tz: Option<Tz>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectionConfig {
    pub name: String,
    pub provider: String,
    pub path: PathBuf,
    pub calendars: Vec<CalendarConfig>,
}

fn find_default_tz() -> Tz {
    const LOCALTIME_LOCATION: &str = "/etc/localtime";
    const ZONEINFO_DIR: &str = "/usr/share/zoneinfo/";

    let tz_name = if let Ok(tz) = env::var("TZ") {
        tz
    } else {
        fs::read_link(LOCALTIME_LOCATION)
            .ok()
            .and_then(|path| {
                path.strip_prefix(ZONEINFO_DIR)
                    .map(|p| p.to_string_lossy().to_string())
                    .ok()
            })
            .unwrap_or("localtime".to_string())
    };

    tz_name.parse::<Tz>().unwrap()
}

fn default_tick_rate() -> Duration {
    Duration::from_secs(60)
}

fn default_event_lookahead() -> Duration {
    // 8 weeks
    Duration::from_secs(3600 * 24 * 7 * 8)
}

fn default_notification_headsup_minutes() -> u32 {
    DEFAULT_NOTIFICATION_HEADSUP_MINUTES
}

pub fn load_suitable_config(
    configfile: Option<&Path>,
) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(if let Some(path) = configfile {
        Config::read(&path)?
    } else if let Ok(path) = find_configfile() {
        Config::read(&path)?
    } else {
        Config::default()
    })
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip, default = "default_tick_rate")]
    pub tick_rate: Duration,

    #[serde(default = "default_event_lookahead")]
    pub event_lookahead: Duration,

    #[serde(default = "find_default_tz")]
    pub tz: Tz,

    #[serde(default = "default_notification_headsup_minutes")]
    pub notification_headsup_minutes: u32,

    pub collections: Vec<CollectionConfig>,
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
            tz: find_default_tz(),
            event_lookahead: default_tick_rate(),
            notification_headsup_minutes: default_notification_headsup_minutes(),
            collections: Vec::new(),
        }
    }
}

impl Config {
    pub fn read(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
        let mut config: Config = toml::from_str(&fs::read_to_string(path)?)?;
        config.path = path.to_owned();
        Ok(config)
    }
}
