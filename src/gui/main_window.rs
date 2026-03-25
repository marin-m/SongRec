use adw::prelude::*;
use chrono::Local;
use gettextrs::gettext;
use log::{debug, error, info, trace};
#[cfg(feature = "mpris")]
use mpris_server::PlaybackStatus;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::json;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::core::http_task::http_task;
use crate::core::logging::Logging;
use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::thread_messages::{GUIMessage::*, *};

use crate::gui::song_history_interface::FavoritesInterface;

use crate::gui::song_history_interface::{RecognitionHistoryInterface, SongRecordInterface};
#[cfg(target_os = "linux")]
use crate::plugins::ksni::SystrayInterface;
#[cfg(feature = "mpris")]
use crate::plugins::mpris_player::{get_player, update_song};
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::utils::filesystem_operations::{
    clear_cache, obtain_favorites_csv_path, obtain_recognition_history_csv_path,
};

use crate::core::preferences::{Preferences, PreferencesInterface};

use crate::gui::context_menu::ContextMenuUtil;
use crate::gui::history_entry::HistoryEntry;
use crate::gui::listed_device::ListedDevice;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn gui_main(
    log_object: Logging,
    recording: bool,
    input_file: Option<String>,
    enable_mpris_cli: bool,
) -> Result<(), Box<dyn Error>> {
    let app = App::new(log_object);
    app.run(recording, enable_mpris_cli, input_file);

    Ok(())
}

struct App {
    builder: gtk::Builder,

    preferences_interface: Arc<Mutex<PreferencesInterface>>,
    song_history_interface: Rc<RefCell<RecognitionHistoryInterface>>,
    favorites_interface: Rc<RefCell<FavoritesInterface>>,
    old_preferences: Preferences,

    ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
    ctx_buffered_log: Rc<RefCell<String>>,
    #[cfg(target_os = "linux")]
    ctx_systray_handle: Rc<RefCell<Option<ksni::Handle<SystrayInterface>>>>,
    ctx_logger_source_id: Rc<RefCell<Option<glib::source::SourceId>>>,

    gui_tx: async_channel::Sender<GUIMessage>,
    gui_rx: async_channel::Receiver<GUIMessage>,
    microphone_tx: async_channel::Sender<MicrophoneMessage>,
    microphone_rx: async_channel::Receiver<MicrophoneMessage>,
    processing_tx: async_channel::Sender<ProcessingMessage>,
    processing_rx: async_channel::Receiver<ProcessingMessage>,
    http_tx: async_channel::Sender<HTTPMessage>,
    http_rx: async_channel::Receiver<HTTPMessage>,
}

// #[gtk::template_callbacks(functions)]
impl App {
    fn new(log_object: Logging) -> App {
        let (gui_tx, gui_rx) = async_channel::unbounded();
        let (microphone_tx, microphone_rx) = async_channel::unbounded();
        let (processing_tx, processing_rx) = async_channel::unbounded();
        let (http_tx, http_rx) = async_channel::unbounded();

        log_object.connect_to_gui_logger(gui_tx.clone());

        glib::set_prgname(Some(match std::env::var("SNAP_NAME") {
            Ok(_) => "com.github.marinm.songrec",
            _ => "re.fossplant.songrec",
        }));
        Self::load_resources();

        let ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>> = Rc::new(RefCell::new(None));
        let ctx_buffered_log: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
        let ctx_logger_source_id: Rc<RefCell<Option<glib::source::SourceId>>> =
            Rc::new(RefCell::new(None));

        let history_list_store: gio::ListStore = gio::ListStore::new::<HistoryEntry>();
        let song_history_interface = Rc::new(RefCell::new(
            RecognitionHistoryInterface::new(
                history_list_store.clone(),
                obtain_recognition_history_csv_path,
            )
            .unwrap(),
        ));

        let favorites_list_store = gio::ListStore::new::<HistoryEntry>();
        let favorites_interface = Rc::new(RefCell::new(
            FavoritesInterface::new(favorites_list_store.clone(), obtain_favorites_csv_path)
                .unwrap(),
        ));

        let builder = gtk::Builder::new();

        let builder_scope = gtk::BuilderRustScope::new();
        // Self::add_callbacks_to_scope(&scope);
        builder.set_scope(Some(&builder_scope));

        Self::setup_callbacks(
            microphone_tx.clone(),
            gui_tx.clone(),
            builder.clone(),
            builder_scope,
            favorites_interface.clone(),
            ctx_selected_item.clone(),
        );
        builder
            .add_from_resource("/re/fossplant/songrec/interface.ui")
            .unwrap();

        let history_selection: gtk::SingleSelection = builder.object("history_selection").unwrap();
        history_selection.set_model(Some(&history_list_store));

        let favorites_selection: gtk::SingleSelection =
            builder.object("favorites_selection").unwrap();
        favorites_selection.set_model(Some(&favorites_list_store));

        let preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();
        let preferences_interface = Arc::new(Mutex::new(preferences_interface));

        let buffer_size_value: gtk::Adjustment = builder.object("buffer_size_value").unwrap();
        buffer_size_value.set_value(old_preferences.buffer_size_secs.unwrap() as f64);

        let request_interval_value: gtk::Adjustment = builder.object("interval_value").unwrap();
        request_interval_value.set_value(old_preferences.request_interval_secs_v3.unwrap() as f64);

        App {
            builder,

            song_history_interface,
            favorites_interface,
            preferences_interface,
            old_preferences,

            #[cfg(target_os = "linux")]
            ctx_systray_handle: Rc::new(RefCell::new(None)),

            ctx_selected_item,
            ctx_buffered_log,
            ctx_logger_source_id,

            gui_tx,
            gui_rx,
            microphone_tx,
            microphone_rx,
            processing_tx,
            processing_rx,
            http_tx,
            http_rx,
        }
    }

