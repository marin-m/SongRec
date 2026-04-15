use cpal::device_description::{DeviceDirection, DeviceType};
use cpal::platform::{Device, Host};
use cpal::traits::{DeviceTrait, HostTrait};

use crate::core::audio_controllers::audio_backend::AudioBackend;
use crate::core::thread_messages::DeviceListItem;

pub struct CpalBackend;

impl AudioBackend for CpalBackend {
    fn list_devices(&mut self, host: &Host) -> Vec<DeviceListItem> {
        let mut device_names: Vec<DeviceListItem> = vec![];

        for device in host.input_devices().unwrap() {
            let device_id = device.id().unwrap().to_string();
            let device_description = device.description().unwrap();

            let mut device_name = device_description.name().to_string();

            if &device_name == "unknown" {
                device_name = device_id.clone();
            }

            match device_description.direction() {
                DeviceDirection::Input => {
                    device_name += " (input)";
                }
                DeviceDirection::Output => {
                    device_name += " (output)";
                }
                _ => {}
            }

            if device_description.device_type() != DeviceType::Unknown {
                device_name = format!("{} {}", device_description.device_type(), device_name);
            }

            // Selecting the "upmix" or "vdownmix" input
            // source on an ALSA-based configuration may
            // crash our underlying sound library.

            #[cfg(target_os = "linux")]
            if device_id.contains("upmix") || device_id.contains("downmix") {
                continue;
            }

            device_names.push(DeviceListItem {
                inner_name: device_id.clone(),
                display_name: device_name.clone(),
                is_monitor: device_description.direction() != DeviceDirection::Input,
            });
        }

        device_names
    }

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device {
        let mut device: cpal::Device = host.default_input_device().unwrap();

        for possible_device in host.input_devices().unwrap() {
            if possible_device.id().unwrap().to_string() == inner_name {
                device = possible_device;
                break;
            }
        }

        device
    }
}
