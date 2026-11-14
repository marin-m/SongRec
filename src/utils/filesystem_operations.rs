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
    create_directory(PROJECT_DIRS.cache_dir())
}

fn create_directory(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !path.exists() {
        create_dir_all(path)?;
    }
    Ok(path.to_path_buf())
}

pub fn clear_cache() {
    clear_cache_directory(obtain_cache_directory());
}

/// Remove the cover files that MPRIS left in CACHE_DIR. This runs at startup,
/// so a cache directory that can't be created (for example when ~/.cache
/// belongs to another user) is only logged.
fn clear_cache_directory(cache_dir: Result<PathBuf, Box<dyn Error>>) {
    let cache_dir = match cache_dir {
        Ok(cache_dir) => cache_dir,
        Err(error) => {
            log::warn!("Could not open the cache directory: {}", error);
            return;
        }
    };
    if let Ok(contents) = std::fs::read_dir(cache_dir) {
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

/// The file:// URI of PATH, with the characters that URIs reserve, such as
/// spaces, `#` and `%`, percent-encoded so that clients read the same path
#[cfg(any(all(target_os = "linux", feature = "mpris"), test))]
pub fn file_uri(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(glib::filename_to_uri(path, None)?.to_string())
}

/// Write PATH through WRITE, into a temporary file of the same directory that
/// replaces PATH only once it is complete. Writing PATH directly truncates it
/// first, so an error or a crash midway would leave the history, the
/// favorites or the preferences cut short.
pub fn write_atomically(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut builder = tempfile::Builder::new();
    // tempfile creates files as 0o600. Give the new file the mode PATH has,
    // or the 0o666 that `File::create` uses, so that the umask still decides
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(match std::fs::metadata(path) {
            Ok(metadata) => metadata.permissions(),
            Err(_) => std::fs::Permissions::from_mode(0o666),
        });
    }
    let mut file = builder.tempfile_in(directory)?;
    write(file.as_file_mut())?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

/// The audio file that a command line subcommand reads: the given file, or a
/// temporary copy of the standard input when the file is `-`. The decoders
/// seek in their input, which a pipe doesn't allow, hence the copy.
pub struct AudioInput {
    path: PathBuf,
    // Deletes the copy of the standard input when dropped
    _stdin_copy: Option<tempfile::TempPath>,
}

impl AudioInput {
    pub fn new(input_file: &Path) -> std::io::Result<AudioInput> {
        if input_file != Path::new("-") {
            return Ok(AudioInput {
                path: input_file.to_path_buf(),
                _stdin_copy: None,
            });
        }
        let stdin_copy = copy_to_temp_file(std::io::stdin().lock(), &std::env::temp_dir())?;
        Ok(AudioInput {
            path: stdin_copy.to_path_buf(),
            _stdin_copy: Some(stdin_copy),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn copy_to_temp_file(
    mut reader: impl std::io::Read,
    directory: &Path,
) -> std::io::Result<tempfile::TempPath> {
    let mut file = tempfile::Builder::new()
        .prefix("songrec-stdin-")
        .tempfile_in(directory)?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(file.into_temp_path())
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

    /// A path under a regular file, so no directory can be created there
    fn uncreatable_directory(root: &Path) -> PathBuf {
        let file = root.join("file");
        fs::write(&file, "").unwrap();
        file.join("cache")
    }

    #[test]
    fn clearing_the_cache_removes_only_cached_covers() {
        let root = scratch_directory("cache");
        fs::write(root.join("songrec_cover_1000_1.jpg"), "cover").unwrap();
        fs::write(root.join("other.txt"), "other").unwrap();

        clear_cache_directory(Ok(root.clone()));

        assert!(!root.join("songrec_cover_1000_1.jpg").exists());
        assert!(root.join("other.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_cache_directory_that_cannot_be_created_is_an_error() {
        let root = scratch_directory("cache-uncreatable");
        let cache_dir = uncreatable_directory(&root);

        let result = create_directory(&cache_dir);
        assert!(result.is_err());
        // Clearing it is logged instead of panicking
        clear_cache_directory(result);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_uris_read_back_as_the_same_path() {
        for path in ["/tmp/plain.jpg", "/tmp/a b#c%d.jpg", "/tmp/\u{e9}.jpg"] {
            let uri = file_uri(Path::new(path)).unwrap();
            assert!(uri.starts_with("file:///"), "{uri}");
            let (read_back, _) = glib::filename_from_uri(&uri).unwrap();
            assert_eq!(read_back, Path::new(path));
        }
        assert_eq!(
            file_uri(Path::new("/tmp/a b#c%d.jpg")).unwrap(),
            "file:///tmp/a%20b%23c%25d.jpg"
        );
        // Without the encoding, the path stopped at `#`
        let (read_back, _) = glib::filename_from_uri("file:///tmp/a b#c%d.jpg").unwrap();
        assert_eq!(read_back, Path::new("/tmp/a b"));
    }

    #[test]
    fn an_atomic_write_replaces_the_file_once_complete() {
        let root = scratch_directory("atomic");
        let path = root.join("song_history.csv");
        fs::write(&path, "old").unwrap();

        write_atomically(&path, |file| {
            use std::io::Write;
            file.write_all(b"new")?;
            Ok(())
        })
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_atomic_write_keeps_the_permissions_a_direct_write_would_give() {
        use std::os::unix::fs::PermissionsExt;
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let write = |path: &Path| {
            write_atomically(path, |file| {
                use std::io::Write;
                file.write_all(b"new")?;
                Ok(())
            })
            .unwrap()
        };
        let root = scratch_directory("atomic-permissions");

        // A new file gets the same mode as one created by `fs::write`, that is
        // 0o666 restricted by the umask
        let reference = root.join("reference.csv");
        fs::write(&reference, "").unwrap();
        let new_file = root.join("new.csv");
        write(&new_file);
        assert_eq!(mode(&new_file), mode(&reference));

        // An existing file keeps its mode
        let existing = root.join("existing.csv");
        fs::write(&existing, "old").unwrap();
        fs::set_permissions(&existing, fs::Permissions::from_mode(0o640)).unwrap();
        write(&existing);
        assert_eq!(mode(&existing), 0o640);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_atomic_write_keeps_the_previous_file() {
        let root = scratch_directory("atomic-failed");
        let path = root.join("song_history.csv");
        fs::write(&path, "old").unwrap();

        let result = write_atomically(&path, |file| {
            use std::io::Write;
            file.write_all(b"partial")?;
            Err("serialization failed".into())
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old");
        // The temporary file is removed too
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn audio_inputs_other_than_a_dash_are_read_as_they_are() {
        let input = AudioInput::new(Path::new("song.mp3")).unwrap();
        assert_eq!(input.path(), Path::new("song.mp3"));
        let input = AudioInput::new(Path::new("./-")).unwrap();
        assert_eq!(input.path(), Path::new("./-"));
    }

    #[test]
    fn the_standard_input_copy_holds_all_of_it_and_is_deleted_when_dropped() {
        let root = scratch_directory("stdin-copy");
        // Bigger than the buffer of `io::copy`
        let contents: Vec<u8> = (0..100_000u32).map(|i| i as u8).collect();

        let copy = copy_to_temp_file(&contents[..], &root).unwrap();
        let path = copy.to_path_buf();
        assert!(path.starts_with(&root));
        assert_eq!(fs::read(&path).unwrap(), contents);

        drop(copy);
        assert!(!path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_empty_standard_input_gives_an_empty_copy() {
        let root = scratch_directory("stdin-empty");
        let copy = copy_to_temp_file(std::io::empty(), &root).unwrap();
        assert_eq!(fs::read(&copy).unwrap(), b"");
        drop(copy);
        fs::remove_dir_all(root).unwrap();
    }
}
