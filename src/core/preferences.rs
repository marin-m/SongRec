use gettextrs::gettext;
use log::{debug, error};
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::path::PathBuf;

use crate::utils::filesystem_operations::{obtain_preferences_file_path, write_atomically};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Preferences {
    pub enable_notifications: Option<bool>,
    pub enable_background: Option<bool>,
    pub enable_systray: Option<bool>,
    pub enable_autostart: Option<bool>,
    pub start_minimized: Option<bool>,
    pub enable_mpris: Option<bool>, // Legacy, before setting default to true
    pub enable_mpris_v2: Option<bool>,
    pub remember_window_state: Option<bool>,
    pub no_duplicates: Option<bool>,
    pub buffer_size_secs: Option<u64>,         // Removed in 0.7.3
    pub request_interval_secs: Option<u64>,    // Legacy, before increasing default from 4 to 10
    pub request_interval_secs_v2: Option<u64>, // before decreasing from 10 to 8
    pub request_interval_secs_v3: Option<u64>,
    pub current_device_name: Option<String>,
    pub website_search_url: Option<String>,
    pub website_search_text: Option<String>,
    pub window_width: Option<i32>,
    pub window_height: Option<i32>,
    pub window_maximized: Option<bool>,
}

impl Preferences {
    pub fn new() -> Self {
        Preferences {
            enable_notifications: None,
            enable_background: None,
            enable_systray: None,
            enable_autostart: None,
            start_minimized: None,
            enable_mpris: None,
            enable_mpris_v2: None,
            remember_window_state: None,
            no_duplicates: None,
            buffer_size_secs: None,
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: None,
            current_device_name: None,
            website_search_url: None,
            website_search_text: None,
            window_width: None,
            window_height: None,
            window_maximized: None,
        }
    }

