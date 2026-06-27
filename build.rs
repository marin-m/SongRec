use clap::{command, Arg, ArgAction, Command};
use flate2::Compression;
use flate2::GzBuilder;
use gettextrs::gettext;
use std::io::prelude::*;

// The below is copied from src/main.rs

macro_rules! base_app {
    () => {
        command!()
        .about(gettext("An open-source Shazam client for Linux, written in Rust."))
        .author("Marin M. - Fossplant.re")

        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help(gettext("-v: Set the log level to DEBUG instead of INFO for SongRec-related messages\n\
-vv: Set the log level to DEBUG for SongRec-related messages and INFO for library-related messages\n\
-vvv: Set the log level to TRACE"))
        )
        .subcommand(
            Command::new("listen")
                .about(gettext("Run as a command-line program listening the microphone and printing recognized songs to stdout, exposing current song info via MPRIS"))
                .arg(
                    Arg::new("list-devices")
                        .short('l')
                        .long("list-devices")
                        .action(ArgAction::SetTrue)
                        .help(gettext("List available audio devices and quit"))
                )
                .arg(
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .help(gettext("Specify the audio device to use"))
                )
                .arg(
                    Arg::new("request-interval")
                        .short('i')
                        .long("request-interval")
                        .default_value("10")
                        .value_parser(clap::value_parser!(u64))
                        .help(gettext("Shazam interval between requests in seconds (increase if you are rate-limited)"))
                )
                .arg(
                    Arg::new("json")
                        .short('j')
                        .long("json")
                        .conflicts_with("csv")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Enable printing full song info in JSON"))
                )
                .arg(
                    Arg::new("csv")
                        .short('c')
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Enable printing full song info in the CSV format"))
                )
                .arg(
                    Arg::new("disable-mpris")
                        .long("disable-mpris")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Disable MPRIS support"))
                )
        )
        .subcommand(
            Command::new("recognize")
                .about(gettext("Recognize one song from a sound file or microphone and print its info."))
                .arg(
                    Arg::new("list-devices")
                        .short('l')
                        .long("list-devices")
                        .action(ArgAction::SetTrue)
                        .help(gettext("List available audio devices and quit"))
                )
                .arg(
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .action(ArgAction::Set)
                        .help(gettext("Specify the audio device to use"))
                )
                .arg(
                    Arg::new("request-interval")
                        .short('i')
                        .long("request-interval")
                        .default_value("10")
                        .value_parser(clap::value_parser!(u64))
                        .help(gettext("Shazam interval between requests in seconds (increase if you are rate-limited)"))
                )
                .arg(
                    Arg::new("json")
                        .short('j')
                        .long("json")
                        .conflicts_with("csv")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Enable printing full song info in JSON"))
                )
                .arg(
                    Arg::new("csv")
                        .short('c')
                        .long("csv")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Enable printing full song info in the CSV format"))
                )
                .arg(
                    Arg::new("input_file")
                        .required(false)
                        .help(gettext("Recognize a file instead of using mic input"))
                )
        )
        .subcommand(
            Command::new("audio-file-to-recognized-song")
                .about(gettext("Generate a Shazam fingerprint from a sound file, perform song recognition towards Shazam's servers and print obtained information to the standard output."))
                .arg(
                    Arg::new("input_file")
                        .required(true)
                        .help(gettext("The audio file to recognize."))
                )
        )
        .subcommand(
            Command::new("microphone-to-recognized-song")
                .about(gettext("Recognize a currently playing song using the microphone and print obtained information to the standard output"))
                .arg(
                    Arg::new("list-devices")
                        .short('l')
                        .long("list-devices")
                        .action(ArgAction::SetTrue)
                        .help(gettext("List available audio devices and quit"))
                )
                .arg(
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .help(gettext("Specify the audio device to use"))
                )
                .arg(
                    Arg::new("request-interval")
                        .short('i')
                        .long("request-interval")
                        .default_value("10")
                        .value_parser(clap::value_parser!(u64))
                        .help(gettext("Shazam interval between requests in seconds (increase if you are rate-limited)"))
                )
        )
        .subcommand(
            Command::new("audio-file-to-fingerprint")
                .about(gettext("Generate a Shazam fingerprint from a sound file, and print it to the standard output."))
                .arg(
                    Arg::new("input_file")
                        .required(true)
                        .help(gettext("The .WAV or .MP3 file to generate an audio fingerprint for."))
                )
        )
        .subcommand(
            Command::new("fingerprint-to-recognized-song")
                .about(gettext("Take a data-URI Shazam fingerprint, perform song recognition towards Shazam's servers and print obtained information to the standard output."))
                .arg(
                    Arg::new("fingerprint")
                        .required(true)
                        .help(gettext("The data-URI Shazam fingerprint to recognize."))
                )
        )
    };
}

