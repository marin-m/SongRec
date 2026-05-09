use app_dirs::{get_app_root, AppDataType::*, AppInfo};
use directories::ProjectDirs;
use std::error::Error;
use std::fs::create_dir_all;
#[cfg(not(windows))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;
use std::path::PathBuf;
use std::sync::LazyLock;

const QUALIFIER: &str = "";
const ORGANIZATION: &str = "SongRec";
const APPLICATION: &str = "SongRec";

static PROJECT_DIRS: LazyLock<ProjectDirs> =
    LazyLock::new(|| ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).unwrap());

pub fn obtain_recognition_history_csv_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut csv_path = obtain_data_directory()?;
    csv_path.push("song_history.csv");
    Ok(csv_path)
}

pub fn obtain_favorites_csv_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut csv_path = obtain_data_directory()?;
    csv_path.push("favorites.csv");
    Ok(csv_path)
}

pub fn obtain_preferences_file_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut preferences_file_path = obtain_preferences_directory()?;
    preferences_file_path.push("preferences.toml");
    Ok(preferences_file_path)
}

fn obtain_data_directory() -> Result<PathBuf, Box<dyn Error>> {
    let data_dir = PROJECT_DIRS.data_dir();
    if !data_dir.exists() {
        let old_dir = get_old_data_dir_path()?;
        if old_dir.exists() {
            #[cfg(not(windows))]
            symlink(old_dir, data_dir)?;
            #[cfg(windows)]
            symlink_dir(old_dir, data_dir)?;
        } else {
            create_dir_all(data_dir)?;
        }
    }
    Ok(data_dir.to_path_buf())
}

fn obtain_preferences_directory() -> Result<PathBuf, Box<dyn Error>> {
    let preferences_dir = PROJECT_DIRS.preference_dir();
    if !preferences_dir.exists() {
        create_dir_all(preferences_dir)?;
    }
    Ok(preferences_dir.to_path_buf())
}

pub fn obtain_cache_directory() -> Result<PathBuf, Box<dyn Error>> {
    let cache_path = PROJECT_DIRS.cache_dir();
    if !cache_path.exists() {
        create_dir_all(cache_path)?;
    }
    Ok(cache_path.to_path_buf())
}

pub fn clear_cache() {
    if let Ok(contents) = std::fs::read_dir(obtain_cache_directory().unwrap()) {
        for entry in contents.flatten() {
            if entry
                .file_name()
                .to_str()
                .unwrap_or("")
                .starts_with("songrec_cover_")
            {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }
}

// Backwards compatibility
fn get_old_data_dir_path() -> Result<PathBuf, Box<dyn Error>> {
    let app_info = AppInfo {
        name: "SongRec",
        author: "SongRec",
    };

    Ok(get_app_root(UserData, &app_info)?)
}
