
#![windows_subsystem = "windows"]

mod fingerprinting {
    pub mod communication;
    pub mod algorithm;
    pub mod signature_format;
    mod user_agent;
    mod hanning;
}

mod gui {
    pub mod main_window;
    mod microphone_thread;
    mod processing_thread;
    mod http_thread;
    mod thread_messages;
    mod csv_song_history;
}

mod utils {
    pub mod pulseaudio_loopback;
    pub mod ffmpeg_wrapper;
    pub mod internationalization;
}

use crate::fingerprinting::algorithm::SignatureGenerator;
use crate::fingerprinting::signature_format::DecodedSignature;
use crate::fingerprinting::communication::recognize_song_from_signature;

use crate::utils::internationalization::setup_internationalization;
use crate::gui::main_window::gui_main;

use std::error::Error;
use gettextrs::gettext;
use clap::{App, Arg};

fn main() -> Result<(), Box<dyn Error>> {
    
    // Set up the translation/internationalization part
    
    setup_internationalization();
    
    // Collect the program arguments
    
    let args = App::new("SongRec")
        .about(gettext("An open-source Shazam client for Linux, written in Rust.").as_str())
        .subcommand(
            App::new("gui")
                .about(gettext("The default action. Display a GUI.").as_str())
                .arg(
                    Arg::with_name("input_file")
                        .required(false)
                        .help(gettext("An optional audio file to recognize on the launch of the application.").as_str())
                )
        )
        .subcommand(
            App::new("gui-norecording")
                .about(gettext("Launch the GUI, but don't recognize audio through the microphone as soon as it is launched (rather than expecting the user to click on a button).").as_str())
                .arg(
                    Arg::with_name("input_file")
                        .required(false)
                        .help(gettext("An optional audio file to recognize on the launch of the application.").as_str())
                )
        )
        .subcommand(
            App::new("audio-file-to-recognized-song")
                .about(gettext("Generate a Shazam fingerprint from a sound file, perform song recognition towards Shazam's servers and print obtained information to the standard output.").as_str())
                .arg(
                    Arg::with_name("input_file")
                        .required(true)
                        .help(gettext("The audio file to recognize.").as_str())
                )
        )
        .subcommand(
            App::new("audio-file-to-fingerprint")
                .about(gettext("Generate a Shazam fingerprint from a sound file, and print it to the standard output.").as_str())
                .arg(
                    Arg::with_name("input_file")
                        .required(true)
                        .help(gettext("The .WAV or .MP3 file to generate an audio fingerprint for.").as_str())
                )
        )
        .subcommand(
            App::new("fingerprint-to-recognized-song")
                .about(gettext("Take a data-URI Shazam fingerprint, perform song recognition towards Shazam's servers and print obtained information to the standard output.").as_str())
                .arg(
                    Arg::with_name("fingerprint")
                        .required(true)
                        .help(gettext("The data-URI Shazam fingerprint to recognize.").as_str())
                )
        )
        .subcommand(
            App::new("fingerprint-to-lure")
                .about(gettext("Convert a data-URI Shazam fingerprint into hearable tones, played back instantly (or written to a file, if a path is provided). Not particularly useful, but gives the simplest output that will trick Shazam into recognizing a non-song.").as_str())
                .arg(
                    Arg::with_name("fingerprint")
                        .required(true)
                        .help(gettext("The data-URI Shazam fingerprint to convert into hearable sound.").as_str())
                )
                .arg(
                    Arg::with_name("output_file")
                        .required(false)
                        .help(gettext("File path of the .WAV file to write tones to, or nothing to play back the sound instantly.").as_str())
                )
        )
        .get_matches();
    
    match args.subcommand_name() {
        Some("audio-file-to-recognized-song") => {            
            let subcommand_args = args.subcommand_matches("audio-file-to-recognized-song").unwrap();
            
            let input_file_string = subcommand_args.value_of("input_file").unwrap();
            
            println!("{}", serde_json::to_string_pretty(&recognize_song_from_signature(&SignatureGenerator::make_signature_from_file(input_file_string)?)?)?);
        },
        Some("audio-file-to-fingerprint") => {
            let subcommand_args = args.subcommand_matches("audio-file-to-fingerprint").unwrap();
            
            let input_file_string = subcommand_args.value_of("input_file").unwrap();
            
            println!("{}", SignatureGenerator::make_signature_from_file(input_file_string)?.encode_to_uri()?);
        },
        Some("fingerprint-to-recognized-song") => {
            let subcommand_args = args.subcommand_matches("fingerprint-to-recognized-song").unwrap();
            
            let fingerprint_string = subcommand_args.value_of("fingerprint").unwrap();
            
            println!("{}", serde_json::to_string_pretty(&recognize_song_from_signature(&DecodedSignature::decode_from_uri(fingerprint_string)?)?)?);
        },
        Some("fingerprint-to-lure") => {
            let subcommand_args = args.subcommand_matches("fingerprint-to-lure").unwrap();
            
            let fingerprint_string = subcommand_args.value_of("fingerprint").unwrap();
            
            let samples: Vec<i16> = DecodedSignature::decode_from_uri(fingerprint_string)?.to_lure()?;
            
            match subcommand_args.value_of("output_file") {
                Some(output_file_string) => {
                    let spec = hound::WavSpec {
                        channels: 1,
                        sample_rate: 16000,
                        bits_per_sample: 16,
                        sample_format: hound::SampleFormat::Int,
                    };
                    
                    let mut writer = hound::WavWriter::create(output_file_string, spec)?;
                    
                    for sample in samples {
                        writer.write_sample(sample)?;
                    }
                    
                    writer.finalize()?;
                },
                None => {
                    let mixed_source = rodio::buffer::SamplesBuffer::new::<Vec<i16>>(1, 16000, samples);
                            
                    let (_stream, handle) = rodio::OutputStream::try_default()?;
                    let sink = rodio::Sink::try_new(&handle).unwrap();
                        
                    sink.append(mixed_source);

                    sink.sleep_until_end();

                }
            };
            
        },
        Some("gui-norecording") => {
            let subcommand_args = args.subcommand_matches("gui-norecording").unwrap();

            gui_main(false, subcommand_args.value_of("input_file"))?;
        },
        Some("gui") | None => {
            if let Some(subcommand_args) = args.subcommand_matches("gui") {
                gui_main(true, subcommand_args.value_of("input_file"))?;
            }
            else {
                gui_main(true, None)?;
            }
        },
        _ => unreachable!()
    }
    
    Ok(())
    
}
