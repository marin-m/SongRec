use std::iter::Copied;
use std::num::NonZero;
use std::slice::Iter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::preferences::PreferencesInterface;
use crate::core::thread_messages::{MicrophoneMessage::*, *};

use cpal::platform::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gettextrs::gettext;
use log::{debug, warn};
use rodio::conversions::SampleTypeConverter;
use rodio::nz;

use crate::core::audio_controllers::audio_backend::get_any_backend;

const BUFFER_SIZE_SECS: usize = 12;

struct ProcessingState<'a> {
    input_samples: Vec<f32>,
    processing_tx: &'a async_channel::Sender<ProcessingMessage>,
    gui_tx: async_channel::Sender<GUIMessage>,
    channels: u16,
    sample_rate: u32,
    twelve_seconds_buffer: &'a mut [f32; 16000 * BUFFER_SIZE_SECS],
    number_unprocessed_samples: &'a mut usize,
    number_unmeasured_samples: &'a mut usize,
    processing_already_ongoing: &'a AtomicBool,
    preferences_interface: &'a Arc<Mutex<PreferencesInterface>>,
}

pub fn microphone_thread(
    microphone_rx: async_channel::Receiver<MicrophoneMessage>,
    microphone_tx: async_channel::Sender<MicrophoneMessage>,
    processing_tx: async_channel::Sender<ProcessingMessage>,
    gui_tx: async_channel::Sender<GUIMessage>,
    preferences_interface: Arc<Mutex<PreferencesInterface>>,
    _enable_pipewire_cli: bool,
) {
    #[cfg(all(target_os = "linux", feature = "pipewire"))]
    let preference_order: [bool; 2] = if _enable_pipewire_cli {
        [true, false]
    } else {
        [false, true]
    };

    #[cfg(all(target_os = "linux", not(feature = "pipewire")))]
    let preference_order: [bool; 2] = [false, true];

    #[cfg(not(target_os = "linux"))]
    let preference_order: [bool; 1] = [false];

    let gui_tx_2 = gui_tx.clone();
    let microphone_tx_2 = microphone_tx.clone();

    let err_fn = move |location: &'static str, error: cpal::Error, mut popup: bool| {
        if error.kind() == cpal::ErrorKind::DeviceChanged {
            microphone_tx_2
                .try_send(MicrophoneMessage::RefreshDevices)
                .unwrap();
        } else if error.kind() != cpal::ErrorKind::RealtimeDenied
            && error.kind() != cpal::ErrorKind::Xrun
        {
            let error_string = format!(
                "{} {}: {:?} - {} - {}",
                gettext("Audio error:"),
                location,
                error.kind(),
                error.message().unwrap_or_default(),
                error.kind()
            );

            if error.message().unwrap_or_default() == "no target node available" {
                popup = false;
            }

            if popup {
                gui_tx_2
                    .try_send(GUIMessage::ErrorMessage(error_string))
                    .unwrap();
            } else {
                warn!("{}", error_string);
            }
        }
    };

    'pipewire_switch: for prefer_pipewire in preference_order {
        // Use the default host for working with audio devices.

        debug!("Trying to initialize CPAL...");
        #[cfg(target_os = "linux")]
        let host: cpal::Host = if prefer_pipewire {
            cpal::default_host()
        } else {
            match cpal::host_from_id(cpal::HostId::Alsa) {
                Ok(host) => host,
                Err(err) => {
                    if prefer_pipewire == preference_order[0] {
                        err_fn("ALSA driver not available", err, false);
                    } else {
                        err_fn("ALSA driver not available", err, true);
                    }
                    continue;
                }
            }
        };
        #[cfg(not(target_os = "linux"))]
        let host = cpal::default_host();
        debug!("Using audio playback backend: {:?}", host.id());
        debug!("CPAL initialized");

        let mut backend = get_any_backend();

        // Run the input stream on a separate thread.

        let mut stream: Option<cpal::Stream> = None;

        // Whether our data is already being processed in
        // other threads (pointer to a bool shared between
        // this thread and the CPAL thread, hence the Arc<AtomicBool>)

        let processing_already_ongoing: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        // Send a list of the active microphone-alike devices to the GUI thread
        // (the combo box will be filed with device names when a "DevicesList"
        // inter-thread message will be received at the initialization of the
        // microphone thread, because CPAL which underlies Rodio can't be called
        // from the same thread as the microphone thread under Windows, see:
        //  - https://github.com/RustAudio/rodio/issues/270
        //  - https://github.com/RustAudio/rodio/issues/214 )

        let device_names: Vec<DeviceListItem> = backend.list_devices(&host);

        gui_tx
            .try_send(GUIMessage::DevicesList(device_names))
            .unwrap();

        // Process ingress inter-thread messages (stopping or starting
        // recording from the microphone, and knowing from which device
        // in particular)

        while let Ok(message) = microphone_rx.recv_blocking() {
            match message {
                MicrophoneRecordStart(device_name) => {
                    let processing_tx_2 = processing_tx.clone();
                    let gui_tx_3 = gui_tx.clone();
                    let gui_tx_4 = gui_tx.clone();

                    let err_fn_2 = err_fn.clone();
                    let err_fn_cb = move |error: cpal::Error| {
                        err_fn_2("stream error", error, true);
                    };

                    if host.default_input_device().is_none() {
                        #[cfg(target_os = "linux")]
                        if prefer_pipewire == preference_order[0] {
                            warn!("{}", gettext("Audio error: No input device available"));
                            continue 'pipewire_switch;
                        }
                        gui_tx
                            .try_send(GUIMessage::ErrorMessage(gettext(
                                "Audio error: No input device available",
                            )))
                            .unwrap();
                        return;
                    };

                    let device: Device = backend.set_device(&host, &device_name);

                    let config = match device.default_input_config() {
                        Ok(res) => res,
                        Err(err) => {
                            #[cfg(target_os = "linux")]
                            if prefer_pipewire == preference_order[0] {
                                err_fn("default_input_config", err, false);
                                continue 'pipewire_switch;
                            }
                            err_fn("default_input_config", err, true);
                            return;
                        }
                    };
                    let channels = config.channels();
                    let sample_rate = config.sample_rate();

                    let mut twelve_seconds_buffer = Box::new([0.0f32; 16000 * BUFFER_SIZE_SECS]);
                    let mut number_unprocessed_samples: usize = 0; // Sample count for the interval of doing Shazam recognition (every 4 seconds)
                    let mut number_unmeasured_samples: usize = 0; // Sample count for doing volume measurement (every 24th of second)

                    let processing_already_ongoing_2 = processing_already_ongoing.clone();

                    let preferences_interface = preferences_interface.clone();
                    macro_rules! build_input_streams {
                        ($($sample_format:tt, $generic:ty);+) => {
                            match config.sample_format() {

                                // See https://github.com/RustAudio/rodio/blob/a352fb53846b47523d828b276b6d625f251aabb2/src/microphone.rs#L280
                                // See https://dev.to/sgchris/returning-iterators-from-functions-4cbh

                                cpal::SampleFormat::F32 => match device.build_input_stream(
                                    config.into(),
                                    move |data, _: &_| {
                                        write_data(ProcessingState {
                                            input_samples: data.into_iter().copied().collect(),
                                            processing_tx: &processing_tx_2,
                                            gui_tx: gui_tx_3.clone(),
                                            channels,
                                            sample_rate,
                                            twelve_seconds_buffer: &mut twelve_seconds_buffer,
                                            number_unprocessed_samples: &mut number_unprocessed_samples,
                                            number_unmeasured_samples: &mut number_unmeasured_samples,
                                            processing_already_ongoing: &processing_already_ongoing_2,
                                            preferences_interface: &preferences_interface,
                                        })
                                    },
                                    err_fn_cb,
                                    None,
                                ) {
                                    Ok(res) => {
                                        // Re-call the function in the case the backend is PulseBackend,
                                        // because we may have appeared in the list of PulseAudio's
                                        // source outputs now
                                        let microphone_tx = microphone_tx.clone();
                                        let device_name = device_name.clone();
                                        glib::source::timeout_add_once(std::time::Duration::from_millis(50), move || {
                                            microphone_tx
                                                .try_send(MicrophoneMessage::MicrophoneRecordSetDevice(
                                                    device_name
                                                ))
                                                .unwrap();
                                        });

                                        res
                                    },
                                    Err(err) => {
                                        #[cfg(all(target_os = "linux"))]
                                        if prefer_pipewire == preference_order[0] {
                                            err_fn("build_input_stream", err, false);
                                            continue 'pipewire_switch;
                                        }
                                        err_fn("build_input_stream", err, true);
                                        return;
                                    }
                                },
                                $(
                                    cpal::SampleFormat::$sample_format => match device.build_input_stream(
                                        config.into(),
                                        move |data, _: &_| {
                                            write_data(ProcessingState {
                                                input_samples: SampleTypeConverter::<Copied<Iter<$generic>>, f32>::new(data.into_iter().copied()).collect(),
                                                processing_tx: &processing_tx_2,
                                                gui_tx: gui_tx_3.clone(),
                                                channels,
                                                sample_rate,
                                                twelve_seconds_buffer: &mut twelve_seconds_buffer,
                                                number_unprocessed_samples: &mut number_unprocessed_samples,
                                                number_unmeasured_samples: &mut number_unmeasured_samples,
                                                processing_already_ongoing: &processing_already_ongoing_2,
                                                preferences_interface: &preferences_interface,
                                            })
                                        },
                                        err_fn_cb,
                                        None,
                                    ) {
                                        Ok(res) => {
                                            // Re-call the function in the case the backend is PulseBackend,
                                            // because we may have appeared in the list of PulseAudio's
                                            // source outputs now
                                            let microphone_tx = microphone_tx.clone();
                                            let device_name = device_name.clone();
                                            glib::source::timeout_add_once(std::time::Duration::from_millis(50), move || {
                                                microphone_tx
                                                    .try_send(MicrophoneMessage::MicrophoneRecordSetDevice(
                                                        device_name
                                                    ))
                                                    .unwrap();
                                            });

                                            res
                                        },
                                        Err(err) => {
                                            #[cfg(all(target_os = "linux"))]
                                            if prefer_pipewire == preference_order[0] {
                                                err_fn("build_input_stream", err, false);
                                                continue 'pipewire_switch;
                                            }
                                            err_fn("build_input_stream", err, true);
                                            return;
                                        }
                                    },
                                )+
                                _ => unreachable!(),
                            }
                        };
                    }

                    stream = Some(build_input_streams!(
                        F64, f64;
                        I8, i8;
                        I16, i16;
                        I24, cpal::I24;
                        I32, i32;
                        I64, i64;
                        U8, u8;
                        U16, u16;
                        U24, cpal::U24;
                        U32, u32;
                        U64, u64
                    ));

                    stream.as_ref().unwrap().play().unwrap();

                    gui_tx_4.try_send(GUIMessage::MicrophoneRecording).unwrap();
                }

                MicrophoneRecordSetDevice(device_name) => {
                    backend.set_device(&host, &device_name);
                }

                RefreshDevices => {
                    debug!("Refreshing audio devices...");

                    let device_names: Vec<DeviceListItem> = backend.list_devices(&host);

                    gui_tx
                        .try_send(GUIMessage::DevicesList(device_names))
                        .unwrap();
                }

                MicrophoneRecordStop => {
                    if let Some(some_stream) = stream {
                        drop(some_stream);
                    }

                    stream = None;
                }

                ProcessingDone => {
                    processing_already_ongoing.store(false, Ordering::SeqCst);
                }
            }
        }
        break;
    }
}

