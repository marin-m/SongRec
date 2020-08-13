
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

use crate::fingerprinting::algorithm::SignatureGenerator;
use crate::fingerprinting::signature_format::DecodedSignature;
use crate::fingerprinting::communication::recognize_song_from_signature;

use crate::gui::main_window::gui_main;

use std::error::Error;
use clap::{App, Arg};

fn main() -> Result<(), Box<dyn Error>> {
    
    let args = App::new("SongRec")
        .about("An open-source Shazam client for Linux, written in Rust.")
        .subcommand(
            App::new("gui")
                .about("The default action. Display a GUI.")
        )
        .subcommand(
            App::new("gui-norecording")
                .about("Launch the GUI, but don't recognize audio through the microphone as soon as it is launched (rather than expecting the user to click on a button).")
        )
        .subcommand(
            App::new("audio-file-to-recognized-song")
                .about("Generate a Shazam fingerprint from a sound file, perform song recognition towards Shazam's servers and print obtained information to the standard output.")
                .arg(
                    Arg::with_name("input_file")
                        .required(true)
                        .help("The .WAV or .MP3 file to recognize.")
                )
        )
        .subcommand(
            App::new("audio-file-to-fingerprint")
                .about("Generate a Shazam fingerprint from a sound file, and print it to the standard output.")
                .arg(
                    Arg::with_name("input_file")
                        .required(true)
                        .help("The .WAV or .MP3 file to generate an audio fingerprint for.")
                )
        )
        .subcommand(
            App::new("fingerprint-to-recognized-song")
                .about("Take a data-URI Shazam fingerprint, perform song recognition towards Shazam's servers and print obtained information to the standard output.")
                .arg(
                    Arg::with_name("fingerprint")
                        .required(true)
                        .help("The data-URI Shazam fingerprint to recognize.")
                )
        )
        .subcommand(
            App::new("fingerprint-to-lure")
                .about("Convert a data-URI Shazam fingerprint into readable hearable tones, played back instantly (or written to a file, if a path is provided). Not particularly useful, but gives the simplest output that will trick Shazam into  recognizing a non-song.")
                .arg(
                    Arg::with_name("fingerprint")
                        .required(true)
                        .help("The data-URI Shazam fingerprint to convert into hearable sound.")
                )
                .arg(
                    Arg::with_name("output_file")
                        .required(false)
                        .help("File path of the .WAV file to write tones to, or nothing to play black the sound instantly.")
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
            gui_main(false)?;
        },
        Some("gui") | None => {
            gui_main(true)?;
        },
        _ => unreachable!()
    }
    
    Ok(())
    
}
