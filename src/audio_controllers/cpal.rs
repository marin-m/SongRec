use cpal::platform::{Host, Device};
use cpal::traits::{HostTrait, DeviceTrait};

use crate::audio_controllers::audio_backend::AudioBackend;
use crate::core::thread_messages::DeviceListItem;

pub struct CpalBackend;

impl AudioBackend for CpalBackend {
    fn list_devices(&mut self, host: &Host) -> Vec<DeviceListItem> {
        let mut device_names: Vec<DeviceListItem> = vec![];

        for device in host.input_devices().unwrap() {
            let device_name = device.name().unwrap();
            
            // Selecting the "upmix" or "vdownmix" input
            // source on an ALSA-based configuration may
            // crash our underlying sound library.
            
            if device_name.contains("upmix") || device_name.contains("downmix") {
                continue;
            }
    
            device_names.push(DeviceListItem {
                inner_name: device_name.clone(),
                display_name: device_name.clone(),
                is_monitor: false
            });
        }

        device_names
    }

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device {
        let mut device: cpal::Device = host.default_input_device().unwrap();
                
        for possible_device in host.input_devices().unwrap() {
            
            if possible_device.name().unwrap() == inner_name {
                
                device = possible_device;
                break;
                
            }
        }

        device
    }

}