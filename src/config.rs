use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

const APP_DIR: &str = "menubar-world-clock";
const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClockConfig {
    pub city: String,
    pub timezone: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
    pub clocks: Vec<ClockConfig>,
}

#[derive(Clone, Debug)]
pub struct ResolvedClock {
    pub city: String,
    pub timezone: Tz,
}

#[derive(Debug)]
pub enum ConfigError {
    HomeDirectoryUnavailable,
    Io(io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    Empty,
    EmptyCity(usize),
    InvalidTimezone { index: usize, value: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeDirectoryUnavailable => write!(f, "could not locate the home directory"),
            Self::Io(error) => write!(f, "could not read or write the configuration: {error}"),
            Self::Parse(error) => write!(f, "invalid TOML configuration: {error}"),
            Self::Serialize(error) => write!(f, "could not serialize defaults: {error}"),
            Self::Empty => write!(f, "the configuration must contain at least one clock"),
            Self::EmptyCity(index) => write!(f, "clock {} has an empty city name", index + 1),
            Self::InvalidTimezone { index, value } => write!(
                f,
                "clock {} has an invalid IANA time zone: {value}",
                index + 1
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clocks: vec![
                clock("San Francisco", "America/Los_Angeles"),
                clock("New York", "America/New_York"),
                clock("UTC", "UTC"),
                clock("Paris", "Europe/Paris"),
            ],
        }
    }
}

fn clock(city: &str, timezone: &str) -> ClockConfig {
    ClockConfig {
        city: city.to_owned(),
        timezone: timezone.to_owned(),
    }
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    // `dirs` is intentionally used here instead of reading HOME directly.
    let home = dirs::home_dir().ok_or(ConfigError::HomeDirectoryUnavailable)?;
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    Ok(config_home.join(APP_DIR).join(CONFIG_FILE))
}

pub fn load_or_create() -> Result<(PathBuf, Vec<ResolvedClock>), ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        let defaults = Config::default();
        let contents = toml::to_string_pretty(&defaults).map_err(ConfigError::Serialize)?;
        let parent = path.parent().expect("configuration path has a parent");
        fs::create_dir_all(parent)?;
        fs::write(&path, contents)?;
    }

    let contents = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents).map_err(ConfigError::Parse)?;
    let clocks = config.resolve()?;
    Ok((path, clocks))
}

impl Config {
    fn resolve(self) -> Result<Vec<ResolvedClock>, ConfigError> {
        if self.clocks.is_empty() {
            return Err(ConfigError::Empty);
        }

        self.clocks
            .into_iter()
            .enumerate()
            .map(|(index, clock)| {
                let city = clock.city.trim().to_owned();
                if city.is_empty() {
                    return Err(ConfigError::EmptyCity(index));
                }

                let timezone =
                    clock
                        .timezone
                        .parse::<Tz>()
                        .map_err(|_| ConfigError::InvalidTimezone {
                            index,
                            value: clock.timezone.clone(),
                        })?;

                Ok(ResolvedClock { city, timezone })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_the_requested_locations() {
        let config = Config::default();
        let cities: Vec<_> = config
            .clocks
            .iter()
            .map(|clock| clock.city.as_str())
            .collect();
        assert_eq!(cities, ["San Francisco", "New York", "UTC", "Paris"]);
    }

    #[test]
    fn resolves_iana_timezones() {
        let clocks = Config::default().resolve().unwrap();
        assert_eq!(clocks[0].timezone, chrono_tz::America::Los_Angeles);
        assert_eq!(clocks[3].timezone, chrono_tz::Europe::Paris);
    }

    #[test]
    fn rejects_an_invalid_timezone() {
        let config = Config {
            clocks: vec![clock("Somewhere", "Mars/Olympus_Mons")],
        };
        assert!(matches!(
            config.resolve(),
            Err(ConfigError::InvalidTimezone { .. })
        ));
    }
}
