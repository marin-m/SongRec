use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use log::warn;
use std::path::{Path, PathBuf};

/// Set up the translation/internationalization part
pub fn setup_internationalization() -> Option<PathBuf> {
    // Paths are kept as `Path`: the directory of SongRec may not be UTF-8
    let translations_path = std::env::current_exe()
        .ok()
        .and_then(|current_exe| find_translations_directory(&current_exe));

    if let Some(ref translations_path) = translations_path
        && let Err(error) = bindtextdomain("songrec", translations_path.clone())
    {
        warn!("Failed to run bindtextdomain: {:?}", error);
    }

    if let Err(error) = textdomain("songrec") {
        warn!("Failed to run textdomain: {:?}", error);
    }
    if let Err(error) = bind_textdomain_codeset("songrec", "UTF-8") {
        warn!("Failed to run bind_textdomain_codeset: {:?}", error);
    }

    unsafe {
        setlocale(LocaleCategory::LcAll, "");
    }

    translations_path
}

/// The first existing translations directory for the binary at CURRENT_EXE
fn find_translations_directory(current_exe: &Path) -> Option<PathBuf> {
    translations_directory_candidates(current_exe)
        .into_iter()
        .find(|path| path.is_dir())
}

/// Where translations are looked for, in order: a "translations" directory
/// next to the binary, then in "../.." (assuming that the binary is in
/// something like "target/release" or "target/debug" from the root of the
/// source tree), then a "../share" directory (for a binary in a directory
/// like "/usr/bin" or "/usr/local/bin")
fn translations_directory_candidates(current_exe: &Path) -> Vec<PathBuf> {
    let Some(binary_dir) = current_exe.parent() else {
        return Vec::new();
    };
    let mut candidates = vec![binary_dir.join("translations").join("locale")];
    if let Some(source_root) = binary_dir.parent().and_then(Path::parent) {
        candidates.push(source_root.join("translations").join("locale"));
    }
    if let Some(prefix) = binary_dir.parent() {
        candidates.push(prefix.join("share").join("locale"));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn translation_directories_are_found_next_to_a_binary_whose_path_is_not_utf8() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = |bytes: &[u8]| PathBuf::from(OsStr::from_bytes(bytes));
        assert_eq!(
            translations_directory_candidates(&path(b"/opt/caf\xe9/bin/songrec")),
            [
                path(b"/opt/caf\xe9/bin/translations/locale"),
                path(b"/opt/translations/locale"),
                path(b"/opt/caf\xe9/share/locale"),
            ]
        );
    }

    #[test]
    fn the_first_existing_translation_directory_is_used() {
        let root =
            std::env::temp_dir().join(format!("songrec-translations-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let exe = root.join("prefix").join("bin").join("songrec");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();

        assert_eq!(find_translations_directory(&exe), None);

        let share = root.join("prefix").join("share").join("locale");
        std::fs::create_dir_all(&share).unwrap();
        assert_eq!(find_translations_directory(&exe), Some(share));

        let next_to_binary = root
            .join("prefix")
            .join("bin")
            .join("translations")
            .join("locale");
        std::fs::create_dir_all(&next_to_binary).unwrap();
        assert_eq!(find_translations_directory(&exe), Some(next_to_binary));
        std::fs::remove_dir_all(root).unwrap();
    }
}
