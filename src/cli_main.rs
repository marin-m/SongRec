use log::{error, info, warn};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use chrono::Local;
use gettextrs::gettext;

#[cfg(feature = "mpris")]
use mpris_player::PlaybackStatus;

use crate::core::http_task::http_task;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{
    spawn_big_thread, GUIMessage, MicrophoneMessage, ProcessingMessage,
};

use crate::core::preferences::{Preferences, PreferencesInterface};
use crate::utils::csv_song_history::SongHistoryRecord;
// TODO re-implement this
#[cfg(feature = "mpris")]
use crate::plugins::mpris_player::{get_player, update_song};

pub enum CLIOutputType {
    SongName,
    JSON,
    CSV,
}

pub struct CLIParameters {
    pub enable_mpris: bool,
    pub list_devices: bool,
    pub recognize_once: bool,
    pub audio_device: Option<String>,
    pub request_interval: u64,
    pub input_file: Option<String>,
    pub output_type: CLIOutputType,
}

pub fn cli_main(parameters: CLIParameters) -> Result<(), Box<dyn Error>> {
    let (gui_tx, gui_rx) = async_channel::unbounded();
    let (microphone_tx, microphone_rx) = async_channel::unbounded();
    let (processing_tx, processing_rx) = async_channel::unbounded();
    let (http_tx, http_rx) = async_channel::unbounded();

    let gui_tx_2 = gui_tx.clone();
    let gui_tx_3 = gui_tx.clone();
    let processing_tx_2 = processing_tx.clone();
    let microphone_tx_2 = microphone_tx.clone();

    let preferences_interface = Arc::new(Mutex::new(PreferencesInterface {
        preferences_file_path: None,
        preferences: Preferences::with_interval(parameters.request_interval),
    }));

    spawn_big_thread(move || {
        microphone_thread(
            microphone_rx,
            processing_tx_2,
            gui_tx_2,
            preferences_interface,
        );
    });

    spawn_big_thread(move || {
        processing_thread(processing_rx, http_tx, gui_tx_3);
    });

    glib::spawn_future_local(http_task(http_rx, gui_tx, microphone_tx_2));

    // recognize once if an input file is provided
    let do_recognize_once = parameters.recognize_once || parameters.input_file.is_some();

    // do not enable mpris if recognizing one song

    // TODO re-implement this with new lib
    #[cfg(feature = "mpris")]
    let mpris_obj = {
        let do_enable_mpris = parameters.enable_mpris && !do_recognize_once;
        if do_enable_mpris {
            get_player()
        } else {
            None
        }
    };
    #[cfg(feature = "mpris")]
    let mut last_cover_path = None;

    let last_track: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let audio_dev_name = parameters.audio_device.as_ref().map(|dev| dev.to_string());
    let input_file_name = parameters.input_file.as_ref().map(|dev| dev.to_string());

    if let Some(ref filename) = parameters.input_file {
        processing_tx
            .try_send(ProcessingMessage::ProcessAudioFile(filename.to_string()))
            .unwrap();
    }

    let main_loop = glib::MainLoop::new(None, false);
    let loop_inner = main_loop.clone();

    glib::spawn_future_local(async move {
        let mut csv_writer = csv::Writer::from_writer(std::io::stdout());

        while let Ok(gui_message) = gui_rx.recv().await {
            match gui_message {
                GUIMessage::DevicesList(device_names) => {
                    // no need to start a microphone if recognizing from file
                    if input_file_name.is_some() {
                        continue;
                    }
                    for device in device_names.iter() {
                        info!(
                            "{} {} ({})",
                            gettext("Available device:"),
                            device.inner_name,
                            device.display_name
                        );
                    }
                    if parameters.list_devices {
                        loop_inner.quit();
                        break;
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
                            error!("{}", gettext("Exiting: audio device not found"));
                            break;
                        }
                        dev
                    } else {
                        if device_names.is_empty() {
                            error!("{}", gettext("Exiting: no audio devices found!"));
                            break;
                        }
                        &device_names[0].inner_name
                    };
                    info!("{} {}", gettext("Using device"), dev_name);
                    microphone_tx
                        .try_send(MicrophoneMessage::MicrophoneRecordStart(
                            dev_name.to_owned(),
                        ))
                        .unwrap();
                }
                GUIMessage::NetworkStatus(reachable) => {
                    #[cfg(feature = "mpris")]
                    {
                        // TODO re-implement this
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
                            error!("{}", gettext("Error: Network unreachable"));
                            break;
                        } else {
                            warn!("{}", gettext("Warning: Network unreachable"));
                        }
                    }
                }
                GUIMessage::ErrorMessage(string) => {
                    if !(string == gettext("No match for this song") && !input_file_name.is_some())
                    {
                        error!("{} {}", gettext("Error:"), string);
                    }
                    if input_file_name.is_some() {
                        break;
                    }
                }
                GUIMessage::MicrophoneRecording => {
                    if !do_recognize_once {
                        info!("{}", gettext("Recording started!"));
                    }
                }
                GUIMessage::SongRecognized(message) => {
                    let mut last_track_borrow = last_track.borrow_mut();
                    let track_key = Some(message.track_key.clone());
                    let song_name = format!("{} - {}", message.artist_name, message.song_name);

                    if *last_track_borrow != track_key {
                        // TODO re-implement this with new lib
                        #[cfg(feature = "mpris")]
                        mpris_obj
                            .as_ref()
                            .map(|p| update_song(p, &message, &mut last_cover_path));

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
        loop_inner.quit();
    });

    main_loop.run();

    Ok(())
}
