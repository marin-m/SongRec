use directories::ProjectDirs;
use std::fs::create_dir_all;
use std::path::{PathBuf, Path};
use std::error::Error;

pub fn obtain_csv_path() -> Result<String, Box<dyn Error>> {
    let project_dir = ProjectDirs::from("com", "Github", "SongRec").ok_or("No valid path")?;
    let mut csv_path: PathBuf = obtain_data_dir(project_dir)?;
    csv_path.push("song_history.csv");
    Ok(csv_path.to_str().unwrap().to_string())
}

pub fn obtain_preferences_file_path() -> Result<String, Box<dyn Error>> {
    let project_dir = ProjectDirs::from("com", "Github", "SongRec").ok_or("No valid path")?;
    let mut preferences_file_path: PathBuf = obtain_preferences_dir(project_dir)?;
    preferences_file_path.push("preferences.toml");
    Ok(preferences_file_path.to_str().unwrap().to_string())
}

fn obtain_data_dir(project_dir: ProjectDirs) -> Result<PathBuf, Box<dyn Error>> {
    let data_dir: &Path = project_dir.data_dir();
    create_dir_all(data_dir)?;
    Ok(data_dir.to_path_buf())
}

fn obtain_preferences_dir(project_dir: ProjectDirs) -> Result<PathBuf, Box<dyn Error>> {
    let preference_dir: &Path = project_dir.preference_dir();
    create_dir_all(preference_dir)?;
    Ok(preference_dir.to_path_buf())
}