    fn load_resources() {
        gio::resources_register_include!("compiled.gresource")
            .expect("Failed to register resources.");

        gtk::init().unwrap();

        if let Some(display) = gdk::Display::default() {
            let icon_theme = gtk::IconTheme::for_display(&display);
            icon_theme.add_resource_path("/re/fossplant/songrec/");
        }

        let css_theme = gtk::CssProvider::new();
        css_theme.load_from_resource("/re/fossplant/songrec/style.css");
    }

    fn run(self, set_recording: bool, enable_mpris_cli: bool, input_file: Option<String>) {
        let application = adw::Application::new(
            // We're using a different DBus ID over Snap
            // per request of the Snapcraft team
            // (they don't want to allocate us the one
            // that we had to switch to in order to
            // get verified on Flathub)
            Some(match std::env::var("SNAP_NAME") {
                Ok(_) => "com.github.marinm.songrec",
                _ => "re.fossplant.songrec",
            }),
            gio::ApplicationFlags::HANDLES_OPEN,
        );

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

                    processing_tx
                        .try_send(ProcessingMessage::ProcessAudioFile(file_path_string))
                        .unwrap();
                }
            }
        });

        application.connect_activate(move |application| {
            let main_window = &application.windows()[0];

            // Raise/highlight the existing window whenever a second
            // GUI instance is attempted to be launched
            main_window.present();
        });

        application.connect_startup(move |application| {
            self.on_startup(application, set_recording, enable_mpris_cli);
        });

        if let Some(input_file_string) = input_file {
            application.run_with_args(&["songrec".to_string(), input_file_string]);
        } else {
            application.run_with_args(&["songrec".to_string()]);
        }
    }

    fn notify_application_error(
        preferences_interface: Arc<Mutex<PreferencesInterface>>,
        label: &str,
        application: &adw::Application,
    ) {
        if preferences_interface
            .lock()
            .unwrap()
            .preferences
            .enable_notifications
            == Some(true)
        {
            let notification = gio::Notification::new(&gettext("Application error"));
            notification.set_body(Some(&label));
            notification.set_category(Some("network.error"));
            application.send_notification(Some("application-error"), &notification);
        }
    }

    fn notify_network_error(
        preferences_interface: Arc<Mutex<PreferencesInterface>>,
        label: &str,
        application: &adw::Application,
        always: bool,
    ) {
        if always
            || preferences_interface
                .lock()
                .unwrap()
                .preferences
                .enable_notifications
                == Some(true)
        {
            let notification = gio::Notification::new(&gettext("Network error"));
            notification.set_body(Some(&label));
            notification.set_category(Some("network.error"));
            application.send_notification(Some("network-error"), &notification);
        }
    }

    fn on_startup(
        &self,
        application: &adw::Application,
        set_recording: bool,
        enable_mpris_cli: bool,
    ) {
        clear_cache();
        self.setup_intercom(application, set_recording, enable_mpris_cli);
        self.setup_actions(application, enable_mpris_cli);
        #[cfg(target_os = "linux")]
        if self.old_preferences.enable_systray == Some(true) {
            let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
            Self::setup_systray(self.ctx_systray_handle.clone(), window, self.gui_tx.clone());
        }
        self.setup_context_menus();
        self.show_window(application);
    }

    #[cfg(target_os = "linux")]
    fn setup_systray(
        ctx_systray_handle: Rc<RefCell<Option<ksni::Handle<SystrayInterface>>>>,
        window: adw::ApplicationWindow,
        gui_tx: async_channel::Sender<GUIMessage>,
    ) {
        glib::spawn_future_local(async move {
            if ctx_systray_handle.take().is_none() {
                match SystrayInterface::try_enable(gui_tx).await {
                    Ok(handle) => {
                        *ctx_systray_handle.borrow_mut() = Some(handle);
                        window.set_hide_on_close(true);
                    }
                    Err(err) => {
                        error!(
                            "{}: {:?}",
                            gettext("Unable to enable notification icon"),
                            err
                        );
                    }
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    fn unsetup_systray(
        ctx_systray_handle: Rc<RefCell<Option<ksni::Handle<SystrayInterface>>>>,
        window: adw::ApplicationWindow,
    ) {
        let window = window.clone();
        glib::spawn_future_local(async move {
            let ctx_systray_handle = ctx_systray_handle.clone();
            if let Some(handle) = ctx_systray_handle.take() {
                window.set_hide_on_close(false);
                *ctx_systray_handle.borrow_mut() = None;
                SystrayInterface::disable(&handle).await;
            }
        });
    }

    fn setup_context_menus(&self) {
        ContextMenuUtil::connect_menu_key_actions(
            self.builder.clone(),
            self.builder.object("history_view").unwrap(),
            self.builder.object("history_context_menu").unwrap(),
            self.ctx_selected_item.clone(),
            self.favorites_interface.clone(),
        );

        ContextMenuUtil::connect_menu_key_actions(
            self.builder.clone(),
            self.builder.object("favorites_view").unwrap(),
            self.builder.object("history_context_menu").unwrap(),
            self.ctx_selected_item.clone(),
            self.favorites_interface.clone(),
        );

        ContextMenuUtil::bind_actions(
            self.builder.object("main_window").unwrap(),
            self.ctx_selected_item.clone(),
            self.song_history_interface.clone(),
            self.favorites_interface.clone(),
        );

        // See:
        // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L266 (right click)
        // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L349 (context menu key)
        // https://discourse.gnome.org/t/adding-a-context-menu-to-a-listview-using-gtk4-rs/19995/5
    }

    fn setup_callbacks(
        microphone_tx_shared: async_channel::Sender<MicrophoneMessage>,
        gui_tx_shared: async_channel::Sender<GUIMessage>,
        builder_shared: gtk::Builder,
        builder_scope: gtk::BuilderRustScope,
        favorites: Rc<RefCell<FavoritesInterface>>,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
    ) {
        let microphone_tx = microphone_tx_shared.clone();
        let builder = builder_shared.clone();

        builder_scope.add_callback("history_cell_setup_cb", move |values| {
            let popover_menu: gtk::PopoverMenu = builder.object("history_context_menu").unwrap();

            let builder = builder.clone();
            let favorites = favorites.clone();
            let ctx_selected_item = ctx_selected_item.clone();

            let cell = values[1].get::<gtk::ColumnViewCell>().unwrap();
            /* let column_view = values[0]
            .get::<gtk::ColumnViewColumn>()
            .unwrap()
            .column_view()
            .unwrap(); */

            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.add_css_class("cell_label");
            cell.set_child(Some(&label));

            ContextMenuUtil::connect_menu_mouse_actions(
                builder,
                cell,
                label,
                popover_menu,
                ctx_selected_item,
                favorites,
            );

            None
        });

        builder_scope.add_callback("history_cell_bind_cb", move |values| {
            let col = values[0].get::<gtk::ColumnViewColumn>().unwrap();
            let cell = values[1].get::<gtk::ColumnViewCell>().unwrap();
            let label = cell.child().unwrap().downcast::<gtk::Label>().unwrap();
            let entry = cell.item().unwrap().downcast::<HistoryEntry>().unwrap();
            let prop_name = col.id().unwrap();

            let text = match prop_name.as_str() {
                "song_name" => entry.song_name(),
                "album" => entry.album().unwrap_or(String::new()),
                "recognition_date" => entry.recognition_date(),
                _ => unreachable!(),
            };
            label.set_text(&text);
            None
        });

        let builder = builder_shared.clone();

        builder_scope.add_callback("loopback_options_switched", move |_values| {
            let loopback_switch: adw::SwitchRow = builder.object("loopback_switch").unwrap();
            let microphone_switch: adw::SwitchRow = builder.object("microphone_switch").unwrap();
            let device_section: adw::PreferencesGroup =
                builder.object("input_device_section").unwrap();
            let g_list_store: gio::ListStore = builder.object("audio_inputs_model").unwrap();

            if loopback_switch.is_active() {
                microphone_switch.set_active(false);
                device_section.set_visible(true);

                let adw_combo_row: adw::ComboRow = builder.object("audio_inputs").unwrap();

                if let Some(current_device) = adw_combo_row.selected_item() {
                    let current_device = current_device.downcast::<ListedDevice>().unwrap();

                    if !current_device.is_monitor() {
                        // Choose a monitor mode device instead

                        for position in 0..g_list_store.n_items() {
                            let other_device = g_list_store
                                .item(position)
                                .unwrap()
                                .downcast::<ListedDevice>()
                                .unwrap();

                            if other_device.is_monitor() {
                                adw_combo_row.set_selected(position);
                                break;
                            }
                        }
                    } else {
                        microphone_tx
                            .try_send(MicrophoneMessage::MicrophoneRecordStop)
                            .unwrap();
                        microphone_tx
                            .try_send(MicrophoneMessage::MicrophoneRecordStart(
                                current_device.inner_name().to_owned(),
                            ))
                            .unwrap();
                    }
                }
            } else if !microphone_switch.is_active() && !loopback_switch.is_active() {
                device_section.set_visible(false);
                microphone_tx
                    .try_send(MicrophoneMessage::MicrophoneRecordStop)
                    .unwrap();
            }

            None
        });

        let microphone_tx = microphone_tx_shared.clone();
        let builder = builder_shared.clone();

        builder_scope.add_callback("microphone_option_switched", move |_values| {
            let microphone_switch: adw::SwitchRow = builder.object("microphone_switch").unwrap();
            let loopback_switch: adw::SwitchRow = builder.object("loopback_switch").unwrap();
            let device_section: adw::PreferencesGroup =
                builder.object("input_device_section").unwrap();
            let g_list_store: gio::ListStore = builder.object("audio_inputs_model").unwrap();

            if microphone_switch.is_active() {
                loopback_switch.set_active(false);
                device_section.set_visible(true);

                let adw_combo_row: adw::ComboRow = builder.object("audio_inputs").unwrap();

                if let Some(current_device) = adw_combo_row.selected_item() {
                    let current_device = current_device.downcast::<ListedDevice>().unwrap();

                    if current_device.is_monitor() {
                        // Choose a non-monitor mode device instead

                        for position in 0..g_list_store.n_items() {
                            let other_device = g_list_store
                                .item(position)
                                .unwrap()
                                .downcast::<ListedDevice>()
                                .unwrap();

                            if !other_device.is_monitor() {
                                adw_combo_row.set_selected(position);
                                break;
                            }
                        }
                    } else {
                        microphone_tx
                            .try_send(MicrophoneMessage::MicrophoneRecordStop)
                            .unwrap();
                        microphone_tx
                            .try_send(MicrophoneMessage::MicrophoneRecordStart(
                                current_device.inner_name().to_owned(),
                            ))
                            .unwrap();
                    }
                }
            } else if !microphone_switch.is_active() && !loopback_switch.is_active() {
                device_section.set_visible(false);
                microphone_tx
                    .try_send(MicrophoneMessage::MicrophoneRecordStop)
                    .unwrap();
            }

            None
        });

        let microphone_tx = microphone_tx_shared.clone();
        let gui_tx = gui_tx_shared.clone();
        let builder = builder_shared.clone();

        builder_scope.add_callback("input_device_switched", move |values| {
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
                } else if loopback_switch.is_active() && !is_monitor {
                    loopback_switch.set_active(false);
                    microphone_switch.set_active(true);
                }

                // Save the selected microphone device name so that it is
                // remembered after relaunching the app

                let mut new_preference = Preferences::new();
                new_preference.current_device_name = Some(device_name.to_string());
                gui_tx
                    .try_send(GUIMessage::UpdatePreference(new_preference))
                    .unwrap();

                // Should we start recording yet? (will depend of the possible
                // command line flags of the application)

                if microphone_switch.is_active() || loopback_switch.is_active() {
                    microphone_tx
                        .try_send(MicrophoneMessage::MicrophoneRecordStop)
                        .unwrap();
                    microphone_tx
                        .try_send(MicrophoneMessage::MicrophoneRecordStart(
                            device_name.to_owned(),
                        ))
                        .unwrap();
                }
            }
            None
        });

        let gui_tx = gui_tx_shared.clone();

        builder_scope.add_callback("buffer_size_changed", move |values| {
            let adjustment = values[0].get::<gtk::Adjustment>().unwrap();
            debug!("Buffer size set to: {}", adjustment.value());
            let mut new_preference = Preferences::new();
            new_preference.buffer_size_secs = Some(adjustment.value() as u64);
            gui_tx
                .try_send(GUIMessage::UpdatePreference(new_preference))
                .unwrap();
            None
        });

        let gui_tx = gui_tx_shared.clone();

        builder_scope.add_callback("interval_changed", move |values| {
            let adjustment = values[0].get::<gtk::Adjustment>().unwrap();
            debug!("Request interval set to: {}", adjustment.value());
            let mut new_preference = Preferences::new();
            new_preference.request_interval_secs_v3 = Some(adjustment.value() as u64);
            gui_tx
                .try_send(GUIMessage::UpdatePreference(new_preference))
                .unwrap();
            None
        });

        let builder = builder_shared;

        builder_scope.add_callback("about_dialog_closed", move |_values| {
            let about_dialog: adw::AboutDialog = builder.object("about_dialog").unwrap();
            about_dialog.set_visible(false);
            None
        });
    }

    fn setup_intercom(
        &self,
        application: &adw::Application,
        set_recording: bool,
        enable_mpris_cli: bool,
    ) {
        // Setup communication using threads + smol-rs/async-channel::unbounded listener

        // NOTE: Dropping the removed glib::MainContext from legacy code:
        // https://discourse.gnome.org/t/help-required-to-migrate-from-dropped-maincontext-channel-api/20922
        // + https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html#how-to-avoid-blocking-the-main-loop

        let microphone_rx = self.microphone_rx.clone();
        let microphone_tx = self.microphone_tx.clone();
        let processing_tx = self.processing_tx.clone();
        let gui_tx = self.gui_tx.clone();
        let preferences_interface = self.preferences_interface.clone();
        spawn_big_thread(move || {
            microphone_thread(
                microphone_rx,
                microphone_tx,
                processing_tx,
                gui_tx,
                preferences_interface,
            );
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
        glib::spawn_future_local(http_task(http_rx, gui_tx, microphone_tx));

        let gui_rx = self.gui_rx.clone();
        let preferences_interface_ptr = self.preferences_interface.clone();

        let old_device_name = self.old_preferences.current_device_name.clone();

        let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
        let systray_setting: adw::SwitchRow = self.builder.object("systray_setting").unwrap();
        let adw_combo_row: adw::ComboRow = self.builder.object("audio_inputs").unwrap();
        let g_list_store: gio::ListStore = self.builder.object("audio_inputs_model").unwrap();
        let microphone_switch: adw::SwitchRow = self.builder.object("microphone_switch").unwrap();
        let recognize_file_row: adw::PreferencesRow =
            self.builder.object("recognize_file_row").unwrap();
        let spinner_row: adw::PreferencesRow = self.builder.object("spinner_row").unwrap();
        let volume_row: adw::PreferencesRow = self.builder.object("volume_row").unwrap();
        let volume_gauge: gtk::ProgressBar = self.builder.object("volume_gauge").unwrap();
        let results_section: adw::PreferencesGroup =
            self.builder.object("results_section").unwrap();
        let no_network_message: gtk::Label = self.builder.object("no_network_message").unwrap();
        let rate_limited_message: gtk::Label = self.builder.object("rate_limited_message").unwrap();
        let results_image: gtk::Image = self.builder.object("results_image").unwrap();
        let results_label: gtk::Label = self.builder.object("results_label").unwrap();
        let loopback_switch: adw::SwitchRow = self.builder.object("loopback_switch").unwrap();

        #[cfg(target_os = "linux")]
        systray_setting.set_visible(true);

        microphone_switch.set_active(set_recording);

        let song_history_interface = self.song_history_interface.clone();
        let old_preferences = self.old_preferences.clone();
        let ctx_buffered_log = self.ctx_buffered_log.clone();
        let application = application.clone();

        glib::spawn_future_local(async move {
            #[cfg(feature = "mpris")]
            let mut mpris_obj = {
                let player = if enable_mpris_cli && old_preferences.enable_mpris == Some(true) {
                    let player_maybe = get_player(true).await;
                    if let Some(ref player) = player_maybe {
                        let application = application.clone();
                        let window = window.clone();
                        player.connect_quit(move |_player| {
                            application.quit();
                        });
                        player.connect_raise(move |_player| {
                            window.present();
                        });
                    }
                    player_maybe
                } else {
                    None
                };
                if enable_mpris_cli
                    && old_preferences.enable_mpris == Some(true)
                    && player.is_none()
                {
                    println!("{}", gettext("Unable to enable MPRIS support"))
                }
                player
            };
            #[cfg(feature = "mpris")]
            let mut last_cover_path = None;

            while let Ok(gui_message) = gui_rx.recv().await {
                if let AppendToLog(log_string) = gui_message {
                    const MAX_LOG_SIZE: usize = 2 * 1024 * 1024; // 2 MB

                    {
                        let buffer_ptr: &mut String = &mut *ctx_buffered_log.borrow_mut();
                        buffer_ptr.push_str(&log_string);
                        if buffer_ptr.len() > MAX_LOG_SIZE {
                            let to_cut: String = buffer_ptr
                                .chars()
                                .take(buffer_ptr.len() - MAX_LOG_SIZE)
                                .collect();
                            buffer_ptr.drain(..to_cut.len());
                        }
                    }
                } else {
                    if let MicrophoneVolumePercent(_) = gui_message {
                        trace!("Received GUI message: {:?}", gui_message);
                    } else if let SongRecognized(ref msg) = gui_message {
                        debug!("Received GUI message: SongRecognized({})", json!({
                            "artist_name": msg.artist_name.clone(),
                            "album_name": msg.album_name.clone(),
                            "song_name": msg.song_name.clone(),
                            "cover_image": match &msg.cover_image {
                                Some(data) => Some::<String>(format!("{:02x?}...", &data[..16]).into()),
                                None => None
                            },
                            "track_key": msg.track_key.clone(),
                            "release_year": msg.release_year.clone(),
                            "genre": msg.genre.clone(),
                            "shazam_json": msg.shazam_json.clone()
                        }).to_string());
                    } else {
                        debug!("Received GUI message: {:?}", gui_message);
                    }

                    match gui_message {
                        ErrorMessage(_) | NetworkStatus(_) | SongRecognized(_) => {
                            recognize_file_row.set_sensitive(true);
                            spinner_row.set_visible(false);
                        }
                        _ => {}
                    }

                    match gui_message {
                        UpdatePreference(new_preference) => {
                            preferences_interface_ptr
                                .lock()
                                .unwrap()
                                .update(new_preference);
                            #[cfg(feature = "mpris")]
                            if enable_mpris_cli {
                                let mpris_enabled = preferences_interface_ptr
                                    .lock()
                                    .unwrap()
                                    .preferences
                                    .enable_mpris
                                    == Some(true);

                                if mpris_enabled && mpris_obj.is_none() {
                                    mpris_obj = {
                                        let player_maybe = get_player(true).await;
                                        if let Some(ref player) = player_maybe {
                                            let application = application.clone();
                                            let window = window.clone();
                                            player.connect_quit(move |_player| {
                                                application.quit();
                                            });
                                            player.connect_raise(move |_player| {
                                                window.present();
                                            });
                                        } else {
                                            println!(
                                                "{}",
                                                gettext("Unable to enable MPRIS support")
                                            )
                                        }
                                        player_maybe
                                    };
                                } else if let Some(ref player) = mpris_obj {
                                    if mpris_enabled != player.can_play() {
                                        player.set_can_play(mpris_enabled).await.ok();
                                    }
                                }
                            }
                        }
                        ErrorMessage(string) => {
                            if !(string == gettext("No match for this song")
                                && (microphone_switch.is_active() || loopback_switch.is_active()))
                            {
                                error!("Displaying error: {}", string);
                                let dialog = adw::AlertDialog::builder()
                                    .body(&string)
                                    .close_response("ok")
                                    .default_response("ok")
                                    .build();
                                dialog.add_responses(&[("ok", &gettext("_Ok"))]);
                                glib::spawn_future_local(dialog.choose_future(Some(&window)));

                                if string != gettext("No match for this song") {
                                    Self::notify_application_error(
                                        preferences_interface_ptr.clone(),
                                        &string,
                                        &application.clone(),
                                    );
                                }
                            }
                        }
                        RateLimitState(is_rate_limited) => {
                            if is_rate_limited && !rate_limited_message.is_visible() {
                                Self::notify_network_error(
                                    preferences_interface_ptr.clone(),
                                    &rate_limited_message.label(),
                                    &application.clone(),
                                    true,
                                );
                            }
                            rate_limited_message.set_visible(is_rate_limited);
                        }
                        NetworkStatus(network_is_reachable) => {
                            if !network_is_reachable && !no_network_message.is_visible() {
                                Self::notify_network_error(
                                    preferences_interface_ptr.clone(),
                                    &no_network_message.label(),
                                    &application.clone(),
                                    false,
                                );
                            }
                            no_network_message.set_visible(!network_is_reachable);

                            #[cfg(feature = "mpris")]
                            {
                                let mpris_enabled = preferences_interface_ptr
                                    .lock()
                                    .unwrap()
                                    .preferences
                                    .enable_mpris
                                    == Some(true);

                                if mpris_enabled {
                                    let mpris_status = if network_is_reachable {
                                        PlaybackStatus::Playing
                                    } else {
                                        PlaybackStatus::Paused
                                    };

                                    if let Some(ref player) = mpris_obj {
                                        if let Err(error) =
                                            player.set_playback_status(mpris_status).await
                                        {
                                            error!(
                                                "Could not set MPRIS playback status: {:?}",
                                                error
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        SongRecognized(message) => {
                            results_section.set_visible(true);

                            // https://gtk-rs.org/gtk4-rs/git/docs/gdk4/struct.Texture.html#method.from_bytes
                            // https://docs.gtk.org/gdk4/ctor.Texture.new_from_bytes.html
                            // The file format is detected automatically. The supported formats are PNG, JPEG and TIFF, though more formats might be available.

                            // + https://gtk-rs.org/gtk4-rs/git/docs/gtk4/struct.Image.html#method.set_paintable
                            // + https://docs.gtk.org/gtk4/method.Image.set_from_paintable.html

                            let song_name =
                                format!("{} - {}", message.artist_name, message.song_name);

                            if results_label.text().as_str() != &song_name {
                                results_label.set_label(&song_name);

                                let notification =
                                    gio::Notification::new(&gettext("Song recognized"));
                                notification.set_body(Some(&song_name));

                                if let Some(ref cover_image) = message.cover_image {
                                    if let Ok(texture) =
                                        gdk::Texture::from_bytes(&glib::Bytes::from(cover_image))
                                    {
                                        results_image.set_visible(true);
                                        results_image.set_paintable(Some(&texture));

                                        match message.album_name {
                                            Some(ref value) => {
                                                results_image.set_tooltip_text(Some(&value))
                                            }
                                            None => results_image.set_tooltip_text(None),
                                        };
                                        notification.set_icon(&texture);
                                    } else {
                                        results_image.set_visible(false);
                                    }
                                } else {
                                    results_image.set_visible(false);
                                }

                                #[cfg(feature = "mpris")]
                                if preferences_interface_ptr
                                    .lock()
                                    .unwrap()
                                    .preferences
                                    .enable_mpris
                                    == Some(true)
                                {
                                    if let Some(ref player) = mpris_obj {
                                        update_song(player, &message, &mut last_cover_path).await;
                                    }
                                }

                                if preferences_interface_ptr
                                    .lock()
                                    .unwrap()
                                    .preferences
                                    .enable_notifications
                                    == Some(true)
                                {
                                    application
                                        .send_notification(Some("recognized-song"), &notification);
                                }

                                let new_entry = SongHistoryRecord {
                                    song_name: song_name,
                                    album: Some(
                                        message
                                            .album_name
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    track_key: Some(message.track_key),
                                    release_year: Some(
                                        message
                                            .release_year
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    genre: Some(
                                        message
                                            .genre
                                            .as_ref()
                                            .unwrap_or(&"".to_string())
                                            .to_string(),
                                    ),
                                    recognition_date: Local::now().format("%c").to_string(),
                                };

                                if preferences_interface_ptr
                                    .lock()
                                    .unwrap()
                                    .preferences
                                    .no_duplicates
                                    == Some(true)
                                {
                                    song_history_interface
                                        .borrow_mut()
                                        .remove(new_entry.clone());
                                }
                                song_history_interface
                                    .borrow_mut()
                                    .add_row_and_save(new_entry);
                            }
                        }
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

                            for device in devices.iter() {
                                // device: thread_messages::DeviceListItem
                                let listed_device = ListedDevice::new(
                                    device.display_name.clone(),
                                    device.inner_name.clone(),
                                    device.is_monitor,
                                );
                                g_list_store.append(&listed_device);

                                if old_device_name == Some(device.inner_name.to_string()) {
                                    initial_device_index = current_index;
                                    initial_device = Some(listed_device);
                                } else if old_device_name == None
                                    && device.is_monitor
                                    && !found_monitor_device
                                {
                                    initial_device_index = current_index;
                                    initial_device = Some(listed_device);
                                } else if current_index == 0 {
                                    initial_device = Some(listed_device);
                                }
                                current_index += 1;

                                if device.is_monitor {
                                    found_monitor_device = true;
                                }
                            }

                            if let Some(device) = initial_device {
                                // device: ListedDevice
                                adw_combo_row.set_selected(initial_device_index);
                                loopback_switch.set_visible(found_monitor_device);

                                debug!(
                                    "Initally selected audio input device: {:?} / {:?}",
                                    device.inner_name(),
                                    device.display_name()
                                );

                                microphone_switch.set_visible(true);
                                volume_row.set_visible(true);

                                // Will trigger the "input_device_switched" callback
                            }
                        }
                        MicrophoneRecording => {}

                        MicrophoneVolumePercent(percent) => {
                            volume_gauge.set_fraction((percent / 100.0) as f64);
                        }

                        WipeSongHistory => {
                            let dialog = adw::AlertDialog::builder()
                                .body(&gettext("Are you sure you want to wipe history?"))
                                .default_response("yes")
                                .close_response("no")
                                .build();

                            dialog.add_responses(&[
                                ("yes", &gettext("_Yes")),
                                ("no", &gettext("_No")),
                            ]);

                            let song_history_interface = song_history_interface.clone();
                            dialog.choose(
                                Some(&window),
                                None::<&gio::Cancellable>,
                                move |result| {
                                    if result == "yes" {
                                        song_history_interface.borrow_mut().wipe_and_save();
                                    }
                                },
                            );
                        }

                        ShowWindow => {
                            window.present();
                        }

                        QuitApplication => {
                            application.quit();
                        }

                        _ => {
                            debug!("(parsing unimplemented yet): {:?}", gui_message);
                        }
                    }

                    // Possibly handle missing messages here
                }
            }
        });
    }

    fn setup_actions(&self, application: &adw::Application, enable_mpris_cli: bool) {
        let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
        let file_picker: gtk::FileDialog = self.builder.object("file_picker").unwrap();
        let shortcuts_dialog: gtk::ShortcutsWindow =
            self.builder.object("shortcuts_window").unwrap();
        let about_dialog: adw::AboutDialog = self.builder.object("about_dialog").unwrap();
        let results_label: gtk::Label = self.builder.object("results_label").unwrap();
        let menu_button: gtk::MenuButton = self.builder.object("menu_button").unwrap();
        let navigation_view: adw::NavigationView =
            self.builder.object("main_window_pages").unwrap();
        let recognize_file_row: adw::PreferencesRow =
            self.builder.object("recognize_file_row").unwrap();
        let spinner_row: adw::PreferencesRow = self.builder.object("spinner_row").unwrap();

        let ctx_buffered_log = self.ctx_buffered_log.clone();
        let ctx_logger_source_id = self.ctx_logger_source_id.clone();

        let action_show_about = gio::ActionEntry::builder("show-about")
            .activate(move |window, _, _| {
                about_dialog.set_visible(true);
                about_dialog.set_version(env!("CARGO_PKG_VERSION"));
                about_dialog.present(Some(window));

                about_dialog.set_debug_info(&*ctx_buffered_log.borrow());

                // Sync the debug info with the About modal at most every
                // 1 sec as it may require a lot of text rendering power
                // each time

                let ctx_buffered_log = ctx_buffered_log.clone();
                let ctx_logger_source_id_2 = ctx_logger_source_id.clone();
                let about_dialog = about_dialog.clone();

                if *ctx_logger_source_id.borrow() == None {
                    *ctx_logger_source_id.borrow_mut() =
                        Some(glib::source::timeout_add_seconds_local(1, move || {
                            if about_dialog.is_visible() {
                                about_dialog.set_debug_info(&*ctx_buffered_log.borrow());
                                glib::ControlFlow::Continue
                            } else {
                                *ctx_logger_source_id_2.borrow_mut() = None;
                                glib::ControlFlow::Break
                            }
                        }));
                }
            })
            .build();

        let processing_tx = self.processing_tx.clone();

        let action_recognize_file = gio::ActionEntry::builder("recognize-file")
            .activate(move |window, _action, _obj| {
                // Call a XDG file picker here

                let processing_tx = processing_tx.clone();

                let window: &adw::ApplicationWindow = window;
                let recognize_file_row = recognize_file_row.clone();
                let spinner_row = spinner_row.clone();

                file_picker.open(
                    Some(window),
                    None::<&gio::Cancellable>,
                    move |file| match file {
                        Ok(gio_file) => {
                            info!("Picked file: {:?}", gio_file.path());
                            let path_str = gio_file.path().unwrap().to_string_lossy().into_owned();

                            recognize_file_row.set_sensitive(false);
                            spinner_row.set_visible(true);

                            processing_tx
                                .try_send(ProcessingMessage::ProcessAudioFile(path_str))
                                .unwrap();
                        }
                        Err(error) => {
                            error!("Error picking file: {:?}", error);
                        }
                    },
                );
            })
            .build();

        let action_search_youtube = gio::ActionEntry::builder("search-youtube")
            .activate(move |window: &adw::ApplicationWindow, _, _| {
                let window = window.clone();

                let results_label = results_label.text();

                let mut encoded_search_term =
                    utf8_percent_encode(results_label.as_str(), NON_ALPHANUMERIC).to_string();
                encoded_search_term = encoded_search_term.replace("%20", "+");

                let search_url = format!(
                    "https://www.youtube.com/results?search_query={}",
                    encoded_search_term
                );

                glib::spawn_future_local(async move {
                    info!("Launching URL: {}", search_url);
                    if let Err(err) = gtk::UriLauncher::new(&search_url)
                        .launch_future(Some(&window))
                        .await
                    {
                        error!("Could not launch URL {}: {:?}", search_url, err);
                    }
                });
            })
            .build();

        let action_export_to_csv = gio::ActionEntry::builder("export-to-csv")
            .activate(move |window: &adw::ApplicationWindow, _action, _obj| {
                #[cfg(not(windows))]
                {
                    let window = window.clone();

                    glib::spawn_future_local(async move {
                        let launch_path = obtain_recognition_history_csv_path().unwrap();
                        info!("Launching file: {}", launch_path);
                        let launch_file = gio::File::for_path(launch_path.clone());
                        if let Err(err) = gtk::FileLauncher::new(Some(&launch_file))
                            .launch_future(Some(&window))
                            .await
                        {
                            error!("Could not launch file {}: {:?}", launch_path, err);
                        }
                    });
                }

                #[cfg(windows)]
                std::process::Command::new("cmd")
                    .args(&[
                        "/c",
                        &format!("start {}", obtain_recognition_history_csv_path().unwrap()),
                    ])
                    .creation_flags(0x00000008) // Set "CREATE_NO_WINDOW" on Windows
                    .output()
                    .ok();
            })
            .build();

        let action_export_favorites_to_csv = gio::ActionEntry::builder("export-favorites-to-csv")
            .activate(move |window: &adw::ApplicationWindow, _action, _obj| {
                #[cfg(not(windows))]
                {
                    let window = window.clone();

                    glib::spawn_future_local(async move {
                        let launch_path = obtain_favorites_csv_path().unwrap();
                        info!("Launching file: {}", launch_path);
                        let launch_file = gio::File::for_path(launch_path.clone());
                        if let Err(err) = gtk::FileLauncher::new(Some(&launch_file))
                            .launch_future(Some(&window))
                            .await
                        {
                            error!("Could not launch file {}: {:?}", launch_path, err);
                        }
                    });
                }

                #[cfg(windows)]
                std::process::Command::new("cmd")
                    .args(&[
                        "/c",
                        &format!("start {}", obtain_favorites_csv_path().unwrap()),
                    ])
                    .creation_flags(0x00000008) // Set "CREATE_NO_WINDOW" on Windows
                    .output()
                    .ok();
            })
            .build();

        let gui_tx = self.gui_tx.clone();

        let action_wipe_history = gio::ActionEntry::builder("wipe-history")
            .activate(move |_window, _action, _obj| {
                gui_tx.try_send(GUIMessage::WipeSongHistory).unwrap();
            })
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
                gui_tx
                    .try_send(GUIMessage::UpdatePreference(new_preference))
                    .unwrap();
            })
            .build();

        let gui_tx = self.gui_tx.clone();

        let action_notification_setting = gio::ActionEntry::builder("notification-setting")
            .state(
                self.old_preferences
                    .enable_notifications
                    .unwrap()
                    .to_variant(),
            )
            .activate(move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.enable_notifications = Some(new_state);
                gui_tx
                    .try_send(GUIMessage::UpdatePreference(new_preference))
                    .unwrap();
            })
            .build();

        let gui_tx = self.gui_tx.clone();
        #[cfg(target_os = "linux")]
        let ctx_systray_handle = self.ctx_systray_handle.clone();

        #[cfg(target_os = "linux")]
        let action_systray_setting = gio::ActionEntry::builder("systray-setting")
            .state(self.old_preferences.enable_systray.unwrap().to_variant())
            .activate(
                move |window: &adw::ApplicationWindow, action: &gio::SimpleAction, _| {
                    let state = action.state().unwrap();
                    let action_state: bool = state.get().unwrap();
                    let new_state = !action_state; // toggle
                    action.set_state(&new_state.to_variant());

                    let ctx_systray_handle = ctx_systray_handle.clone();

                    if new_state {
                        Self::setup_systray(ctx_systray_handle, window.clone(), gui_tx.clone());
                    } else {
                        Self::unsetup_systray(ctx_systray_handle, window.clone());
                    }

                    let mut new_preference: Preferences = Preferences::new();
                    new_preference.enable_systray = Some(new_state);
                    gui_tx
                        .try_send(GUIMessage::UpdatePreference(new_preference))
                        .unwrap();
                },
            )
            .build();

        let gui_tx = self.gui_tx.clone();

        let action_no_dupes_setting = gio::ActionEntry::builder("no-dupes-setting")
            .state(self.old_preferences.no_duplicates.unwrap().to_variant())
            .activate(move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.no_duplicates = Some(new_state);
                gui_tx
                    .try_send(GUIMessage::UpdatePreference(new_preference))
                    .unwrap();
            })
            .build();

        let action_close = gio::ActionEntry::builder("close")
            .activate(move |window: &adw::ApplicationWindow, _, _| {
                window.close();
            })
            .build();

        let action_display_shortcuts = gio::ActionEntry::builder("display-shortcuts")
            .activate(move |_, _, _| {
                shortcuts_dialog.present();
            })
            .build();

        let action_show_preferences = gio::ActionEntry::builder("show-preferences")
            .activate(move |_, _, _| {
                navigation_view.push_by_tag("settings_tag");
            })
            .build();

        let microphone_tx = self.microphone_tx.clone();

        let action_refresh_devices = gio::ActionEntry::builder("refresh-devices")
            .activate(move |_, _, _| {
                microphone_tx
                    .try_send(MicrophoneMessage::RefreshDevices)
                    .unwrap();
            })
            .build();

        let action_show_menu = gio::ActionEntry::builder("show-menu")
            .activate(move |_, _, _| {
                menu_button.activate();
            })
            .build();

        window.add_action_entries([
            action_show_about,
            action_recognize_file,
            action_search_youtube,
            action_export_to_csv,
            action_export_favorites_to_csv,
            action_wipe_history,
            action_display_shortcuts,
            action_show_preferences,
            action_notification_setting,
            #[cfg(target_os = "linux")]
            action_systray_setting,
            action_no_dupes_setting,
            action_refresh_devices,
            action_close,
            action_show_menu,
        ]);

        #[cfg(feature = "mpris")]
        if enable_mpris_cli {
            window.add_action_entries([action_mpris_setting]);
        }

        // GDK key names are available here:
        // https://gitlab.gnome.org/GNOME/gtk/-/blob/main/gdk/gdkkeysyms.h

        application.set_accels_for_action("win.close", &["<Primary>Q", "<Primary>W"]);
        application.set_accels_for_action("win.recognize-file", &["<Primary>O"]);
        application.set_accels_for_action("win.display-shortcuts", &["<Primary>question"]);
        application
            .set_accels_for_action("win.show-preferences", &["<Primary>comma", "<Primary>P"]);
        application.set_accels_for_action("win.show-menu", &["F10"]);
    }

    fn show_window(&self, application: &adw::Application) {
        let window: adw::ApplicationWindow = self.builder.object("main_window").unwrap();
        window.set_application(Some(application));

        window.present();
    }
}
