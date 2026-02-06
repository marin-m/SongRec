use gio::prelude::*;
use gtk::prelude::*;
use gtk::glib::clone;
use adw::prelude::*;
use gettextrs::gettext;
use std::error::Error;
use log::{debug, trace};

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::thread_messages::{*, GUIMessage::*};

use crate::gui::preferences::{PreferencesInterface, Preferences};
use crate::gui::listed_device::ListedDevice;

pub fn gui_main(recording: bool, input_file: Option<String>, enable_mpris_cli: bool) -> Result<(), Box<dyn Error>> {
    
    let app = App::new();
    app.run(recording, input_file);

    Ok(())
}

struct App {
    builder: gtk::Builder,
    preferences_interface: PreferencesInterface,
    old_preferences: Preferences,

    gui_tx: async_channel::Sender<GUIMessage>,
    gui_rx: async_channel::Receiver<GUIMessage>,
    microphone_tx: async_channel::Sender<MicrophoneMessage>, // WIP switch everything to async_channel so that we can clone receivers too
    microphone_rx: async_channel::Receiver<MicrophoneMessage>,
    processing_tx: async_channel::Sender<ProcessingMessage>,
    processing_rx: async_channel::Receiver<ProcessingMessage>,
    http_tx: async_channel::Sender<HTTPMessage>,
    http_rx: async_channel::Receiver<HTTPMessage>
}

impl App {
    fn new() -> App {
        let (gui_tx, gui_rx) = async_channel::unbounded();
        let (microphone_tx, microphone_rx) = async_channel::unbounded();
        let (processing_tx, processing_rx) = async_channel::unbounded();
        let (http_tx, http_rx) = async_channel::unbounded();

        Self::load_resources();

        gtk::init().unwrap();
        gtk::glib::set_prgname(Some("re.fossplant.songrec"));

        let builder = gtk::Builder::from_resource("/re/fossplant/songrec/interface.ui");

        let preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();

        App {
            builder,
            preferences_interface,
            old_preferences,

            gui_tx, gui_rx,
            microphone_tx, microphone_rx,
            processing_tx, processing_rx,
            http_tx, http_rx
        }
    }

    fn load_resources() {
        gio::resources_register_include!("compiled.gresource")
            .expect("Failed to register resources.");
    }

    fn setup_intercom(&self, set_recording: bool) {
        // WIP: Setup threads + smol-rs/async-channel::unbounded listener

        // NOTE: Dropping the removed glib::MainContext from legacy code:
        // https://discourse.gnome.org/t/help-required-to-migrate-from-dropped-maincontext-channel-api/20922
        // + https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html#how-to-avoid-blocking-the-main-loop

        let microphone_rx = self.microphone_rx.clone();
        let processing_tx = self.processing_tx.clone();
        let gui_tx = self.gui_tx.clone();
        spawn_big_thread(move || {
            microphone_thread(microphone_rx, processing_tx, gui_tx);
        });

        let processing_rx = self.processing_rx.clone();
        let http_tx = self.http_tx.clone();
        let gui_tx = self.gui_tx.clone();
        spawn_big_thread(move || {
            processing_thread(processing_rx, http_tx, gui_tx);
        });

        let http_rx = self.http_rx.clone();
        let gui_tx = self.gui_tx.clone();
        let microphone_tx = self.microphone_tx.clone();
        spawn_big_thread(move || {
            http_thread(http_rx, gui_tx, microphone_tx);
        });

        let gui_rx = self.gui_rx.clone();

        let old_device_name = self.old_preferences.current_device_name.clone();
        
        let adw_combo_row: adw::ComboRow = self.builder.object("audio_inputs").unwrap();
        let g_list_store: gio::ListStore = self.builder.object("audio_inputs_model").unwrap();
        let microphone_switch: adw::SwitchRow = self.builder.object("microphone_switch").unwrap();
        let volume_row: adw::PreferencesRow = self.builder.object("volume_row").unwrap();
        let volume_gauge: gtk::ProgressBar = self.builder.object("volume_gauge").unwrap();
        let loopback_switch: adw::SwitchRow = self.builder.object("loopback_switch").unwrap();

        microphone_switch.set_active(set_recording);
        
        let microphone_tx_2 = self.microphone_tx.clone();
        gtk::glib::spawn_future_local(async move {
            while let Ok(gui_message) = gui_rx.recv().await {

                if let MicrophoneVolumePercent(_) = gui_message {
                    trace!("Received GUI message: {:?}", gui_message);
                }
                else {
                    debug!("Received GUI message: {:?}", gui_message);
                }
                
                match gui_message {
                    ErrorMessage(_) | NetworkStatus(_) | SongRecognized(_) => {
                        // TODO hide spinner
                    },
                    _ =>  { }
                }

                match gui_message {

                    // This message is sent once in the program execution for
                    // the moment (maybe it should be updated automatically
                    // later?):
                    DevicesList(devices) => {
                        let mut old_device_index: u32 = 0;
                        let mut old_device: Option<ListedDevice> = None;
                        let mut found_monitor_device = false;
                        let mut current_index: u32 = 0;

                        // Fill in the list of available devices, and
                        // set back the old device if it was recorded

                        g_list_store.remove_all();

                        for device in devices.iter() {
                            let listed_device = ListedDevice::new(
                                device.display_name.clone(),
                                device.inner_name.clone(),
                                device.is_monitor
                            );
                            g_list_store.append(&listed_device);
                            
                            if old_device_name == Some(device.inner_name.to_string()) {
                                old_device_index = current_index;
                                old_device = Some(listed_device);
                            }
                            else if old_device_name == None && device.is_monitor && !found_monitor_device {
                                old_device_index = current_index;
                                old_device = Some(listed_device);
                            }
                            else if current_index == 0 {
                                old_device = Some(listed_device);
                            }
                            current_index += 1;
                        
                            if device.is_monitor {
                                found_monitor_device = true;
                            }
                        }

                        if let Some(device) = old_device {
                            adw_combo_row.set_selected(old_device_index);
                            loopback_switch.set_visible(found_monitor_device);

                            let device_name = device.inner_name();
                            let is_monitor = device.is_monitor();

                            debug!("Initally selected audio input device: {:?} / {:?}", device.inner_name(), device.display_name());

                            microphone_switch.set_visible(true);
                            volume_row.set_visible(true);

                            if microphone_switch.is_active() && is_monitor {
                            
                                microphone_switch.set_active(false);
                                loopback_switch.set_active(true);
                            }
                    
                            // Should we start recording yet? (will depend of the possible
                            // command line flags of the application)

                            if microphone_switch.is_active() || loopback_switch.is_active() {
                                microphone_tx_2.send(MicrophoneMessage::MicrophoneRecordStart(
                                    device_name.to_owned()
                                )).await.unwrap();
                            }
                        }
                        
                    },

                    MicrophoneVolumePercent(percent) => {
                        volume_gauge.set_fraction((percent / 100.0) as f64);
                    },

                    _ => {
                        debug!("(parsing unimplemented yet): {:?}", gui_message);
                    }
                }
                
                // TODO handle missing messages here
            }
        });
    }

