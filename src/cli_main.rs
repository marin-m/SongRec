use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::{mpsc, Arc};

use chrono::Local;
use gettextrs::gettext;
use glib;
use glib::clone;

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

pub fn cli_main(parameters: CLIParameters) -> Result<(), Box<dyn Error>> {
    glib::MainContext::default().acquire();
    let main_loop = Arc::new(glib::MainLoop::new(None, false));

    let (gui_tx, gui_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (microphone_tx, microphone_rx) = mpsc::channel();
    let (processing_tx, processing_rx) = mpsc::channel();
    let (http_tx, http_rx) = mpsc::channel();

    let processing_microphone_tx = processing_tx.clone();
    let microphone_http_tx = microphone_tx.clone();

    spawn_big_thread(
        clone!(@strong gui_tx => move || { // microphone_rx, processing_tx
            microphone_thread(microphone_rx, processing_microphone_tx, gui_tx);
        }),
    );

    spawn_big_thread(clone!(@strong gui_tx => move || { // processing_rx, http_tx
        processing_thread(processing_rx, http_tx, gui_tx);
    }));

    spawn_big_thread(clone!(@strong gui_tx => move || { // http_rx
        http_thread(http_rx, gui_tx, microphone_http_tx);
    }));

    // recognize once if an input file is provided
    let do_recognize_once = parameters.recognize_once || parameters.input_file.is_some();

    // do not enable mpris if recognizing one song
    
    #[cfg(feature = "mpris")]
    let mpris_obj = {
        let do_enable_mpris = parameters.enable_mpris && !do_recognize_once;
        if do_enable_mpris { get_player() } else { None }
    };
    let last_track: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let main_loop_cli = main_loop.clone();

    let audio_dev_name = parameters.audio_device.as_ref().map(|dev| dev.to_string());
    let input_file_name = parameters.input_file.as_ref().map(|dev| dev.to_string());

    if let Some(ref filename) = parameters.input_file {
        processing_tx
            .send(ProcessingMessage::ProcessAudioFile(filename.to_string()))
            .unwrap();
    }

    let mut csv_writer = csv::Writer::from_writer(std::io::stdout());

    gui_rx.attach(None, move |gui_message| {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                // no need to start a microphone if recognizing from file
                if input_file_name.is_some() {
                    return glib::Continue(true);
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
                        main_loop_cli.quit();
                        return glib::Continue(false);
                    }
                dev
                } else {
                    if device_names.is_empty() {
                        eprintln!("{}", gettext("Exiting: no audio devices found!"));
                        main_loop_cli.quit();
                        return glib::Continue(false);
                    }
                    &device_names[0].inner_name
                };
                eprintln!("{} {}", gettext("Using device"), dev_name);
                microphone_tx
                    .send(MicrophoneMessage::MicrophoneRecordStart(
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
                        main_loop_cli.quit();
                        return glib::Continue(false);
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
                    main_loop_cli.quit();
                    return glib::Continue(false);
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
                    mpris_obj.as_ref().map(|p| update_song(p, &message));
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
                    main_loop_cli.quit();
                    return glib::Continue(false);
                }
            }
            _ => {}
        }
        glib::Continue(true)
    });

    main_loop.run();
    Ok(())
}
