use std::sync::{Arc, Mutex};
use std::iter::Copied;
use std::slice::Iter;
use std::num::NonZero;

use crate::core::thread_messages::{MicrophoneMessage::*, *};
use crate::gui::preferences::PreferencesInterface;

use rodio::conversions::SampleTypeConverter;
use rodio::nz;
use cpal::platform::Device;
use cpal::traits::{DeviceTrait, StreamTrait};
use gettextrs::gettext;

use crate::audio_controllers::audio_backend::get_any_backend;

const MAX_BUFFER_SIZE: usize = 512;

pub fn microphone_thread(
    microphone_rx: async_channel::Receiver<MicrophoneMessage>,
    processing_tx: async_channel::Sender<ProcessingMessage>,
    gui_tx: async_channel::Sender<GUIMessage>,
    preferences_interface: Arc<Mutex<PreferencesInterface>>,
) {
    // Use the default host for working with audio devices.

    #[cfg(target_os = "linux")]
    let host = match cpal::host_from_id(cpal::HostId::PulseAudio) {
        Ok(host) => host,
        _ => cpal::default_host()
    };
    #[cfg(not(target_os = "linux"))]
    let host = cpal::default_host();

    let mut backend = get_any_backend();

    // Run the input stream on a separate thread.

    let mut stream: Option<cpal::Stream> = None;

    let processing_already_ongoing: Arc<Mutex<bool>> = Arc::new(Mutex::new(false)); // Whether our data is already being processed in other threads (pointer to a bool shared between this thread and the CPAL thread, hence the Arc<Mutex>)

    // Send a list of the active microphone-alike devices to the GUI thread
    // (the combo box will be filed with device names when a "DevicesList"
    // inter-thread message will be received at the initialization of the
    // microphone thread, because CPAL which underlies Rodio can't be called
    // from the same thread as the microphone thread under Windows, see:
    //  - https://github.com/RustAudio/rodio/issues/270
    //  - https://github.com/RustAudio/rodio/issues/214 )

    let device_names: Vec<DeviceListItem> = backend.list_devices(&host);

    gui_tx
        .try_send(GUIMessage::DevicesList(Box::new(device_names)))
        .unwrap();

    // Process ingress inter-thread messages (stopping or starting
    // recording from the microphone, and knowing from which device
    // in particular)

    while let Ok(message) = microphone_rx.recv_blocking() {
        match message {
            MicrophoneRecordStart(device_name) => {
                let processing_tx_2 = processing_tx.clone();
                let gui_tx_2 = gui_tx.clone();
                let gui_tx_3 = gui_tx.clone();
                let gui_tx_4 = gui_tx.clone();

                let err_fn = move |error: Box<dyn std::error::Error>| {
                    gui_tx_2
                        .try_send(GUIMessage::ErrorMessage(format!(
                            "{} {}",
                            gettext("Audio error:"),
                            error
                        )))
                        .unwrap();
                };

                let err_fn_2 = err_fn.clone();
                let err_fn_3 = err_fn.clone();
                let err_fn_cb = move |error: cpal::StreamError| {
                    err_fn_2(Box::new(error));
                };

                let device: Device = backend.set_device(&host, &device_name);

                let config = match device.default_input_config() {
                    Ok(res) => res,
                    Err(err) => {
                        err_fn_3(Box::new(err));
                        return;
                    }
                };
                let channels = config.channels();
                let sample_rate = config.sample_rate();

                let mut twelve_seconds_buffer: Vec<f32> = vec![0.0f32; 16000 * MAX_BUFFER_SIZE];
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
                                &config.into(),
                                move |data, _: &_| {
                                    write_data(
                                        data.into_iter().copied().collect(),
                                        &processing_tx_2,
                                        gui_tx_3.clone(),
                                        channels,
                                        sample_rate,
                                        &mut twelve_seconds_buffer,
                                        &mut number_unprocessed_samples,
                                        &mut number_unmeasured_samples,
                                        &processing_already_ongoing_2,
                                        &preferences_interface,
                                    )
                                },
                                err_fn_cb,
                                None,
                            ) {
                                Ok(res) => res,
                                Err(err) => {
                                    err_fn_3(Box::new(err));
                                    return;
                                }
                            },
                            $(
                                cpal::SampleFormat::$sample_format => match device.build_input_stream(
                                    &config.into(),
                                    move |data, _: &_| {
                                        write_data(
                                            SampleTypeConverter::<Copied<Iter<$generic>>, f32>::new(data.into_iter().copied()).collect(),
                                            &processing_tx_2,
                                            gui_tx_3.clone(),
                                            channels,
                                            sample_rate,
                                            &mut twelve_seconds_buffer,
                                            &mut number_unprocessed_samples,
                                            &mut number_unmeasured_samples,
                                            &processing_already_ongoing_2,
                                            &preferences_interface,
                                        )
                                    },
                                    err_fn_cb,
                                    None,
                                ) {
                                    Ok(res) => res,
                                    Err(err) => {
                                        err_fn_3(Box::new(err));
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

                // Re-call the function in the case the backend is PulseBackend,
                // because we may have appeared in the list of PulseAudio's
                // source outputs now
                backend.set_device(&host, &device_name);

                gui_tx_4.try_send(GUIMessage::MicrophoneRecording).unwrap();
            }

            RefreshDevices => {
                let device_names: Vec<DeviceListItem> = backend.list_devices(&host);

                gui_tx
                    .try_send(GUIMessage::DevicesList(Box::new(device_names)))
                    .unwrap();
            }

            MicrophoneRecordStop => {
                if let Some(some_stream) = stream {
                    drop(some_stream);
                }

                stream = None;
            }

            ProcessingDone => {
                let mut processing_already_ongoing_borrow =
                    processing_already_ongoing.lock().unwrap();
                *processing_already_ongoing_borrow = false;
            }
        }
    }
}

fn write_data(
    input_samples: Vec<f32>,
    processing_tx: &async_channel::Sender<ProcessingMessage>,
    gui_tx: async_channel::Sender<GUIMessage>,
    channels: u16,
    sample_rate: u32,
    twelve_seconds_buffer: &mut [f32],
    number_unprocessed_samples: &mut usize,
    number_unmeasured_samples: &mut usize,
    processing_already_ongoing: &Arc<Mutex<bool>>,
    preferences_interface: &Arc<Mutex<PreferencesInterface>>,
) {
    // Reassemble data into a 12-second buffer, and do recognition
    // every 4 seconds if the queue to "processing_tx" is empty

    let input_buffer =
        rodio::buffer::SamplesBuffer::new(NonZero::new(channels).unwrap(), NonZero::new(sample_rate).unwrap(), input_samples);

    let converted_file = rodio::source::UniformSourceIterator::new(input_buffer, nz!(1), nz!(16000));

    let raw_pcm_samples: Vec<f32> = converted_file.collect();

    let preferences = preferences_interface.lock().unwrap().preferences.clone();
    let buffer_size_secs = preferences.buffer_size_secs.unwrap() as usize;
    let request_interval_secs = preferences.request_interval_secs_v3.unwrap() as usize;

    let twelve_seconds_buffer = &mut twelve_seconds_buffer[..16000 * buffer_size_secs];

    // Update our buffer with data from CPAL

    if raw_pcm_samples.len() >= 16000 * buffer_size_secs {
        twelve_seconds_buffer
            .copy_from_slice(&raw_pcm_samples[raw_pcm_samples.len() - 16000 * buffer_size_secs..]);
    } else {
        let latter_data = twelve_seconds_buffer[raw_pcm_samples.len()..].to_vec();

        twelve_seconds_buffer[..16000 * buffer_size_secs - raw_pcm_samples.len()]
            .copy_from_slice(&latter_data);
        twelve_seconds_buffer[16000 * buffer_size_secs - raw_pcm_samples.len()..]
            .copy_from_slice(&raw_pcm_samples);
    }

    *number_unprocessed_samples += raw_pcm_samples.len();

    let mut processing_already_ongoing_borrow = processing_already_ongoing.lock().unwrap();

    if *number_unprocessed_samples >= 16000 * request_interval_secs
        && *processing_already_ongoing_borrow == false
    {
        if !twelve_seconds_buffer.iter().all(|x| *x == 0.0) {
            processing_tx
                .try_send(ProcessingMessage::ProcessAudioSamples(Box::new(
                    twelve_seconds_buffer.to_vec(),
                )))
                .unwrap();

            *processing_already_ongoing_borrow = true;
        }

        *number_unprocessed_samples = 0;
    }

    // Do microphone volume measurement every 24th of second (so that we can
    // update it at 24 FPS) and over the last two 100th of second (so that we
    // can be sure to measure volume for at most 100 Hz)

    *number_unmeasured_samples += raw_pcm_samples.len();

    if *number_unmeasured_samples >= 16000 / 24 {
        let mut max_f32_amplitude = 0.0f32;

        for index in 16000 * buffer_size_secs - 16000 / 100 * 2..16000 * buffer_size_secs {
            if twelve_seconds_buffer[index].abs() > max_f32_amplitude {
                max_f32_amplitude = twelve_seconds_buffer[index].abs();
            }
        }

        gui_tx
            .try_send(GUIMessage::MicrophoneVolumePercent(
                max_f32_amplitude * 100.0,
            ))
            .unwrap();

        *number_unmeasured_samples = 0;
    }
}
