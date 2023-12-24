use directories::ProjectDirs;
use std::fs::create_dir_all;
#[cfg(not(windows))]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;
use std::path::{PathBuf, Path};
use std::error::Error;
use app_dirs::{get_app_root, AppInfo, AppDataType::*};

const QUALIFIER: &str = "";
const ORGANIZATION: &str = "SongRec";
const APPLICATION: &str = "SongRec";

pub fn obtain_recognition_history_csv_path() -> Result<String, Box<dyn Error>> {
    let project_dir = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or("No valid path")?;
    let mut csv_path: PathBuf = obtain_data_directory(project_dir)?;
    csv_path.push("song_history.csv");
    Ok(csv_path.to_str().unwrap().to_string())
}

pub fn obtain_favorites_csv_path() -> Result<String, Box<dyn Error>> {
    let project_dir = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or("No valid path")?;
    let mut csv_path: PathBuf = obtain_data_directory(project_dir)?;
    csv_path.push("favorites.csv");
    Ok(csv_path.to_str().unwrap().to_string())
}


pub fn obtain_preferences_file_path() -> Result<String, Box<dyn Error>> {
    let project_dir = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or("No valid path")?;
    let mut preferences_file_path: PathBuf = obtain_preferences_directory(project_dir)?;
    preferences_file_path.push("preferences.toml");
    Ok(preferences_file_path.to_str().unwrap().to_string())
}

fn obtain_data_directory(project_directory: ProjectDirs) -> Result<PathBuf, Box<dyn Error>> {
    let data_dir: &Path = project_directory.data_dir();
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

fn obtain_preferences_directory(project_directory: ProjectDirs) -> Result<PathBuf, Box<dyn Error>> {
    let preferences_dir: &Path = project_directory.preference_dir();
    if !preferences_dir.exists() {
        create_dir_all(preferences_dir)?;
    }
    Ok(preferences_dir.to_path_buf())
}

//Backwards compatibility
fn get_old_data_dir_path() -> Result<PathBuf, Box<dyn Error>>{
    let app_info = AppInfo {
        name: "SongRec",
        author: "SongRec"
    };
    
    Ok(get_app_root(UserData, &app_info)?)
}
