use cpal::platform::{Device, Host};
use cpal::traits::HostTrait;

use pulsectl::controllers::{AppControl, DeviceControl, SourceController};

use crate::audio_controllers::audio_backend::AudioBackend;
use crate::core::thread_messages::DeviceListItem;

use log::{debug, error};

pub struct PulseBackend {
    handler: SourceController,
}

impl PulseBackend {
    pub fn try_init() -> Option<Self> {
        match SourceController::create() {
            Ok(mut handler) => {
                if let Err(error) = handler.get_server_info() {
                    error!("Could not get PulseAudio server info: {:?}", error);
                } else if let Err(error) = handler.list_devices() {
                    error!("Could not list PulseAudio devices: {:?}", error);
                } else {
                    return Some(Self { handler });
                }
            }
            Err(error) => {
                error!("Could not initialize PulseAudio backend: {:?}", error);
            }
        }
        None
    }

    fn get_app_idx(&mut self) -> Option<u32> {
        // Get SongRec's source-output index

        let applications = self.handler.list_applications().unwrap();

        let criteria: Vec<String> = vec![
            format!("process.id = \"{}\"", std::process::id()),
            "alsa plug-in [songrec]".to_string(),
            "songrec".to_string(),
            format!("{}", std::process::id()),
        ];

        for criterion in criteria {
            for app in applications.clone() {
                if app
                    .proplist
                    .to_string()
                    .unwrap()
                    .to_lowercase()
                    .contains(&criterion)
                {
                    return Some(app.index);
                }
            }
        }
        None
    }
}

impl AudioBackend for PulseBackend {
    fn list_devices(&mut self, _host: &Host) -> Vec<DeviceListItem> {
        let mut device_names: Vec<DeviceListItem> = vec![];
        let mut monitor_device_names: Vec<DeviceListItem> = vec![];

        match self.handler.get_server_info() {
            Ok(info) => match self.handler.list_devices() {
                Ok(devices) => {
                    for dev in devices {
                        if let Some(desc) = &dev.description {
                            if let Some(name) = &dev.name {
                                if &dev.name == &info.default_source_name {
                                    device_names.insert(
                                        0,
                                        DeviceListItem {
                                            inner_name: name.to_string(),
                                            display_name: desc.to_string(),
                                            is_monitor: dev.monitor != None,
                                        },
                                    );
                                } else if dev.monitor != None {
                                    monitor_device_names.push(DeviceListItem {
                                        inner_name: name.to_string(),
                                        display_name: desc.to_string(),
                                        is_monitor: true,
                                    });
                                } else {
                                    device_names.push(DeviceListItem {
                                        inner_name: name.to_string(),
                                        display_name: desc.to_string(),
                                        is_monitor: false,
                                    });
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    error!("Could not list PulseAudio devices: {:?}", error);
                }
            },
            Err(error) => {
                error!("Could not get PulseAudio server info: {:?}", error);
            }
        }

        device_names.extend(monitor_device_names);
        device_names
    }

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device {
        match self.handler.list_devices() {
            Ok(devices) => {
                if let Some(app_idx) = self.get_app_idx() {
                    for dev in devices {
                        debug!(
                            "Comparing libpulse device names: {:?} / {:?}",
                            dev.name, inner_name
                        );
                        if Some(inner_name) == dev.name.as_deref() {
                            debug!("Selected libpulse device found: {:?}", dev);

                            self.handler.move_app_by_name(app_idx, inner_name).unwrap();
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                error!("Could not list PulseAudio devices: {:?}", error);
            }
        }

        host.default_input_device().unwrap()
    }
}
