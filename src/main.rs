#![windows_subsystem = "windows"]

pub mod cli_main;

mod fingerprinting {
    pub mod algorithm;
    pub mod communication;
    mod hanning;
    pub mod signature_format;
    mod user_agent;
}

mod core {
    pub mod http_task;
    pub mod logging;
    pub mod microphone_thread;
    pub mod preferences;
    pub mod processing_thread;
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
    pub mod main_window;
    pub mod song_history_interface;

    pub mod context_menu;
    pub mod history_entry;
    pub mod listed_device;
}

mod utils {
    pub mod csv_song_history;
    pub mod filesystem_operations;
    pub mod internationalization;

    #[cfg(feature = "ffmpeg")]
    pub mod ffmpeg_wrapper;
}

mod plugins {
    #[cfg(feature = "gui")]
    #[cfg(target_os = "linux")]
    pub mod ksni;
    #[cfg(feature = "mpris")]
    pub mod mpris_player;
}

use crate::fingerprinting::algorithm::SignatureGenerator;
use crate::fingerprinting::communication::recognize_song_from_signature;
use crate::fingerprinting::signature_format::DecodedSignature;

use crate::cli_main::{cli_main, CLIOutputType, CLIParameters};
use crate::core::logging::Logging;
#[cfg(feature = "gui")]
use crate::gui::main_window::gui_main;
use crate::utils::internationalization::setup_internationalization;

use clap::{command, Arg, ArgAction, Command};
use gettextrs::gettext;
use log::{debug, error};
use soup::prelude::SessionExt;
use std::error::Error;

