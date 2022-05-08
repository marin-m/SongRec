use serde::Serialize;
use serde::Deserialize;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write};

use crate::utils::filesystem_reader::obtain_preferences_file_path;
#[derive(Serialize)]
#[derive(Deserialize)]
pub struct Preferences{
    enable_notifications: bool,
    device_name: String
}

pub struct PreferencesInterface{
    preferences_file_path: String,
    pub preferences: Preferences
}

impl PreferencesInterface {

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let preferences_file_path: String = obtain_preferences_file_path()?;
        match PreferencesInterface::load(preferences_file_path.as_str()) {
            Ok(preferences) => {
                return Ok(PreferencesInterface {
                    preferences_file_path: preferences_file_path,
                    preferences: preferences
                })
            },
            Err(e) => {
                return Ok(PreferencesInterface {
                    preferences_file_path: preferences_file_path,
                    preferences: Preferences {
                        enable_notifications: true,
                        device_name: "".to_string()
                    }
                })
            }
        }
    }


    fn load(preferences_file_path: &str) -> Result<Preferences, Box<dyn Error>> {
        let mut file: File = File::options().write(true).read(true).create(true).open(preferences_file_path)?;
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)?;
        let preferences: Preferences = toml::from_str(&contents)?;
        Ok(preferences)
    }

    pub fn update(self: &mut Self, preferences: &Preferences) -> Result<(), Box<dyn Error>> {
        let mut file: File = File::options().write(true).read(true).create(true).open(self.preferences_file_path.as_str())?;
        let contents: String = toml::to_string(preferences)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }
}