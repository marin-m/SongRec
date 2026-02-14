use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use std::path::PathBuf;

pub fn setup_internationalization() {
    // Set up the translation/internationalization part

    let mut translations_path = std::env::current_exe().unwrap();

    // First, check for a "translations" directory in the
    // same directory as the current binary
    translations_path.pop();

    translations_path.push("translations");
    translations_path.push("locale");
    translations_path.push("fr_FR");
    translations_path.push("LC_MESSAGES");
    translations_path.push("songrec.mo");
    if !translations_path.is_file() {
        translations_path.pop();
        translations_path.pop();
        translations_path.pop();
        translations_path.pop();
        translations_path.pop();

        // Alternatively, check for a "../share/"
        // directory relative to the current binary, which would then
        // be assumed either to likely lay in a directory like
        // "/usr/bin" or "/usr/local/bin"
        translations_path.pop();
        translations_path.push("share");
        translations_path.push("locale");
        translations_path.push("fr_FR");
        translations_path.push("LC_MESSAGES");
        translations_path.push("songrec.mo");

        if !translations_path.is_file() {
            translations_path.pop();
            translations_path.pop();
            translations_path.pop();
            translations_path.pop();
            translations_path.pop();

            // Alternatively, check for a "translations" directory in
            // "../..", assuming that the current directory is
            // something like "target/release" or "target/debug" from
            // the root of the source tree
            translations_path.pop();

            translations_path.push("translations");
            translations_path.push("locale");
            translations_path.push("fr_FR");
            translations_path.push("LC_MESSAGES");
            translations_path.push("songrec.mo");
            if !translations_path.is_file() {
                // Alternatively, check in /usr/share

                translations_path = PathBuf::new();
                translations_path.push("usr");
                translations_path.push("share");
                translations_path.push("locale");
                translations_path.push("fr_FR");
                translations_path.push("LC_MESSAGES");
                translations_path.push("songrec.mo");

                translations_path.push("translations");
                if !translations_path.is_file() {
                    translations_path.pop();

                    // Alternatively, check in /usr/local/share

                    translations_path.pop();
                    translations_path.pop();
                    translations_path.pop();
                    translations_path.pop();
                    translations_path.pop();

                    translations_path.push("local");
                    translations_path.push("share");
                    translations_path.push("locale");
                    translations_path.push("fr_FR");
                    translations_path.push("LC_MESSAGES");
                    translations_path.push("songrec.mo");
                }
            }
        }
    }

    if translations_path.is_file() {
        translations_path.pop();
        translations_path.pop();
        translations_path.pop();

        textdomain("songrec");
        bindtextdomain("songrec", translations_path.to_str().unwrap());
        bind_textdomain_codeset("songrec", "UTF-8");

        setlocale(LocaleCategory::LcAll, "");
    }
}
