use cpal::platform::{Device, Host};

use crate::core::audio_controllers::cpal::CpalBackend;
#[cfg(all(target_os = "linux", feature = "pulse"))]
use crate::core::audio_controllers::pulseaudio::PulseBackend;

use crate::core::thread_messages::DeviceListItem;

pub fn get_any_backend() -> Box<dyn AudioBackend> {
    #[cfg(not(all(target_os = "linux", feature = "pulse")))]
    return Box::new(CpalBackend {});

    #[cfg(all(target_os = "linux", feature = "pulse"))]
    if let Some(backend) = PulseBackend::try_init() {
        Box::new(backend)
    } else {
        Box::new(CpalBackend {})
    }
}

pub trait AudioBackend {
    fn list_devices(&mut self, host: &Host) -> Vec<DeviceListItem>;

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device;
}
