use gio::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::clone;
use glib::Value;
use std::cell::RefCell;
use adw::prelude::*;
use gettextrs::gettext;
use std::error::Error;
use log::{error, info, debug, trace};

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::logging::Logging;
use crate::core::thread_messages::{*, GUIMessage::*};

use crate::gui::preferences::{PreferencesInterface, Preferences};
use crate::gui::listed_device::ListedDevice;

pub fn gui_main(
    log_object: Logging,
    recording: bool,
    input_file: Option<String>,
    enable_mpris_cli: bool
) -> Result<(), Box<dyn Error>> {
    
    let app = App::new(log_object);
    app.run(recording, input_file);

    Ok(())
}

struct App {
    builder: gtk::Builder,
    builder_scope: gtk::BuilderRustScope,

    preferences_interface: RefCell<PreferencesInterface>,
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

// #[gtk::template_callbacks(functions)]
impl App {
    fn new(log_object: Logging) -> App {
        let (gui_tx, gui_rx) = async_channel::unbounded();
        let (microphone_tx, microphone_rx) = async_channel::unbounded();
        let (processing_tx, processing_rx) = async_channel::unbounded();
        let (http_tx, http_rx) = async_channel::unbounded();

        log_object.connect_to_gui_logger(gui_tx.clone());

        Self::load_resources();

        gtk::init().unwrap();
        gtk::glib::set_prgname(Some("re.fossplant.songrec"));

        let builder = gtk::Builder::new();

        let builder_scope = gtk::BuilderRustScope::new();
        // Self::add_callbacks_to_scope(&scope);
        builder.set_scope(Some(&builder_scope));

        let preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();
        let preferences_interface = RefCell::new(preferences_interface);

        App {
            builder,
            builder_scope,

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

    fn run(self, set_recording: bool, input_file: Option<String>) {
        let application = adw::Application::new(Some("re.fossplant.songrec"),
            gio::ApplicationFlags::HANDLES_OPEN);

        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.Application.html
        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/prelude/trait.ApplicationExtManual.html#method.run
        // => https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.ApplicationFlags.html#associatedconstant.HANDLES_COMMAND_LINE

        // We create a callback for handling files to recognize opened
        // from the command line or through "xdg-open".
        
        let processing_tx = self.processing_tx.clone();

        application.connect_open(move |_application, files, _hint| {
            if files.len() >= 1 {
                if let Some(file_path) = files[0].path() {
                    let file_path_string = file_path.into_os_string().into_string().unwrap();
                    
                    processing_tx.send_blocking(ProcessingMessage::ProcessAudioFile(file_path_string)).unwrap();
                }
            }
        });

        application.connect_activate(move |application| {
            let main_window = &application.windows()[0];

            // Raise/highlight the existing window whenever a second
            // GUI instance is attempted to be launched
            main_window.present();
        });

        self.setup_callbacks();
        self.builder.add_from_resource("/re/fossplant/songrec/interface.ui").unwrap();

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

    fn setup_callbacks(&self) {

        let microphone_tx = self.microphone_tx.clone();
        let gui_tx = self.gui_tx.clone();
        let builder = self.builder.clone();

        self.builder_scope.add_callback("loopback_options_switched", move |values| {
            let loopback_switch: adw::SwitchRow = builder.object("loopback_switch").unwrap();
            let microphone_switch: adw::SwitchRow = builder.object("microphone_switch").unwrap();
            let g_list_store: gio::ListStore = builder.object("audio_inputs_model").unwrap();

            if loopback_switch.is_active() {
                microphone_switch.set_active(false);

                let adw_combo_row: adw::ComboRow = builder.object("audio_inputs").unwrap();

                if let Some(current_device) = adw_combo_row.selected_item() {
                    let current_device = current_device.downcast::<ListedDevice>().unwrap();

                    if !current_device.is_monitor() {
                        // Choose a monitor mode device instead
                        
                        for position in 0..g_list_store.n_items() {
                            let other_device = g_list_store.item(position).unwrap()
                                .downcast::<ListedDevice>().unwrap();
                            
                            if other_device.is_monitor() {
                                adw_combo_row.set_selected(position);
                                break;
                            }
                        }
                    }
                    else {
                        microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStop).unwrap();
                        microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStart(
                            current_device.inner_name().to_owned()
                        )).unwrap();
                    }
                }
            }

            else if !microphone_switch.is_active() && !loopback_switch.is_active() {
                microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStop).unwrap();
            }

            None
        });

        let microphone_tx = self.microphone_tx.clone();
        let builder = self.builder.clone();

        self.builder_scope.add_callback("microphone_option_switched", move |values| {
            let microphone_switch: adw::SwitchRow = builder.object("microphone_switch").unwrap();
            let loopback_switch: adw::SwitchRow = builder.object("loopback_switch").unwrap();
            let g_list_store: gio::ListStore = builder.object("audio_inputs_model").unwrap();

            if microphone_switch.is_active() {
                loopback_switch.set_active(false);

                let adw_combo_row: adw::ComboRow = builder.object("audio_inputs").unwrap();

                if let Some(current_device) = adw_combo_row.selected_item() {
                    let current_device = current_device.downcast::<ListedDevice>().unwrap();

                    if current_device.is_monitor() {
                        // Choose a non-monitor mode device instead
                        
                        for position in 0..g_list_store.n_items() {
                            let other_device = g_list_store.item(position).unwrap()
                                .downcast::<ListedDevice>().unwrap();
                            
                            if !other_device.is_monitor() {
                                adw_combo_row.set_selected(position);
                                break;
                            }
                        }
                    }
                    else {
                        microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStop).unwrap();
                        microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStart(
                            current_device.inner_name().to_owned()
                        )).unwrap();
                    }
                }
            }

            else if !microphone_switch.is_active() && !loopback_switch.is_active() {
                microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStop).unwrap();
            }