fn write_data(state: ProcessingState) {
    // Reassemble data into a 12-second buffer, and do recognition
    // every 4 seconds if the queue to "processing_tx" is empty

    let input_buffer = rodio::buffer::SamplesBuffer::new(
        NonZero::new(state.channels).unwrap(),
        NonZero::new(state.sample_rate).unwrap(),
        state.input_samples,
    );

    let converted_file =
        rodio::source::UniformSourceIterator::new(input_buffer, nz!(1), nz!(16000));

    let raw_pcm_samples: Vec<f32> = converted_file.collect();

    let request_interval_secs;
    {
        let preferences = &state.preferences_interface.lock().unwrap().preferences;
        request_interval_secs = preferences.request_interval_secs_v3.unwrap() as usize;
    }

    let twelve_seconds_buffer = &mut state.twelve_seconds_buffer[..16000 * BUFFER_SIZE_SECS];

    // Update our buffer with data from CPAL

    if raw_pcm_samples.len() >= 16000 * BUFFER_SIZE_SECS {
        twelve_seconds_buffer
            .copy_from_slice(&raw_pcm_samples[raw_pcm_samples.len() - 16000 * BUFFER_SIZE_SECS..]);
    } else {
        let latter_data = twelve_seconds_buffer[raw_pcm_samples.len()..].to_vec();

        twelve_seconds_buffer[..16000 * BUFFER_SIZE_SECS - raw_pcm_samples.len()]
            .copy_from_slice(&latter_data);
        twelve_seconds_buffer[16000 * BUFFER_SIZE_SECS - raw_pcm_samples.len()..]
            .copy_from_slice(&raw_pcm_samples);
    }

    *state.number_unprocessed_samples += raw_pcm_samples.len();

    if *state.number_unprocessed_samples >= 16000 * request_interval_secs
        && !state.processing_already_ongoing.load(Ordering::SeqCst)
    {
        if !twelve_seconds_buffer.iter().all(|x| *x == 0.0) {
            state
                .processing_tx
                .try_send(ProcessingMessage::ProcessAudioSamples(
                    twelve_seconds_buffer.to_vec(),
                ))
                .unwrap();

            state
                .processing_already_ongoing
                .store(true, Ordering::SeqCst);
        }

        *state.number_unprocessed_samples = 0;
    }

    // Do microphone volume measurement every 24th of second (so that we can
    // update it at 24 FPS) and over the last two 100th of second (so that we
    // can be sure to measure volume for at most 100 Hz)

    *state.number_unmeasured_samples += raw_pcm_samples.len();

    if *state.number_unmeasured_samples >= 16000 / 24 {
        let mut max_f32_amplitude = 0.0f32;

        for item in twelve_seconds_buffer
            .iter()
            .take(16000 * BUFFER_SIZE_SECS)
            .skip(16000 * BUFFER_SIZE_SECS - 16000 / 100 * 2)
        {
            if item.abs() > max_f32_amplitude {
                max_f32_amplitude = item.abs();
            }
        }

        state
            .gui_tx
            .try_send(GUIMessage::MicrophoneVolumePercent(
                max_f32_amplitude * 100.0,
            ))
            .unwrap();

        *state.number_unmeasured_samples = 0;
    }
}
