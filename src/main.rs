
#![windows_subsystem = "windows"]

pub mod cli_main;

mod fingerprinting {
    pub mod communication;
    pub mod algorithm;
    pub mod signature_format;
    mod user_agent;
    mod hanning;
}

mod core {
    pub mod microphone_thread;
    pub mod processing_thread;
    pub mod http_thread;
    pub mod logging;
    pub mod thread_messages;
}

mod audio_controllers {
    pub mod audio_backend;
    pub mod cpal;
    #[cfg(feature = "pulse")]
    pub mod pulseaudio;
}

#[cfg(feature = "gui")]
mod gui {
    pub mod main_window_v4;
    // mod song_history_interface; // <- To be revamped
    pub mod preferences;

    pub mod listed_device;
}

mod utils {
    pub mod csv_song_history;
    pub mod internationalization;

    #[cfg(feature = "gui")]
    pub mod filesystem_operations;

    #[cfg(feature = "ffmpeg")]
    pub mod ffmpeg_wrapper;

    #[cfg(feature = "mpris")]
    pub mod mpris_player;
}

use crate::fingerprinting::algorithm::SignatureGenerator;
use crate::fingerprinting::signature_format::DecodedSignature;
use crate::fingerprinting::communication::recognize_song_from_signature;

use crate::utils::internationalization::setup_internationalization;
#[cfg(feature = "gui")]
use crate::gui::main_window_v4::gui_main;
use crate::core::logging::Logging;
use crate::cli_main::{cli_main, CLIParameters, CLIOutputType};

use std::error::Error;
use gettextrs::gettext;
use clap::{command, Command, Arg, ArgAction};

