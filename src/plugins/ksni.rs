use crate::core::thread_messages::GUIMessage;
use gettextrs::gettext;
use ksni::TrayMethods;

pub struct SystrayInterface {
    pub gui_tx: async_channel::Sender<GUIMessage>,
}

impl ksni::Tray for SystrayInterface {
    fn id(&self) -> String {
        "songrec".into()
    }
    /* fn icon_name(&self) -> String {
        "re.fossplant.songrec-symbolic".into()
    } */
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let img = gdk_pixbuf::Pixbuf::from_resource("re.fossplant.songrec-symbolic.png").unwrap();
        let mut data = img.read_pixel_bytes().to_vec();
        assert_eq!(data.len() % 4, 0);
        for pixel in data.as_chunks_mut::<4>().0 {
            pixel.rotate_right(1) // rgba to argb
        }
        vec![ksni::Icon {
            width: img.width(),
            height: img.height(),
            data,
        }]
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        self.gui_tx.try_send(GUIMessage::ShowWindow).unwrap();
    }
    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        self.gui_tx.try_send(GUIMessage::ShowWindow).unwrap();
        true
    }
    fn title(&self) -> String {
        "SongRec".into()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: gettext("Open SongRec"),
                activate: Box::new(|tray: &mut Self| {
                    tray.gui_tx.try_send(GUIMessage::ShowWindow).unwrap();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Quit..."),
                activate: Box::new(|tray: &mut Self| {
                    tray.gui_tx.try_send(GUIMessage::QuitApplication).unwrap();
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl SystrayInterface {
    pub async fn try_enable(
        gui_tx: async_channel::Sender<GUIMessage>,
    ) -> Result<ksni::Handle<Self>, ksni::Error> {
        match std::env::var("SNAP_NAME") {
            Ok(_) => {
                Self { gui_tx }
                    .disable_dbus_name(true)
                    .assume_sni_available(true)
                    .spawn()
                    .await
            }
            _ => Self { gui_tx }.disable_dbus_name(true).spawn().await,
        }
    }

    pub async fn disable(handle: &ksni::Handle<Self>) {
        handle.shutdown().await;
    }
}
