use gettextrs::{textdomain, bindtextdomain, bind_textdomain_codeset, setlocale, LocaleCategory};
use std::path::PathBuf;

pub fn setup_internationalization() {

    // Set up the translation/internationalization part
    
    let mut translations_dir_path = std::env::current_exe().unwrap();
    
    // First, check for a "translations" directory in the
    // same directory as the current binary
    translations_dir_path.pop();
    
    translations_dir_path.push("translations");
    if !translations_dir_path.is_dir() {
        translations_dir_path.pop();
        
        // Alternatively, check for a "../share/songrec/translations"
        // directory relative to the current binary, which would then
        // be assumed either to likely lay in a directory like
        // "/usr/bin" or "/usr/local/bin"
        translations_dir_path.pop();
        translations_dir_path.push("share");
        translations_dir_path.push("songrec");
        
        translations_dir_path.push("translations");
        if !translations_dir_path.is_dir() {
            translations_dir_path.pop();
            
            translations_dir_path.pop();
            translations_dir_path.pop();
            
            // Alternatively, check for a "translations" directory in
            // "../..", assuming that the current directory is
            // something like "target/release" or "target/debug" from
            // the root of the source tree
            translations_dir_path.pop();
            
            translations_dir_path.push("translations");
            if !translations_dir_path.is_dir() {
                translations_dir_path.pop();
                
                // Alternatively, check in /usr/share
                
                translations_dir_path = PathBuf::new();
                translations_dir_path.push("usr");
                translations_dir_path.push("share");
                translations_dir_path.push("songrec");
                
                translations_dir_path.push("translations");
                if !translations_dir_path.is_dir() {
                    translations_dir_path.pop();
                    
                    // Alternatively, check in /usr/local/share
                    
                    translations_dir_path.pop();
                    translations_dir_path.pop();
                    
                    translations_dir_path.push("local");
                    translations_dir_path.push("share");
                    translations_dir_path.push("songrec");
                    
                    translations_dir_path.push("translations");
                }
            }
        }
    }
    
    if translations_dir_path.is_dir() {
        textdomain("songrec");
        bindtextdomain("songrec", translations_dir_path.to_str().unwrap());
        bind_textdomain_codeset("songrec", "UTF-8");
        
        setlocale(LocaleCategory::LcAll, "");
    }
    
}