macro_rules! base_app {
    () => {
        command!()
        .about(gettext("An open-source Shazam client for Linux, written in Rust."))
        /* .help_template("\
SongRec {version}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
") */
            // TODO handle: (2026-02-02)
            // -v or -v1, -v2, -v3 instead of -1, -2, -3
            // --log-file OUTPUT to output logs to a file in addition to stderr
            // (+ Add menu options in the GUI + Show debug logs in the ABOUT dialog)
                // => READ clap + fern documentations + integrate logging.rs + update main_window_v4.rs

                // Cf. https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
                    // ^ rewrite using this new syntax
                // Cf. https://docs.rs/clap/latest/clap/_tutorial/index.html
                // Cf. https://docs.rs/clap/latest/clap/
                // Cf. https://github.com/clap-rs/clap/tree/master/examples
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help(gettext("-v: Set the log level to DEBUG instead of WARN for SongRec-related messages
-vv: Set the log level to DEBUG for SongRec-related messages and INFO for library-related messages
-vvv: Set the log level to TRACE"))
        )
        .subcommand(
            Command::new("listen")
                .about(gettext("Run as a command-line program listening the microphone and printing recognized songs to stdout, exposing current song info via MPRIS"))
                .arg(
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .action(ArgAction::Set)
                        .help(gettext("Specify the audio device to use"))
                )
                .arg(
                    Arg::new("json")
                        .short('j')
                        .long("json")
                        .conflicts_with("csv")
                        .help(gettext("Enable printing full song info in JSON"))
                )
                .arg(
                    Arg::new("csv")
                        .short('c')
                        .long("csv")
                        .help(gettext("Enable printing full song info in the CSV format"))
                )
                .arg(
                    Arg::new("disable-mpris")
                        .long("disable-mpris")
                        .help(gettext("Disable MPRIS support"))
                )
        )
        .subcommand(
            Command::new("recognize")
                .about(gettext("Recognize one song from a sound file or microphone and print its info."))
                .arg(
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .action(ArgAction::Set)
                        .help(gettext("Specify the audio device to use"))
                )
                .arg(
                    Arg::new("json")
                        .short('j')
                        .long("json")
                        .conflicts_with("csv")
                        .help(gettext("Enable printing full song info in JSON"))
                )
                .arg(
                    Arg::new("csv")
                        .short('c')
                        .long("csv")
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
                    Arg::new("audio-device")
                        .short('d')
                        .long("audio-device")
                        .action(ArgAction::Set)
                        .help(gettext("Specify the audio device to use"))
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

#[cfg(feature="gui")]
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
                        .help(gettext("Disable MPRIS support"))
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
                        .help(gettext("Disable MPRIS support"))
                )
        )
    };
}

#[cfg(feature="gui")]
macro_rules! app {
    () => { gui_app!() };
}

#[cfg(not(feature="gui"))]
macro_rules! app {
    () => { base_app!() };
}

fn main() -> Result<(), Box<dyn Error>> {

    // Set up the translation/internationalization part

    setup_internationalization();

    // TODO simplify the code in this module etc. path handling ^

    // Collect the program arguments
    let args = app!().get_matches();

    // Set up logging

    let log_object: Logging = match args.get_count("verbose") {
        0 => Logging::setup_logging(log::LevelFilter::Warn, log::LevelFilter::Warn),
        1 => Logging::setup_logging(log::LevelFilter::Warn, log::LevelFilter::Debug),
        2 => Logging::setup_logging(log::LevelFilter::Info, log::LevelFilter::Debug),
        _ => Logging::setup_logging(log::LevelFilter::Trace, log::LevelFilter::Trace)
    };

    Logging::bind_glib_logging();

    // Parse other arguments
    
    match args.subcommand_name() {
        Some("audio-file-to-recognized-song") => {            
            let subcommand_args = args.subcommand_matches("audio-file-to-recognized-song").unwrap();
            
            let input_file_string = subcommand_args.get_one::<String>("input_file").unwrap();
            
            println!("{}", serde_json::to_string_pretty(&recognize_song_from_signature(&SignatureGenerator::make_signature_from_file(input_file_string)?)?)?);
        },
        Some("audio-file-to-fingerprint") => {
            let subcommand_args = args.subcommand_matches("audio-file-to-fingerprint").unwrap();
            
            let input_file_string = subcommand_args.get_one::<String>("input_file").unwrap();
            
            println!("{}", SignatureGenerator::make_signature_from_file(input_file_string)?.encode_to_uri()?);
        },
        Some("fingerprint-to-recognized-song") => {
            let subcommand_args = args.subcommand_matches("fingerprint-to-recognized-song").unwrap();
            
            let fingerprint_string = subcommand_args.get_one::<String>("fingerprint").unwrap();
            
            println!("{}", serde_json::to_string_pretty(&recognize_song_from_signature(&DecodedSignature::decode_from_uri(fingerprint_string)?)?)?);
        },
        Some("listen") => {
            let subcommand_args = args.subcommand_matches("listen").unwrap();
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();
            let enable_mpris = !subcommand_args.contains_id("disable-mpris");
            let enable_json = subcommand_args.contains_id("json");
            let enable_csv = subcommand_args.contains_id("csv");

            cli_main(CLIParameters {
                enable_mpris,
                recognize_once: false,
                audio_device,
                input_file: None,
                output_type: if enable_json {
                    CLIOutputType::JSON
                }
                else if enable_csv {
                    CLIOutputType::CSV
                }
                else {
                    CLIOutputType::SongName
                }
            })?;
        },
        Some("recognize") => {
            let subcommand_args = args.subcommand_matches("recognize").unwrap();
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();
            let input_file = subcommand_args.get_one::<String>("input_file").cloned();
            let enable_json = subcommand_args.contains_id("json");
            let enable_csv = subcommand_args.contains_id("csv");

            cli_main(CLIParameters {
                enable_mpris: false,
                recognize_once: true,
                audio_device,
                input_file,

                output_type: if enable_json {
                    CLIOutputType::JSON
                }
                else if enable_csv {
                    CLIOutputType::CSV
                }
                else {
                    CLIOutputType::SongName
                }
            })?;
        },
        Some("microphone-to-recognized-song") => {
            let subcommand_args = args.subcommand_matches("microphone-to-recognized-song").unwrap();
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();

            cli_main(CLIParameters {
                enable_mpris: false,
                recognize_once: true,
                audio_device,
                input_file: None,
                output_type: CLIOutputType::JSON
            })?;
        },
        #[cfg(feature="gui")]
        Some("gui-norecording") => {
            let subcommand_args = args.subcommand_matches("gui-norecording").unwrap();

            gui_main(log_object,
                 false,
                 subcommand_args.get_one::<String>("input_file").cloned(),
                 !subcommand_args.contains_id("disable-mpris"),
            )?;
        },
        #[cfg(feature="gui")]
        Some("gui") | None => {
            if let Some(subcommand_args) = args.subcommand_matches("gui") {
                gui_main(log_object,
                     true,
                     subcommand_args.get_one::<String>("input_file").cloned(),
                     !subcommand_args.contains_id("disable-mpris"),
                )?;
            }
            else {
                gui_main(log_object, true, None, true)?;
            }
        },
        #[cfg(not(feature="gui"))]
        None => {
            cli_main(CLIParameters {
                enable_mpris: true,
                recognize_once: false,
                audio_device: None,
                input_file: None,
                output_type: CLIOutputType::SongName
            })?;
        },
        _ => unreachable!()
    }
    
    Ok(())
    
}
