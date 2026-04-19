use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use log::warn;
use std::path::PathBuf;

/// Set up the translation/internationalization part
pub fn setup_internationalization() -> Option<PathBuf> {
    // First, check for a "translations" directory in the
    // same directory as the current binary

    let mut translations_path = std::env::current_exe().unwrap();
    translations_path.pop();

    translations_path.push("translations");
    translations_path.push("locale");

    if !translations_path.is_dir() {
        // Alternatively, check for a "translations" directory in
        // "../..", assuming that the current directory is
        // something like "target/release" or "target/debug" from
        // the root of the source tree

        translations_path = std::env::current_exe().unwrap();
        translations_path.pop();

        translations_path.pop();
        translations_path.pop();

        translations_path.push("translations");
        translations_path.push("locale");

        if !translations_path.is_dir() {
            // Alternatively, check for a "../share/"
            // directory relative to the current binary, which would then
            // be assumed either to likely lay in a directory like
            // "/usr/bin" or "/usr/local/bin"

            translations_path = std::env::current_exe().unwrap();
            translations_path.pop();

            translations_path.pop();

            translations_path.push("share");
            translations_path.push("locale");
        }
    }

    if translations_path.is_dir() {
        if let Err(error) = bindtextdomain("songrec", translations_path.to_str().unwrap()) {
            warn!("Failed to run bindtextdomain: {:?}", error);
        }
    }

    if let Err(error) = textdomain("songrec") {
        warn!("Failed to run textdomain: {:?}", error);
    }
    if let Err(error) = bind_textdomain_codeset("songrec", "UTF-8") {
        warn!("Failed to run bind_textdomain_codeset: {:?}", error);
    }

    setlocale(LocaleCategory::LcAll, "");

    if translations_path.is_dir() {
        Some(translations_path)
    } else {
        None
    }
}
