use cpal::platform::{Host, Device};

use crate::audio_controllers::cpal::CpalBackend;
#[cfg(feature = "pulse")]
use crate::audio_controllers::pulseaudio::PulseBackend;

use crate::core::thread_messages::DeviceListItem;

pub fn get_any_backend() -> Box<dyn AudioBackend> {
    #[cfg(not(feature = "pulse"))]
    return Box::new(CpalBackend { });

    #[cfg(feature = "pulse")]
    if let Some(backend) = PulseBackend::try_init() {
        return Box::new(backend);
    }
    else {
        return Box::new(CpalBackend { });
    }
}

pub trait AudioBackend {
    fn list_devices(&mut self, host: &Host) -> Vec<DeviceListItem>;

    fn set_device(&mut self, host: &Host, inner_name: &str) -> Device;
}