            None
        });

        let microphone_tx = self.microphone_tx.clone();
        let builder = self.builder.clone();

        self.builder_scope.add_callback("input_device_switched", move |values| {
            let microphone_switch: adw::SwitchRow = builder.object("microphone_switch").unwrap();
            let loopback_switch: adw::SwitchRow = builder.object("loopback_switch").unwrap();

            let combo_row = values[0].get::<adw::ComboRow>().unwrap();

            // Plug the sound

            if let Some(device) = combo_row.selected_item() {
                let device = device.downcast::<ListedDevice>().unwrap();

                let device_name = device.inner_name();
                let is_monitor = device.is_monitor();

                if microphone_switch.is_active() && is_monitor {
                
                    microphone_switch.set_active(false);
                    loopback_switch.set_active(true);
                }
                else if loopback_switch.is_active() && !is_monitor {
                
                    loopback_switch.set_active(false);
                    microphone_switch.set_active(true);
                }

                // Save the selected microphone device name so that it is
                // remembered after relaunching the app
                
                let mut new_preference = Preferences::new();
                new_preference.current_device_name = Some(device_name.to_string());
                gui_tx.send_blocking(GUIMessage::UpdatePreference(new_preference)).unwrap();
        
                // Should we start recording yet? (will depend of the possible
                // command line flags of the application)

                if microphone_switch.is_active() || loopback_switch.is_active() {
                    microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStop).unwrap();
                    microphone_tx.send_blocking(MicrophoneMessage::MicrophoneRecordStart(
                        device_name.to_owned()
                    )).unwrap();
                }
            }
            None
        });
    }

    /* fn sync_selected_device(&self) {

    } */

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
        let mut preferences_interface_ptr = self.preferences_interface.clone();

        let old_device_name = self.old_preferences.current_device_name.clone();
        
        let window: gtk::ApplicationWindow = self.builder.object("main_window").unwrap();
        let adw_combo_row: adw::ComboRow = self.builder.object("audio_inputs").unwrap();
        let about_dialog: adw::AboutDialog = self.builder.object("about_dialog").unwrap(); 
        let g_list_store: gio::ListStore = self.builder.object("audio_inputs_model").unwrap();
        let microphone_switch: adw::SwitchRow = self.builder.object("microphone_switch").unwrap();
        let recognize_file_row: adw::PreferencesRow = self.builder.object("recognize_file_row").unwrap();
        let spinner_row: adw::PreferencesRow = self.builder.object("spinner_row").unwrap();
        let volume_row: adw::PreferencesRow = self.builder.object("volume_row").unwrap();
        let volume_gauge: gtk::ProgressBar = self.builder.object("volume_gauge").unwrap();
        let results_section: adw::PreferencesGroup = self.builder.object("results_section").unwrap();
        let no_network_message: gtk::Label = self.builder.object("no_network_message").unwrap();
        let results_image: gtk::Image = self.builder.object("results_image").unwrap();
        let results_label: gtk::Label = self.builder.object("results_label").unwrap();
        let loopback_switch: adw::SwitchRow = self.builder.object("loopback_switch").unwrap();

        microphone_switch.set_active(set_recording);
        
        let microphone_tx_2 = self.microphone_tx.clone();
        gtk::glib::spawn_future_local(async move {
            while let Ok(gui_message) = gui_rx.recv().await {

                if let AppendToLog(log_string) = gui_message {
                    // Disabled for now, causes freeze when recognizing a song
                    // because of the pixbuf which is very large

                    /* const MAX_LOG_SIZE: usize = 20 * 1024 * 1024; // 20 MB

                    let mut buffer_ptr: &str = &about_dialog.debug_info();
                    if buffer_ptr.len() > MAX_LOG_SIZE {
                        buffer_ptr = &buffer_ptr[..buffer_ptr.len() - MAX_LOG_SIZE];
                    }

                    let mut buffer: String = buffer_ptr.to_owned();
                    buffer.push_str(&log_string);

                    about_dialog.set_debug_info(&buffer); */
                }
                else {

                    if let MicrophoneVolumePercent(_) = gui_message {
                        trace!("Received GUI message: {:?}", gui_message);
                    }
                    else {
                        debug!("Received GUI message: {:?}", gui_message);
                    }
                    
                    match gui_message {
                        ErrorMessage(_) | NetworkStatus(_) | SongRecognized(_) => {
                            recognize_file_row.set_sensitive(true);
                            spinner_row.set_visible(false);
                        },
                        _ =>  { }
                    }

                    match gui_message {

                        UpdatePreference(new_preference) => {
                            preferences_interface_ptr.get_mut().update(new_preference);
                        },
                        ErrorMessage(string) => {
                            if !(string == gettext("No match for this song") && (
                                microphone_switch.is_active() || loopback_switch.is_active()
                            )) {
                                let dialog = gtk::AlertDialog::builder()
                                    .message(&string)
                                    .build();
                                dialog.show(Some(&window));
                            }
                        },
                        NetworkStatus(network_is_reachable) => {
                            no_network_message.set_visible(!network_is_reachable);
                        },
                        SongRecognized(message) => {
                            results_section.set_visible(true);
                            if let Some(cover_image) = message.cover_image {
                                if let Ok(texture) = gdk::Texture::from_bytes(
                                    &glib::Bytes::from(&cover_image)
                                ) {
                                    results_image.set_visible(true);
                                    results_image.set_paintable(Some(&texture));
                                }
                                else {
                                    results_image.set_visible(false);
                                }
                            }
                            else {
                                results_image.set_visible(false);
                            }
                            results_label.set_label(&format!("{} - {}", message.artist_name, message.song_name));

                            // https://gtk-rs.org/gtk4-rs/git/docs/gdk4/struct.Texture.html#method.from_bytes
                            // https://docs.gtk.org/gdk4/ctor.Texture.new_from_bytes.html
                            // The file format is detected automatically. The supported formats are PNG, JPEG and TIFF, though more formats might be available.

                            // + https://gtk-rs.org/gtk4-rs/git/docs/gtk4/struct.Image.html#method.set_paintable
                            // + https://docs.gtk.org/gtk4/method.Image.set_from_paintable.html
                        },
                        // This message is sent once in the program execution for
                        // the moment (maybe it should be updated automatically
                        // later?):
                        DevicesList(devices) => {
                            let mut initial_device_index: u32 = 0;
                            let mut initial_device: Option<ListedDevice> = None;
                            let mut found_monitor_device = false;
                            let mut current_index: u32 = 0;

                            // Fill in the list of available devices, and
                            // set back the old device if it was recorded

                            g_list_store.remove_all();

                            for device in devices.iter() { // device: thread_messages::DeviceListItem
                                let listed_device = ListedDevice::new(
                                    device.display_name.clone(),
                                    device.inner_name.clone(),
                                    device.is_monitor
                                );
                                g_list_store.append(&listed_device);
                                
                                if old_device_name == Some(device.inner_name.to_string()) {
                                    initial_device_index = current_index;
                                    initial_device = Some(listed_device);
                                }
                                else if old_device_name == None && device.is_monitor && !found_monitor_device {
                                    initial_device_index = current_index;
                                    initial_device = Some(listed_device);
                                }
                                else if current_index == 0 {
                                    initial_device = Some(listed_device);
                                }
                                current_index += 1;
                            
                                if device.is_monitor {
                                    found_monitor_device = true;
                                }
                            }

                            if let Some(device) = initial_device { // device: ListedDevice
                                adw_combo_row.set_selected(initial_device_index);
                                loopback_switch.set_visible(found_monitor_device);

                                debug!("Initally selected audio input device: {:?} / {:?}", device.inner_name(), device.display_name());

                                microphone_switch.set_visible(true);
                                volume_row.set_visible(true);

                                // Will trigger the "input_device_switched" callback
                            }
                            
                        },
                        MicrophoneRecording => { },

                        MicrophoneVolumePercent(percent) => {
                            volume_gauge.set_fraction((percent / 100.0) as f64);
                        },

                        _ => {
                            debug!("(parsing unimplemented yet): {:?}", gui_message);
                        }
                    }
                    
                    // TODO handle missing messages here
                }
            }
        });
    }

    fn setup_actions(&self) {
        let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
        let file_picker: gtk::FileDialog = self.builder.object("file_picker").unwrap();
        let about_dialog: adw::AboutDialog = self.builder.object("about_dialog").unwrap();
        let recognize_file_row: adw::PreferencesRow = self.builder.object("recognize_file_row").unwrap();
        let spinner_row: adw::PreferencesRow = self.builder.object("spinner_row").unwrap();

        let action_show_about = gio::ActionEntry::builder("show-about")
            .activate(
                move |window, _, _| {
                    about_dialog.present(Some(window));
                }
            )
            .build();
        
        let processing_tx = self.processing_tx.clone();

        let action_recognize_file = gio::ActionEntry::builder("recognize-file")
            .activate(
                move |window, action, obj| {
                    // Call a XDG file picker here

                    let processing_tx = processing_tx.clone();

                    let window: &adw::ApplicationWindow = window;
                    let recognize_file_row = recognize_file_row.clone();
                    let spinner_row = spinner_row.clone();

                    file_picker.open(Some(window), None::<&gio::Cancellable>, move |file| {

                        match file {
                            Ok(gio_file) => {
                                info!("Picked file: {:?}", gio_file.path());
                                let path_str = gio_file.path().unwrap().to_string_lossy().into_owned();

                                recognize_file_row.set_sensitive(false);
                                spinner_row.set_visible(true);

                                processing_tx.send_blocking(ProcessingMessage::ProcessAudioFile(path_str)).unwrap();
                            },
                            Err(error) => {
                                error!("Error picking file: {:?}", error);
                            }
                        }
                    });
                }
            )
            .build();

        let action_search_youtube = gio::ActionEntry::builder("search-youtube")
            .activate(
                move |window, action, obj| {
                    info!("TEST 4");
                }
            )
            .build();

        let action_export_to_csv = gio::ActionEntry::builder("export-to-csv")
            .activate(
                move |window, action, obj| {
                    info!("TEST 6");
                }
            )
            .build();

        let action_wipe_history = gio::ActionEntry::builder("wipe-history")
            .activate(
                move |window, action, obj| {
                    info!("TEST 7");
                }
            )
            .build();
        
        let gui_tx = self.gui_tx.clone();
        
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
        
        let gui_tx = self.gui_tx.clone();
        
        let action_notification_setting = gio::ActionEntry::builder("notification-setting")
            .state(self.old_preferences.enable_notifications.unwrap().to_variant())
            .activate(move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.enable_notifications = Some(new_state);
                gui_tx.send_blocking(GUIMessage::UpdatePreference(new_preference)).unwrap();

            })
            .build();

        window.add_action_entries([
            action_show_about,
            #[cfg(feature = "mpris")]
            action_mpris_setting, // DON'T FORGET to put a tooltip for this
            action_notification_setting,
            action_recognize_file,
            action_search_youtube,
            action_export_to_csv,
            action_wipe_history,
            // WIP xx
        ]);
    }

    fn show_window(&self, application: &adw::Application) {
        let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
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