#[cfg(feature = "gui")]
macro_rules! gui_app {
    () => {
        base_app!()
        .subcommand(
            Command::new("gui")
                .about(gettext("The default action. Display a GUI."))
                .arg(
                    Arg::new("input_file")
                        .required(false)
                        .help(gettext("An optional audio file to recognize on the launch of the application."))
                )
                .arg(
                    Arg::new("disable-mpris")
                        .long("disable-mpris")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Disable MPRIS support"))
                )
                .arg(
                    Arg::new("disable-pipewire")
                        .long("disable-pipewire")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Disable PipeWire native support"))
                )
        )
        .subcommand(
            Command::new("gui-norecording")
                .about(gettext("Launch the GUI, but don't recognize audio through the microphone as soon as it is launched (rather than expecting the user to click on a button)."))
                .arg(
                    Arg::new("input_file")
                        .required(false)
                        .help(gettext("An optional audio file to recognize on the launch of the application."))
                )
                .arg(
                    Arg::new("disable-mpris")
                        .long("disable-mpris")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Disable MPRIS support"))
                )
                .arg(
                    Arg::new("disable-pipewire")
                        .long("disable-pipewire")
                        .action(ArgAction::SetTrue)
                        .help(gettext("Disable PipeWire native support"))
                )
        )
    };
}

#[cfg(feature = "gui")]
macro_rules! app {
    () => {
        gui_app!()
    };
}

#[cfg(not(feature = "gui"))]
macro_rules! app {
    () => {
        base_app!()
    };
}

fn main() {
    // Lint source code

    #[cfg(target_os = "linux")]
    std::process::Command::new("cargo").arg("fmt").status().ok();

    // Regenerate .po, .mo, .pot translation
    // files from the source tree

    #[cfg(target_os = "linux")]
    if !std::process::Command::new("./update_po_files.sh")
        .current_dir("translations")
        .status()
        .unwrap()
        .success()
    {
        println!(
            "cargo:warning=Running \"./translations/./update_po_files.sh\" \
did not succeed, please think about running it yourself in order to \
troubleshoot the error"
        );
    }

    // Regenerate GTK Builder .ui files from
    // GNOME Builder .blp files

    #[cfg(target_os = "linux")]
    if !std::process::Command::new("blueprint-compiler")
        .current_dir("src/gui")
        .args([
            "compile",
            "--output",
            "interface-autogenerated.ui",
            "interface.blp",
        ])
        .status()
        .unwrap()
        .success()
    {
        println!(
            "cargo:warning=Compiling the Blueprint resource file did not succeed, \
please ensure that your version of \"blueprint-compiler\" is recent enough \
and try out of this build process"
        );
    }

    println!("cargo:rerun-if-changed=src/gui/interface.blp");

    // Generate GLib resources

    glib_build_tools::compile_resources(
        &["src/gui"],
        "src/gui/resources.gresource.xml",
        "compiled.gresource",
    );

    // Generate manpages

    let temp_out_dir = tempfile::tempdir().unwrap();

    let real_out_dir = std::path::PathBuf::from(".")
        .join("packaging")
        .join("rootfs")
        .join("usr")
        .join("share")
        .join("man")
        .join("man1");

    std::fs::create_dir_all(&real_out_dir).unwrap();

    clap_mangen::generate_to(app!(), temp_out_dir.path()).unwrap();

    // Compress the man pages

    for path in std::fs::read_dir(temp_out_dir.path()).unwrap() {
        let path_str = path.unwrap().path().display().to_string();

        if path_str.ends_with(".1") {
            let out_file = std::fs::File::create(format!("{}.gz", path_str)).unwrap();

            let mut gzipper = GzBuilder::new().write(out_file, Compression::best());
            gzipper
                .write_all(&std::fs::read(&path_str).unwrap())
                .unwrap();
            gzipper.finish().unwrap();

            std::fs::remove_file(path_str).unwrap();
        }
    }

    // Mode the modified or added man pages to the source tree

    for in_path in std::fs::read_dir(temp_out_dir.path()).unwrap() {
        if let Ok(in_path) = in_path {
            let out_path = real_out_dir.join(in_path.file_name());
            std::fs::copy(in_path.path(), out_path).unwrap();
        }
    }
}
