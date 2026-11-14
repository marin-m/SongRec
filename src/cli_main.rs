use log::{error, info, warn};
use std::error::Error;
use std::io::Write;
use std::sync::{Arc, Mutex};

use chrono::Local;
use gettextrs::gettext;

#[cfg(all(target_os = "linux", feature = "mpris"))]
use mpris_server::PlaybackStatus;

use crate::core::http_task::http_task;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{
    DeviceListItem, GUIMessage, MicrophoneMessage, ProcessingMessage, SongRecognizedMessage,
    spawn_big_thread,
};

use crate::core::preferences::{Preferences, PreferencesInterface};
#[cfg(all(target_os = "linux", feature = "mpris"))]
use crate::plugins::mpris_player::{get_player, update_song};
use crate::utils::csv_song_history::SongHistoryRecord;

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
    pub input_file: Option<std::path::PathBuf>,
    pub output_type: CLIOutputType,
}

/// Run the command line interface, and return whether it stopped without an
/// error
pub fn cli_main(parameters: CLIParameters) -> Result<bool, Box<dyn Error>> {
    let (gui_tx, gui_rx) = async_channel::unbounded();
    let (microphone_tx, microphone_rx) = async_channel::unbounded();
    let (processing_tx, processing_rx) = async_channel::unbounded();
    let (http_tx, http_rx) = async_channel::unbounded();

    let gui_tx_2 = gui_tx.clone();
    let gui_tx_3 = gui_tx.clone();
    let processing_tx_2 = processing_tx.clone();
    let microphone_tx_2 = microphone_tx.clone();
    let microphone_tx_3 = microphone_tx.clone();

    let preferences_interface = Arc::new(Mutex::new(PreferencesInterface {
        preferences_file_path: None,
        preferences: Preferences::with_interval(parameters.request_interval),
    }));

    spawn_big_thread(move || {
        microphone_thread(
            microphone_rx,
            microphone_tx_2,
            processing_tx_2,
            gui_tx_2,
            preferences_interface,
            true,
        );
    });

    spawn_big_thread(move || {
        processing_thread(processing_rx, http_tx, gui_tx_3);
    });

    glib::spawn_future_local(http_task(http_rx, gui_tx, microphone_tx_3));

    let main_loop = glib::MainLoop::new(None, false);
    let loop_inner = main_loop.clone();
    // Set when an error stops the loop, so that the process exits with a
    // failure status that scripts can check.
    let failed = std::rc::Rc::new(std::cell::Cell::new(false));
    let failed_inner = failed.clone();

    glib::spawn_future_local(async move {
        // Recognize once if an input file is provided

        let do_recognize_once = parameters.recognize_once || parameters.input_file.is_some();

        // Do not enable mpris if recognizing one song

        #[cfg(all(target_os = "linux", feature = "mpris"))]
        let mpris_obj = {
            let do_enable_mpris = parameters.enable_mpris && !do_recognize_once;
            if do_enable_mpris {
                get_player(false).await
            } else {
                None
            }
        };
        #[cfg(all(target_os = "linux", feature = "mpris"))]
        let mut last_cover_path = None;

        let mut last_track: Option<String> = None;

        let audio_dev_name = parameters.audio_device.as_ref().map(|dev| dev.to_string());
        let input_file_name = parameters.input_file.clone();

        // `--list-devices` lists the devices and quits, even with a file
        if let Some(ref filename) = parameters.input_file
            && !parameters.list_devices
        {
            processing_tx
                .try_send(ProcessingMessage::ProcessAudioFile(filename.clone()))
                .unwrap();
        }

        let mut csv_writer = csv::Writer::from_writer(std::io::stdout());

        while let Ok(gui_message) = gui_rx.recv().await {
            match gui_message {
                GUIMessage::DevicesList(device_names) => {
                    // no need to start a microphone if recognizing from file
                    if input_file_name.is_some() && !parameters.list_devices {
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
                        let Some(inner_name) = find_device(&device_names, dev) else {
                            error!("{}", gettext("Exiting: audio device not found"));
                            failed_inner.set(true);
                            break;
                        };
                        inner_name
                    } else {
                        if device_names.is_empty() {
                            error!("{}", gettext("Exiting: no audio devices found!"));
                            failed_inner.set(true);
                            break;
                        }
                        &device_names[0].inner_name
                    };
                    info!("{}", gettext("Using device %s").replace("%s", dev_name));
                    microphone_tx
                        .try_send(MicrophoneMessage::MicrophoneRecordStart(
                            dev_name.to_owned(),
                        ))
                        .unwrap();
                }
                GUIMessage::NetworkStatus(reachable) => {
                    #[cfg(all(target_os = "linux", feature = "mpris"))]
                    {
                        let mpris_status = if reachable {
                            PlaybackStatus::Playing
                        } else {
                            PlaybackStatus::Paused
                        };
                        if let Some(ref player) = mpris_obj
                            && let Err(error) = player.set_playback_status(mpris_status).await
                        {
                            error!("Could not set MPRIS playback status: {:?}", error);
                        }
                    }

                    if !reachable {
                        if input_file_name.is_some() {
                            error!("{}", gettext("Error: Network unreachable"));
                            failed_inner.set(true);
                            break;
                        } else {
                            warn!("{}", gettext("Warning: Network unreachable"));
                        }
                    }
                }
                // The HTTP task sends only this message when Shazam answers
                // 429, so a file recognition would otherwise wait forever.
                GUIMessage::RateLimitState(true) => {
                    if input_file_name.is_some() {
                        error!(
                            "{} {}",
                            gettext("Error:"),
                            gettext("Your IP has been rate-limited")
                        );
                        failed_inner.set(true);
                        break;
                    } else {
                        warn!("{}", gettext("Your IP has been rate-limited"));
                    }
                }
                GUIMessage::ErrorMessage(string) => {
                    if string != gettext("No match for this song") || input_file_name.is_some() {
                        error!("{} {}", gettext("Error:"), string);
                    }
                    if input_file_name.is_some() {
                        failed_inner.set(true);
                        break;
                    }
                }
                GUIMessage::MicrophoneRecording if !do_recognize_once => {
                    info!("{}", gettext("Recording started!"));
                }
                GUIMessage::SongRecognized(message) => {
                    let track_key = Some(message.track_key.clone());

                    if last_track != track_key {
                        // TODO re-implement this with new lib
                        #[cfg(all(target_os = "linux", feature = "mpris"))]
                        if let Some(ref player) = mpris_obj {
                            update_song(player, &message, &mut last_cover_path).await;
                        }

                        last_track = track_key;
                        if let Err(error) = write_recognized_song(
                            &mut std::io::stdout(),
                            &mut csv_writer,
                            &parameters.output_type,
                            *message,
                            Local::now().format("%c").to_string(),
                        ) {
                            // The reader closed the pipe early (`songrec ... | head`):
                            // stop quietly instead of panicking, like other CLI tools.
                            if error.kind() != std::io::ErrorKind::BrokenPipe {
                                error!("{} {}", gettext("Error:"), error);
                                failed_inner.set(true);
                            }
                            break;
                        }
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

    Ok(!failed.get())
}

/// The inner name of the device that `--audio-device REQUESTED` names, by its inner or its
/// displayed name. The microphone thread looks devices up by their inner name only, and falls
/// back to the default device otherwise.
fn find_device<'a>(devices: &'a [DeviceListItem], requested: &str) -> Option<&'a str> {
    devices
        .iter()
        .find(|device| device.inner_name == requested || device.display_name == requested)
        .map(|device| device.inner_name.as_str())
}

/// Write one recognized song to the standard output, in the requested format.
fn write_recognized_song(
    stdout: &mut impl Write,
    csv_writer: &mut csv::Writer<impl Write>,
    output_type: &CLIOutputType,
    message: SongRecognizedMessage,
    recognition_date: String,
) -> std::io::Result<()> {
    let song_name = format!("{} - {}", message.artist_name, message.song_name);
    match output_type {
        CLIOutputType::JSON => writeln!(stdout, "{}", message.shazam_json),
        CLIOutputType::SongName => writeln!(stdout, "{}", song_name),
        CLIOutputType::CSV => {
            csv_writer.serialize(SongHistoryRecord {
                song_name,
                album: Some(message.album_name.unwrap_or_default()),
                track_key: Some(message.track_key),
                release_year: Some(message.release_year.unwrap_or_default()),
                genre: Some(message.genre.unwrap_or_default()),
                recognition_date,
            })?;
            csv_writer.flush()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> SongRecognizedMessage {
        SongRecognizedMessage {
            artist_name: "Artist".to_string(),
            album_name: Some("Album".to_string()),
            song_name: "Title".to_string(),
            cover_image: None,
            track_key: "123".to_string(),
            release_year: Some("2020".to_string()),
            genre: Some("Pop".to_string()),
            shazam_json: r#"{"track":{}}"#.to_string(),
        }
    }

    fn output(output_type: CLIOutputType) -> String {
        let mut stdout = Vec::new();
        let mut csv_writer = csv::Writer::from_writer(Vec::new());
        write_recognized_song(
            &mut stdout,
            &mut csv_writer,
            &output_type,
            message(),
            "date".to_string(),
        )
        .unwrap();
        stdout.extend(csv_writer.into_inner().unwrap());
        String::from_utf8(stdout).unwrap()
    }

    #[test]
    fn audio_devices_are_found_by_either_name_and_passed_by_inner_name() {
        let device = |inner_name: &str, display_name: &str| DeviceListItem {
            inner_name: inner_name.to_string(),
            display_name: display_name.to_string(),
            is_monitor: false,
        };
        let devices = [
            device("coreaudio:BuiltIn", "MacBook Air Microphone (input)"),
            device("alsa:hw:1", "USB Audio (input)"),
        ];

        // The microphone thread looks devices up by their inner name only.
        assert_eq!(find_device(&devices, "alsa:hw:1"), Some("alsa:hw:1"));
        assert_eq!(
            find_device(&devices, "USB Audio (input)"),
            Some("alsa:hw:1")
        );
        assert_eq!(find_device(&devices, "Missing"), None);
    }

    #[test]
    fn writes_each_output_format() {
        assert_eq!(output(CLIOutputType::SongName), "Artist - Title\n");
        assert_eq!(output(CLIOutputType::JSON), "{\"track\":{}}\n");
        assert_eq!(
            output(CLIOutputType::CSV),
            "song_name,album,track_key,release_year,genre,recognition_date\n\
             Artist - Title,Album,123,2020,Pop,date\n"
        );
    }

    /// Stands for a standard output whose reader has exited, as in `songrec ... | head`.
    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn closed_pipe_is_reported_instead_of_panicking() {
        for output_type in [
            CLIOutputType::SongName,
            CLIOutputType::JSON,
            CLIOutputType::CSV,
        ] {
            let mut csv_writer = csv::Writer::from_writer(ClosedPipe);
            let error = write_recognized_song(
                &mut ClosedPipe,
                &mut csv_writer,
                &output_type,
                message(),
                "date".to_string(),
            )
            .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
    }
}
