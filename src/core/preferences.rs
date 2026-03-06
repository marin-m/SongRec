use gettextrs::gettext;
use log::{debug, error};
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Read, Write};

use crate::utils::filesystem_operations::obtain_preferences_file_path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Preferences {
    pub enable_notifications: Option<bool>,
    pub enable_systray: Option<bool>,
    pub enable_mpris: Option<bool>,
    pub no_duplicates: Option<bool>,
    pub buffer_size_secs: Option<u64>,
    pub request_interval_secs: Option<u64>, // Legacy, before increasing default from 4 to 10
    pub request_interval_secs_v2: Option<u64>, // before decreasing from 10 to 8
    pub request_interval_secs_v3: Option<u64>,
    pub current_device_name: Option<String>,
}

impl Preferences {
    pub fn new() -> Self {
        Preferences {
            enable_notifications: None,
            enable_systray: None,
            enable_mpris: None,
            no_duplicates: None,
            buffer_size_secs: None,
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: None,
            current_device_name: None,
        }
    }

    pub fn with_interval(interval: u64) -> Self {
        Preferences {
            enable_notifications: Some(true),
            enable_systray: Some(false),
            enable_mpris: Some(false),
            no_duplicates: Some(false),
            buffer_size_secs: Some(12),
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: Some(interval),
            current_device_name: None,
        }
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            enable_notifications: Some(true),
            enable_systray: Some(false),
            enable_mpris: Some(false),
            no_duplicates: Some(false),
            buffer_size_secs: Some(12),
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: Some(8),
            current_device_name: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreferencesInterface {
    pub preferences_file_path: Option<String>,
    pub preferences: Preferences,
}

impl PreferencesInterface {
    pub fn new() -> Self {
        match PreferencesInterface::load() {
            Ok(preferences_interface) => return preferences_interface,
            Err(e) => {
                error!("{} {}", gettext("When parsing the preferences file:"), e);
                return PreferencesInterface {
                    preferences_file_path: obtain_preferences_file_path().ok(),
                    preferences: Preferences::default(),
                };
            }
        }
    }

    fn load() -> Result<PreferencesInterface, Box<dyn Error>> {
        let preferences_file_path: String = obtain_preferences_file_path()?;
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&preferences_file_path)?;
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)?;
        let preferences: Preferences = toml::from_str(&contents)?;
        debug!(
            "Loaded preferences from {}: {:?}",
            preferences_file_path, preferences
        );
        Ok(PreferencesInterface {
            preferences_file_path: Some(preferences_file_path),
            preferences: preferences,
        })
    }

    pub fn update(self: &mut Self, update_preferences: Preferences) {
        let current_preferences = self.preferences.clone();
        self.preferences = Preferences {
            enable_notifications: update_preferences
                .enable_notifications
                .or(current_preferences.enable_notifications),
            enable_mpris: update_preferences
                .enable_mpris
                .or(current_preferences.enable_mpris),
            enable_systray: update_preferences
                .enable_systray
                .or(current_preferences.enable_systray),
            no_duplicates: update_preferences
                .no_duplicates
                .or(current_preferences.no_duplicates),
            buffer_size_secs: update_preferences
                .buffer_size_secs
                .or(current_preferences.buffer_size_secs),
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: update_preferences
                .request_interval_secs_v2
                .or(match current_preferences.request_interval_secs {
                    Some(4) => None,
                    Some(val) => Some(val),
                    None => None,
                })
                .or(match current_preferences.request_interval_secs_v2 {
                    Some(10) => None,
                    Some(val) => Some(val),
                    None => None,
                })
                .or(current_preferences.request_interval_secs_v3),
            current_device_name: update_preferences
                .current_device_name
                .or(current_preferences.current_device_name),
        };
        match self.write() {
            Ok(_) => {}
            Err(e) => {
                error!("{} {}", gettext("When saving the preferences file:"), e);
            }
        }
    }

    fn write(self: &mut Self) -> Result<(), Box<dyn Error>> {
        if let Some(preferences_file_path) = &self.preferences_file_path {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(preferences_file_path.as_str())?;
            let contents: String = toml::to_string(&self.preferences)?;
            file.write_all(contents.as_bytes())?;
            file.flush()?;
        }
        Ok(())
    }
}
