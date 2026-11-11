use app_dirs::{AppDataType::*, AppInfo, get_app_root};
use directories::ProjectDirs;
use std::error::Error;
use std::fs::create_dir_all;
#[cfg(not(windows))]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
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
            migrate_legacy_data_directory(&old_dir, data_dir)?;
        } else {
            create_dir_all(data_dir)?;
        }
    }
    Ok(data_dir.to_path_buf())
}

fn migrate_legacy_data_directory(old_dir: &Path, data_dir: &Path) -> Result<(), Box<dyn Error>> {
    create_dir_all(data_dir.parent().unwrap_or(data_dir))?;
    #[cfg(not(windows))]
    symlink(old_dir, data_dir)?;
    #[cfg(windows)]
    {
        // Starting with an empty history is better than not starting, so a
        // failed copy is only logged, but the data directory must exist for
        // the history and favorites to be saved
        if let Err(error) = copy_directory_contents(old_dir, data_dir) {
            log::warn!(
                "Could not copy {} to {}: {}",
                old_dir.display(),
                data_dir.display(),
                error
            );
        }
        create_dir_all(data_dir)?;
    }
    Ok(())
}

/// Copy what `from` contains into `to`, which must not exist yet but whose
/// parent must. Without `content_only`, fs_extra would copy into
/// `to/<last component of from>` and fail, as `to` doesn't exist.
#[cfg(any(windows, test))]
fn copy_directory_contents(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    let options = fs_extra::dir::CopyOptions::new().content_only(true);
    fs_extra::dir::copy(from, to, &options)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A fresh, empty directory under the system temporary directory
    fn scratch_directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("songrec-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&path);
        create_dir_all(&path).unwrap();
        path
    }

    /// The layout left by older SongRec versions: a data directory holding
    /// the history, the favorites and a subdirectory
    fn legacy_data_directory(root: &Path) -> PathBuf {
        let old_dir = root.join("old").join("SongRec").join("SongRec");
        create_dir_all(old_dir.join("nested")).unwrap();
        fs::write(old_dir.join("song_history.csv"), "song_name\nhistory\n").unwrap();
        fs::write(old_dir.join("favorites.csv"), "song_name\nfavorite\n").unwrap();
        fs::write(old_dir.join("nested").join("file.txt"), "nested").unwrap();
        old_dir
    }

    fn assert_legacy_files_are_in(data_dir: &Path) {
        assert_eq!(
            fs::read_to_string(data_dir.join("song_history.csv")).unwrap(),
            "song_name\nhistory\n"
        );
        assert_eq!(
            fs::read_to_string(data_dir.join("favorites.csv")).unwrap(),
            "song_name\nfavorite\n"
        );
        assert_eq!(
            fs::read_to_string(data_dir.join("nested").join("file.txt")).unwrap(),
            "nested"
        );
        assert!(!data_dir.join("SongRec").exists());
    }

    #[test]
    fn migration_creates_missing_parents_and_keeps_the_legacy_files() {
        let root = scratch_directory("migration");
        let old_dir = legacy_data_directory(&root);
        let data_dir = root.join("new").join("SongRec").join("SongRec");

        migrate_legacy_data_directory(&old_dir, &data_dir).unwrap();

        assert!(data_dir.is_dir());
        assert_legacy_files_are_in(&data_dir);
        // The history can then be written where SongRec expects it
        csv::Writer::from_path(data_dir.join("song_history.csv"))
            .unwrap()
            .flush()
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_puts_the_contents_directly_into_the_new_directory() {
        let root = scratch_directory("copy");
        let old_dir = legacy_data_directory(&root);
        let data_dir = root.join("new").join("SongRec");
        create_dir_all(data_dir.parent().unwrap()).unwrap();

        copy_directory_contents(&old_dir, &data_dir).unwrap();

        assert!(data_dir.is_dir());
        assert_legacy_files_are_in(&data_dir);
        // The legacy directory is left untouched
        assert_legacy_files_are_in(&old_dir);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_reports_a_missing_legacy_directory() {
        let root = scratch_directory("copy-missing");
        let data_dir = root.join("SongRec");

        assert!(copy_directory_contents(&root.join("missing"), &data_dir).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
