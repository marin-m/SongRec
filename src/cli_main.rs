use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use chrono::Local;
use gettextrs::gettext;

#[cfg(feature = "mpris")]
use mpris_player::PlaybackStatus;

use crate::core::http_thread::http_thread;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{GUIMessage, MicrophoneMessage, ProcessingMessage, spawn_big_thread};

use crate::utils::csv_song_history::SongHistoryRecord;
#[cfg(feature = "mpris")]
use crate::utils::mpris_player::{get_player, update_song};

pub enum CLIOutputType {
    SongName,
    JSON,
    CSV,
}

pub struct CLIParameters {
    pub enable_mpris: bool,
    pub recognize_once: bool,
    pub audio_device: Option<String>,
    pub input_file: Option<String>,
    pub output_type: CLIOutputType,
}

pub fn cli_main(parameters: CLIParameters) -> Result<(), Box<dyn Error>>
{
    let (gui_tx, gui_rx) = async_channel::unbounded(); // WIP: replace with async_channel::unbounded + Receiver.recv_blocking (https://docs.rs/async-channel/latest/async_channel/struct.Receiver.html)
    let (microphone_tx, microphone_rx) = async_channel::unbounded();
    let (processing_tx, processing_rx) = async_channel::unbounded();
    let (http_tx, http_rx) = async_channel::unbounded();

    let gui_tx_2 = gui_tx.clone();
    let gui_tx_3 = gui_tx.clone();
    let processing_tx_2 = processing_tx.clone();
    let microphone_tx_2 = microphone_tx.clone();

    spawn_big_thread(move || {
        microphone_thread(microphone_rx, processing_tx_2, gui_tx_2);
    });

    spawn_big_thread(move || {
        processing_thread(processing_rx, http_tx, gui_tx_3);
    });

    spawn_big_thread(move || {
        http_thread(http_rx, gui_tx, microphone_tx_2);
    });

    // recognize once if an input file is provided
    let do_recognize_once = parameters.recognize_once || parameters.input_file.is_some();

    // do not enable mpris if recognizing one song
    
    #[cfg(feature = "mpris")]
    let mpris_obj = {
        let do_enable_mpris = parameters.enable_mpris && !do_recognize_once;
        if do_enable_mpris { get_player() } else { None }
    };
    #[cfg(feature = "mpris")]
    let mut last_cover_path = None;

    let last_track: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let audio_dev_name = parameters.audio_device.as_ref().map(|dev| dev.to_string());
    let input_file_name = parameters.input_file.as_ref().map(|dev| dev.to_string());

    if let Some(ref filename) = parameters.input_file {
        processing_tx
            .send_blocking(ProcessingMessage::ProcessAudioFile(filename.to_string()))
            .unwrap();
    }

    let mut csv_writer = csv::Writer::from_writer(std::io::stdout());

    while let Ok(gui_message) = gui_rx.recv_blocking() {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                // no need to start a microphone if recognizing from file
                if input_file_name.is_some() {
                    continue;
                }
                let dev_name = if let Some(dev) = &audio_dev_name {
                    let mut found: bool = false;
                    for device in device_names.iter() {
                        if &device.inner_name == dev || &device.display_name == dev {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        eprintln!("{}", gettext("Exiting: audio device not found"));
                        break;
                    }
                dev
                } else {
                    if device_names.is_empty() {
                        eprintln!("{}", gettext("Exiting: no audio devices found!"));
                        break;
                    }
                    &device_names[0].inner_name
                };
                eprintln!("{} {}", gettext("Using device"), dev_name);
                microphone_tx
                    .send_blocking(MicrophoneMessage::MicrophoneRecordStart(
                        dev_name.to_owned(),
                    ))
                    .unwrap();
            }
            GUIMessage::NetworkStatus(reachable) => {
                #[cfg(feature = "mpris")] {
                    let mpris_status = if reachable {
                        PlaybackStatus::Playing
                    } else {
                        PlaybackStatus::Paused
                    };
                    mpris_obj
                        .as_ref()
                        .map(|p| p.set_playback_status(mpris_status));
                }

                if !reachable {
                    if input_file_name.is_some() {
                        eprintln!("{}", gettext("Error: Network unreachable"));
                        break;
                    } else {
                        eprintln!("{}", gettext("Warning: Network unreachable"));
                    }
                }
            }
            GUIMessage::ErrorMessage(string) => {
                if !(string == gettext("No match for this song") && !input_file_name.is_some()) {
                    eprintln!("{} {}", gettext("Error:"), string);
                }
                if input_file_name.is_some() {
                    break;
                }
            }
            GUIMessage::MicrophoneRecording => {
                if !do_recognize_once {
                    eprintln!("{}", gettext("Recording started!"));
                }
            }
            GUIMessage::SongRecognized(message) => {
                let mut last_track_borrow = last_track.borrow_mut();
                let track_key = Some(message.track_key.clone());
                let song_name = format!("{} - {}", message.artist_name, message.song_name);

                if *last_track_borrow != track_key {
                    #[cfg(feature = "mpris")]
                    mpris_obj.as_ref().map(|p| update_song(p, &message, &mut last_cover_path));
                    *last_track_borrow = track_key;
                    match parameters.output_type {
                        CLIOutputType::JSON => {
                            println!("{}", message.shazam_json);
                        }
                        CLIOutputType::CSV => {
                            csv_writer
                                .serialize(SongHistoryRecord {
                                    song_name: song_name,
                                    album: Some(
                                        message
                                            .album_name
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    track_key: Some(message.track_key),
                                    release_year: Some(
                                        message
                                            .release_year
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    genre: Some(
                                        message
                                            .genre
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    recognition_date: Local::now().format("%c").to_string(),
                                })
                                .unwrap();
                            csv_writer.flush().unwrap();
                        }
                        CLIOutputType::SongName => {
                            println!("{}", song_name);
                        }
                    };
                }
                if do_recognize_once {
                    break;
                }
            }
            _ => {}
        }
    }

    gui_rx.close();

    Ok(())
}
