use std::collections::HashMap;
use std::env;
use std::io;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CONFIG_PATH_ENV_VAR: &str = "JACKAL_CONFIG_FILE";

pub(crate) fn find_configfile_locations() -> io::Result<Vec<PathBuf>> {
    let config_env: Option<PathBuf> = if let Ok(path) = env::var(CONFIG_PATH_ENV_VAR) {
        Some(PathBuf::from(path))
    } else {
        None
    };

    let home = if let Ok(dir) = env::var("HOME") {
        PathBuf::from(dir)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to find home directory",
        ));
    };

    let home_config = PathBuf::from_iter([&home, &PathBuf::from(".jackal.toml")].iter());

    let config_xdg = if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from_iter([dir, "jackal".to_string(), "config.toml".to_string()].iter())
    } else {
        PathBuf::from_iter(
            [
                home.as_path(),
                Path::new(".config"),
                Path::new("jackal"),
                Path::new("config.toml"),
            ]
            .iter(),
        )
    };

    let mut locations = vec![config_xdg, home_config];

    if let Some(path) = config_env {
        locations.insert(0, path);
    }

    Ok(locations)
}

#[derive(Debug, Clone)]
pub struct CalendarParams {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub tick_rate: Duration,
    calendar_params: HashMap<String, CalendarParams>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            tick_rate: Duration::from_secs(60),
            calendar_params: HashMap::new(),
        }
    }
}

impl Config {
    pub fn calendar_params(&self) -> Option<Vec<&CalendarParams>> {
        if self.calendar_params.is_empty() {
            None
        } else {
            Some(self.calendar_params.values().collect())
        }
    }

    pub fn calendar_params_for(&self, id: &str) -> Option<&CalendarParams> {
        self.calendar_params.get(id)
    }
}