    fn run(self, set_recording: bool, input_file: Option<String>) {
        let application = adw::Application::new(Some("re.fossplant.songrec"),
            gio::ApplicationFlags::HANDLES_OPEN);

        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.Application.html
        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/prelude/trait.ApplicationExtManual.html#method.run
        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.ApplicationFlags.html#associatedconstant.HANDLES_COMMAND_LINE

        self.setup_open_action(&application);

        application.connect_activate(move |application| {
            let main_window = &application.windows()[0];

            // Raise the existing window to the top whenever a second
            // GUI instance is attempted to be launched
            main_window.present();
        });

        application.connect_startup(move |application| {
            self.on_startup(application, set_recording);
        });
        if let Some(input_file_string) = input_file {
            application.run_with_args(&["songrec".to_string(), input_file_string]);
        }
        else {
            application.run_with_args(&["songrec".to_string()]);
        }
    }

    fn on_startup(&self, application: &adw::Application, set_recording: bool) {
        self.setup_intercom(set_recording);
        self.setup_actions();
        self.show_window(application);
    }

    fn setup_actions(&self) {
        let window: adw::ApplicationWindow = self.builder.object("window").unwrap();
        let about_dialog: adw::AboutDialog = self.builder.object("about_dialog").unwrap();

        let action_show_about = gio::ActionEntry::builder("show-about")
            .activate(
                move |window, _, _| {
                    about_dialog.present(Some(window));
                }
            )
            .build();
        
        let gui_tx = self.gui_tx.clone();
        let gui_tx_2 = self.gui_tx.clone();
        
        #[cfg(feature = "mpris")]
        let action_mpris_setting = gio::ActionEntry::builder("mpris-setting")
            .state(self.old_preferences.enable_mpris.unwrap().to_variant())
            .activate(move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.enable_mpris = Some(new_state);
                gui_tx.send_blocking(GUIMessage::UpdatePreference(new_preference)).unwrap();

            })
            .build();
        
        let action_notification_setting = gio::ActionEntry::builder("notification-setting")
            .state(self.old_preferences.enable_notifications.unwrap().to_variant())
            .activate(move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.enable_notifications = Some(new_state);
                gui_tx_2.send_blocking(GUIMessage::UpdatePreference(new_preference)).unwrap();

            })
            .build();

        window.add_action_entries([
            action_show_about,
            #[cfg(feature = "mpris")]
            action_mpris_setting, // DON'T FORGET to put a tooltip for this
            action_notification_setting,
            // WIP xx
        ]);
    }

    fn setup_open_action(&self, application: &adw::Application) {
        let processing_tx = self.processing_tx.clone();

        // We create a callback for handling files to recognize opened
        // from the command line or through "xdg-open".
        
        application.connect_open(move |_application, files, _hint| {
            if files.len() >= 1 {
                if let Some(file_path) = files[0].path() {
                    let file_path_string = file_path.into_os_string().into_string().unwrap();
                    
                    processing_tx.send_blocking(ProcessingMessage::ProcessAudioFile(file_path_string)).unwrap();
                }
            }
        });
    }

    fn show_window(&self, application: &adw::Application) {
        let window: adw::ApplicationWindow = self.builder.object("window").unwrap();
        window.set_application(Some(application));

        /* let quit = gio::SimpleAction::new("quit", None);
        quit.connect_activate(glib::clone!(#[strong] application, move |_,_| {
            application.quit();
        })); 
        application.set_accels_for_action("app.quit", &["<Primary>Q"]);
        application.add_action(&quit);*/

        window.present();
    }
}