    pub fn with_interval(interval: u64) -> Self {
        Preferences {
            enable_notifications: Some(true),
            enable_background: Some(false),
            enable_systray: Some(false),
            enable_autostart: Some(false),
            start_minimized: Some(false),
            enable_mpris: None,
            enable_mpris_v2: Some(true),
            remember_window_state: Some(false),
            no_duplicates: Some(false),
            buffer_size_secs: None,
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: Some(interval),
            current_device_name: None,
            website_search_url: Some("https://www.youtube.com/results?search_query=".to_string()),
            website_search_text: Some(gettext("Search on YouTube".to_string())),
            window_width: None,
            window_height: None,
            window_maximized: None,
        }
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            enable_notifications: Some(true),
            enable_background: Some(false),
            enable_systray: Some(false),
            enable_autostart: Some(false),
            start_minimized: Some(false),
            enable_mpris: None,
            enable_mpris_v2: Some(true),
            remember_window_state: Some(false),
            no_duplicates: Some(false),
            buffer_size_secs: None,
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: Some(8),
            current_device_name: None,
            website_search_url: Some("https://www.youtube.com/results?search_query=".to_string()),
            website_search_text: Some(gettext("Search on YouTube".to_string())),
            window_width: None,
            window_height: None,
            window_maximized: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreferencesInterface {
    pub preferences_file_path: Option<PathBuf>,
    pub preferences: Preferences,
}

impl PreferencesInterface {
    pub fn new() -> Self {
        match PreferencesInterface::load() {
            Ok(preferences_interface) => preferences_interface,
            Err(e) => {
                error!("{} {}", gettext("When parsing the preferences file:"), e);
                PreferencesInterface {
                    preferences_file_path: obtain_preferences_file_path().ok(),
                    preferences: Preferences::default(),
                }
            }
        }
    }

    fn load() -> Result<PreferencesInterface, Box<dyn Error>> {
        let preferences_file_path = obtain_preferences_file_path()?;
        let contents = std::fs::read_to_string(&preferences_file_path).unwrap_or_default();
        let preferences: Preferences = toml::from_str(&contents)?;
        debug!(
            "Loaded preferences from {}: {:?}",
            preferences_file_path.display(),
            preferences
        );
        Ok(PreferencesInterface {
            preferences_file_path: Some(preferences_file_path),
            preferences,
        })
    }

    pub fn update(&mut self, update_preferences: Preferences) {
        let current_preferences = &self.preferences;
        self.preferences = Preferences {
            enable_notifications: update_preferences
                .enable_notifications
                .or(current_preferences.enable_notifications),
            enable_mpris: None,
            enable_mpris_v2: update_preferences
                .enable_mpris_v2
                .or(current_preferences.enable_mpris_v2)
                .or(current_preferences.enable_mpris),
            enable_background: update_preferences
                .enable_background
                .or(current_preferences.enable_background),
            enable_systray: update_preferences
                .enable_systray
                .or(current_preferences.enable_systray),
            enable_autostart: update_preferences
                .enable_autostart
                .or(current_preferences.enable_autostart),
            start_minimized: update_preferences
                .start_minimized
                .or(current_preferences.start_minimized),
            remember_window_state: update_preferences
                .remember_window_state
                .or(current_preferences.remember_window_state),
            no_duplicates: update_preferences
                .no_duplicates
                .or(current_preferences.no_duplicates),
            buffer_size_secs: None,
            request_interval_secs: None,
            request_interval_secs_v2: None,
            request_interval_secs_v3: update_preferences
                .request_interval_secs_v3
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
                .or_else(|| current_preferences.current_device_name.clone()),
            website_search_url: update_preferences
                .website_search_url
                .or_else(|| current_preferences.website_search_url.clone()),
            website_search_text: update_preferences
                .website_search_text
                .or_else(|| current_preferences.website_search_text.clone()),
            window_width: update_preferences
                .window_width
                .or(current_preferences.window_width),
            window_height: update_preferences
                .window_height
                .or(current_preferences.window_height),
            window_maximized: update_preferences
                .window_maximized
                .or(current_preferences.window_maximized),
        };
        if let Err(error) = self.write() {
            error!("{} {}", gettext("When saving the preferences file:"), error);
        }
    }

    fn write(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(preferences_file_path) = &self.preferences_file_path {
            let contents: String = toml::to_string(&self.preferences)?;
            write_atomically(preferences_file_path, |file| {
                std::io::Write::write_all(file, contents.as_bytes())?;
                Ok(())
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interface_without_file(preferences: Preferences) -> PreferencesInterface {
        PreferencesInterface {
            preferences_file_path: None,
            preferences,
        }
    }

    #[test]
    fn window_size_is_unset_in_older_preference_files() {
        let preferences: Preferences =
            toml::from_str("enable_notifications = true\nrequest_interval_secs_v3 = 8\n").unwrap();

        assert_eq!(preferences.window_width, None);
        assert_eq!(preferences.window_height, None);
        assert_eq!(preferences.window_maximized, None);
    }

    #[test]
    fn window_size_survives_a_round_trip_through_the_file() {
        let preferences = Preferences {
            window_width: Some(700),
            window_height: Some(420),
            window_maximized: Some(true),
            remember_window_state: Some(true),
            ..Default::default()
        };

        let read: Preferences = toml::from_str(&toml::to_string(&preferences).unwrap()).unwrap();

        assert_eq!(read.window_width, Some(700));
        assert_eq!(read.window_height, Some(420));
        assert_eq!(read.window_maximized, Some(true));
        assert_eq!(read.remember_window_state, Some(true));
    }

    #[test]
    fn updating_the_window_size_keeps_the_other_preferences() {
        let mut interface = interface_without_file(Preferences::default());

        let mut new_preference = Preferences::new();
        new_preference.window_width = Some(700);
        new_preference.window_height = Some(420);
        new_preference.window_maximized = Some(false);
        new_preference.remember_window_state = Some(true);
        interface.update(new_preference);

        assert_eq!(interface.preferences.window_width, Some(700));
        assert_eq!(interface.preferences.window_height, Some(420));
        assert_eq!(interface.preferences.window_maximized, Some(false));
        assert_eq!(interface.preferences.remember_window_state, Some(true));
        assert_eq!(interface.preferences.enable_notifications, Some(true));
        assert_eq!(interface.preferences.request_interval_secs_v3, Some(8));
    }

    #[test]
    fn updating_other_preferences_keeps_the_window_size() {
        let preferences = Preferences {
            window_width: Some(700),
            window_height: Some(420),
            remember_window_state: Some(true),

            window_maximized: Some(true),
            ..Default::default()
        };
        let mut interface = interface_without_file(preferences);

        let mut new_preference = Preferences::new();
        new_preference.no_duplicates = Some(true);
        interface.update(new_preference);

        assert_eq!(interface.preferences.no_duplicates, Some(true));
        assert_eq!(interface.preferences.window_width, Some(700));
        assert_eq!(interface.preferences.window_height, Some(420));
        assert_eq!(interface.preferences.window_maximized, Some(true));
    }
}
