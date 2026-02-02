
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
use crate::cli_main::{cli_main, CLIParameters, CLIOutputType};

use std::error::Error;
use gettextrs::gettext;
use clap::{Parser, Subcommand};

// WIP: Updating to the new version of `clap`

#[derive(Parser)]
#[command(version, about, long_about = None, help_template = "\
SongRec {version}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
")] // Read from `Cargo.toml`
struct Cli {
    /// -v: Set the log level to DEBUG instead of WARN for SongRec-related messages
    /// -vv: Set the log level to DEBUG for SongRec-related messages and INFO for GLib-related messages
    /// -vvv: Set the log level to TRACE
    #[arg(short, long, verbatim_doc_comment, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

// (See: https://docs.rs/clap/latest/clap/_derive/index.html#doc-comments etc.)

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    AudioFileToFingerprint {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    AudioFileToRecognizedSong {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    FingerprintToRecognizedSong {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    Gui {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    GuiNorecording {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    Listen {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    MicrophoneToRecognizedSong {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing things
    Recognize {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    Cli::parse();
    // WIP
}