macro_rules! base_app {
    () => {
        command!()
        .about(gettext("An open-source Shazam client for Linux, written in Rust."))
        .author("Marin M. - Fossplant.re")
        /* .help_template("\
SongRec {version}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
") */

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

fn main() -> Result<(), Box<dyn Error>> {
    // Set up the translation/internationalization part

    let i18n_folder = setup_internationalization();

    // TODO simplify the code in this module etc. path handling ^

    // Collect the program arguments

    let args = app!().get_matches();

    // Set up logging

    let log_object: Logging = match args.get_count("verbose") {
        0 => Logging::setup_logging(log::LevelFilter::Warn, log::LevelFilter::Info),
        1 => Logging::setup_logging(log::LevelFilter::Warn, log::LevelFilter::Debug),
        2 => Logging::setup_logging(log::LevelFilter::Info, log::LevelFilter::Debug),
        _ => Logging::setup_logging(log::LevelFilter::Trace, log::LevelFilter::Trace),
    };

    Logging::bind_glib_logging();

    match i18n_folder {
        Some(path) => {
            debug!("Translations folder found at: {}", path.to_str().unwrap());
        }
        None => {
            error!("No usable translations folder found");
        }
    };

    // Parse other arguments

    match args.subcommand_name() {
        Some("audio-file-to-recognized-song") => {
            let subcommand_args = args
                .subcommand_matches("audio-file-to-recognized-song")
                .unwrap();

            let session = soup::Session::new();
            session.set_timeout(20);

            let input_file_string = subcommand_args
                .get_one::<String>("input_file")
                .unwrap()
                .clone();

            let main_loop = glib::MainLoop::new(None, false);
            let main_loop_inner = main_loop.clone();
            glib::spawn_future_local(async move {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &recognize_song_from_signature(
                            &session,
                            &SignatureGenerator::make_signature_from_file(&input_file_string)
                                .unwrap()
                        )
                        .await
                        .unwrap()
                    )
                    .unwrap()
                );
                main_loop_inner.quit();
            });
            main_loop.run();
        }
        Some("audio-file-to-fingerprint") => {
            let subcommand_args = args
                .subcommand_matches("audio-file-to-fingerprint")
                .unwrap();

            let input_file_string = subcommand_args.get_one::<String>("input_file").unwrap();

            println!(
                "{}",
                SignatureGenerator::make_signature_from_file(input_file_string)?.encode_to_uri()?
            );
        }
        Some("fingerprint-to-recognized-song") => {
            let subcommand_args = args
                .subcommand_matches("fingerprint-to-recognized-song")
                .unwrap();

            let fingerprint_string = subcommand_args
                .get_one::<String>("fingerprint")
                .unwrap()
                .clone();

            let session = soup::Session::new();
            session.set_timeout(20);

            let main_loop = glib::MainLoop::new(None, false);
            let main_loop_inner = main_loop.clone();
            glib::spawn_future_local(async move {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &recognize_song_from_signature(
                            &session,
                            &DecodedSignature::decode_from_uri(&fingerprint_string).unwrap()
                        )
                        .await
                        .unwrap()
                    )
                    .unwrap()
                );
                main_loop_inner.quit();
            });
            main_loop.run();
        }
        Some("listen") => {
            let subcommand_args = args.subcommand_matches("listen").unwrap();
            let list_devices = subcommand_args.get_flag("list-devices");
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();
            let request_interval = *subcommand_args.get_one::<u64>("request-interval").unwrap();
            let enable_mpris = !subcommand_args.get_flag("disable-mpris");
            let enable_json = subcommand_args.get_flag("json");
            let enable_csv = subcommand_args.get_flag("csv");

            cli_main(CLIParameters {
                enable_mpris,
                list_devices,
                recognize_once: false,
                audio_device,
                request_interval,
                input_file: None,
                output_type: if enable_json {
                    CLIOutputType::JSON
                } else if enable_csv {
                    CLIOutputType::CSV
                } else {
                    CLIOutputType::SongName
                },
            })?;
        }
        Some("recognize") => {
            let subcommand_args = args.subcommand_matches("recognize").unwrap();
            let list_devices = subcommand_args.get_flag("list-devices");
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();
            let request_interval = *subcommand_args.get_one::<u64>("request-interval").unwrap();
            let input_file = subcommand_args.get_one::<String>("input_file").cloned();
            let enable_json = subcommand_args.get_flag("json");
            let enable_csv = subcommand_args.get_flag("csv");

            cli_main(CLIParameters {
                enable_mpris: false,
                list_devices,
                recognize_once: true,
                audio_device,
                request_interval,
                input_file,

                output_type: if enable_json {
                    CLIOutputType::JSON
                } else if enable_csv {
                    CLIOutputType::CSV
                } else {
                    CLIOutputType::SongName
                },
            })?;
        }
        Some("microphone-to-recognized-song") => {
            let subcommand_args = args
                .subcommand_matches("microphone-to-recognized-song")
                .unwrap();
            let list_devices = subcommand_args.get_flag("list-devices");
            let audio_device = subcommand_args.get_one::<String>("audio-device").cloned();
            let request_interval = *subcommand_args.get_one::<u64>("request-interval").unwrap();

            cli_main(CLIParameters {
                enable_mpris: false,
                list_devices,
                recognize_once: true,
                audio_device,
                request_interval,
                input_file: None,
                output_type: CLIOutputType::JSON,
            })?;
        }
        #[cfg(feature = "gui")]
        Some("gui-norecording") => {
            let subcommand_args = args.subcommand_matches("gui-norecording").unwrap();

            gui_main(
                log_object,
                false,
                subcommand_args.get_one::<String>("input_file").cloned(),
                !subcommand_args.get_flag("disable-mpris"),
            )?;
        }
        #[cfg(feature = "gui")]
        Some("gui") | None => {
            if let Some(subcommand_args) = args.subcommand_matches("gui") {
                gui_main(
                    log_object,
                    true,
                    subcommand_args.get_one::<String>("input_file").cloned(),
                    !subcommand_args.get_flag("disable-mpris"),
                )?;
            } else {
                gui_main(log_object, true, None, true)?;
            }
        }
        #[cfg(not(feature = "gui"))]
        None => {
            cli_main(CLIParameters {
                enable_mpris: true,
                list_devices: false,
                recognize_once: false,
                audio_device: None,
                request_interval: 10,
                input_file: None,
                output_type: CLIOutputType::SongName,
            })?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
