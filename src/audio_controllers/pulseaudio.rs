use cpal::platform::{Host, Device};
use cpal::traits::HostTrait;

use pulsectl::controllers::{SourceController, AppControl, DeviceControl};
use pulsectl::controllers::types::DeviceInfo;

use crate::audio_controllers::audio_backend::AudioBackend;
use crate::core::thread_messages::DeviceListItem;

pub struct PulseBackend {
    handler: SourceController,
    devices: Vec<DeviceInfo>,
    default_device: String
}

impl PulseBackend {
    pub fn try_init() -> Option<Self> {
        if let Ok(mut handler) = SourceController::create() {
            if let Ok(info) = handler.get_server_info() {
                if let Some(default_device) = info.default_source_name {
                    if let Ok(devices) = handler.list_devices() {
                        return Some(Self {
                            handler,
                            devices: devices.clone(),
                            default_device
                        });
                    }
                }
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
            format!("{}", std::process::id())
        ];

        for criterion in criteria {
            for app in applications.clone() {
                if app.proplist.to_string().unwrap().to_lowercase().contains(&criterion) {
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

        for dev in &self.devices {
            if let Some(desc) = &dev.description {
                if let Some(name) = &dev.name {
                    if name == &self.default_device {
                        device_names.insert(0, DeviceListItem {
                            inner_name: name.to_string(),
                            display_name: desc.to_string(),
                            is_monitor: dev.monitor != None
                        });
                    }
                    else if dev.monitor != None {
                        monitor_device_names.push(DeviceListItem {
                            inner_name: name.to_string(),
                            display_name: desc.to_string(),
                            is_monitor: true
                        });
                    }
                    else {
                        device_names.push(DeviceListItem {
                            inner_name: name.to_string(),
                            display_name: desc.to_string(),
                            is_monitor: false
                        });
                    }
                }
            }
        }
        device_names.extend(monitor_device_names);
        device_names
    }

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device {

        if let Some(app_idx) = self.get_app_idx() {

            for dev in self.devices.clone() {
                if Some(inner_name) == dev.name.as_deref() {
                    // println!("find ! {}", dev.index);

                    self.handler.move_app_by_name(app_idx, inner_name).unwrap();
                    break;
                }
            }
        }

        host.default_input_device().unwrap()
    }

}