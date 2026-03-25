use crate::core::thread_messages::GUIMessage;
use gettextrs::gettext;
use image::GenericImageView;
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
        let img = image::load_from_memory_with_format(
            include_bytes!("../../packaging/rootfs/usr/share/icons/hicolor/scalable/apps/re.fossplant.songrec-symbolic.png"),
            image::ImageFormat::Png,
        ).unwrap();
        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();
        assert_eq!(data.len() % 4, 0);
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1) // rgba to argb
        }
        vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
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
                label: gettext("Open SongRec").into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.gui_tx.try_send(GUIMessage::ShowWindow).unwrap();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Quit...").into(),
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
