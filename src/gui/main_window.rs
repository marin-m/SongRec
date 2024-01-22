use gdk::EventButton;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::ResponseType;
use gettextrs::gettext;
use gdk_pixbuf::Pixbuf;
use std::collections::HashSet;
use std::error::Error;

use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
use chrono::Local;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
#[cfg(feature = "mpris")]
use mpris_player::PlaybackStatus;

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::thread_messages::{*, GUIMessage::*};

use crate::gui::song_history_interface::FavoritesInterface;
use crate::gui::song_history_interface::{SongRecordInterface, RecognitionHistoryInterface};
use crate::utils::csv_song_history::{IsSong, Song};
use crate::utils::filesystem_operations::obtain_favorites_csv_path;
#[cfg(feature = "mpris")]
use crate::utils::mpris_player::{get_player, update_song};

use crate::gui::preferences::{PreferencesInterface, Preferences};
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::utils::filesystem_operations::obtain_recognition_history_csv_path;


#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::fingerprinting::signature_format::DecodedSignature;

pub fn gui_main(recording: bool, input_file: Option<&str>, enable_mpris_cli: bool) -> Result<(), Box<dyn Error>> {
    
    let application = gtk::Application::new(Some("com.github.marinm.songrec"),
        gio::ApplicationFlags::HANDLES_OPEN)
        .expect(&gettext("Application::new failed"));

    application.connect_startup(move |application| {
        
        let interface_src = include_str!("interface.glade");
        let main_builder = gtk::Builder::from_string(interface_src);
        
        let favorites_interface_src = include_str!("favorites_interface.glade");
        let favorites_builder = gtk::Builder::from_string(favorites_interface_src);

        // We create the main window.
    
        let main_window: gtk::ApplicationWindow = main_builder.get_object("window").unwrap();

        let prefs_menu_item: gtk::ModelButton = main_builder.get_object("preferences_menu_button").unwrap();
        let main_menu_separator: gtk::Separator = main_builder.get_object("main_menu_separator").unwrap();
        let prefs_window: gtk::Window = main_builder.get_object("preferences_window").unwrap();
        let _enable_mpris_box: gtk::CheckButton = main_builder.get_object("enable_mpris_box").unwrap();

        #[cfg(not(feature = "mpris"))]
        {
            prefs_menu_item.hide();
            main_menu_separator.hide();
            _enable_mpris_box.hide();
        }

        if !enable_mpris_cli {
            prefs_menu_item.hide();
            main_menu_separator.hide();
            _enable_mpris_box.hide();
        }

        prefs_window.connect_delete_event(move |item, _event| {
            item.hide_on_delete()
        });
        prefs_menu_item.connect_clicked(move |_menu_item: &gtk::ModelButton| {
            prefs_window.show_all();
        });

        let about_menu_item: gtk::ModelButton = main_builder.get_object("about_menu_button").unwrap();
        let about_dialog: gtk::AboutDialog = main_builder.get_object("about_dialog").unwrap();

        about_dialog.connect_delete_event(move |item, _event| {
            item.hide_on_delete()
        });
        about_menu_item.connect_clicked(move |_menu_item: &gtk::ModelButton| {
            about_dialog.show_all();
        });

        let favorites_window: gtk::Window = favorites_builder.get_object("favorites_window").unwrap();
        
        main_window.set_application(Some(application));

        // We spawn required background threads, and create the
        // associated communication channels.
        

        // We use the GLib communication channel in order for
        // communication with the main GTK+ loop and the standard
        // Rust channels for other threads. An alternative would be
        // to use async-based stuff with external crates.
        
        let (gui_tx, gui_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let (microphone_tx, microphone_rx) = mpsc::channel();
        let (processing_tx, processing_rx) = mpsc::channel();
        let (http_tx, http_rx) = mpsc::channel();
        
        let microphone_tx_2 = microphone_tx.clone();
        let microphone_tx_3 = microphone_tx.clone();
        let microphone_tx_4 = microphone_tx.clone();
        let microphone_tx_5 = microphone_tx.clone();
        let processing_tx_2 = processing_tx.clone();
        let processing_tx_4 = processing_tx.clone();
        
        spawn_big_thread(clone!(@strong gui_tx => move || { // microphone_rx, processing_tx
            microphone_thread(microphone_rx, processing_tx_2, gui_tx);
        }));
        
        spawn_big_thread(clone!(@strong gui_tx => move || { // processing_rx, http_tx
            processing_thread(processing_rx, http_tx, gui_tx);
        }));
        
        spawn_big_thread(clone!(@strong gui_tx => move || { // http_rx
            http_thread(http_rx, gui_tx, microphone_tx_3);
        }));

        // We create a callback for handling files to recognize opened
        // from the command line or through "xdg-open".
        
        application.connect_open(move |_application, files, _hint| {
            if files.len() >= 1 {
                if let Some(file_path) = files[0].get_path() {
                    let file_path_string = file_path.into_os_string().into_string().unwrap();
                    
                    processing_tx_4.send(ProcessingMessage::ProcessAudioFile(file_path_string)).unwrap();
                }
            }
        });
        
        // Load files

        let mut preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();

        if let Some(old_enable_mpris) = old_preferences.enable_mpris {
            _enable_mpris_box.set_active(old_enable_mpris);
        }

        // We initialize the CSV file that will contain song history.
        let history_list_store = main_builder.get_object("history_list_store").unwrap();
        let mut song_history_interface = RecognitionHistoryInterface::new(history_list_store, obtain_recognition_history_csv_path).unwrap();
        let history_tree_view: gtk::TreeView = main_builder.get_object("history_tree_view").unwrap();

        let favorites_list_store = favorites_builder.get_object("favorites_list_store").unwrap();
        let mut favorites_interface = FavoritesInterface::new(favorites_list_store, obtain_favorites_csv_path).unwrap();
        let favorites_tree_view: gtk::TreeView = favorites_builder.get_object("favorites_tree_view").unwrap();

        // Add a context menu to the history tree view, in order to allow
        // users to copy or search items (see https://stackoverflow.com/a/49720383)
        // add and remove favorites
        let history_context_menu: gtk::Menu = main_builder.get_object("list_view_context_menu").unwrap();
        history_tree_view.connect_right_click(&history_context_menu, &favorites_interface);

        let favorites_context_menu: gtk::Menu = favorites_builder.get_object("list_view_context_menu").unwrap();
        favorites_tree_view.connect_right_click(&favorites_context_menu, &favorites_interface);

        trait ContextMenu {
            fn connect_activate_menu_item<F: Fn(&gtk::MenuItem) + 'static>(&self, name: &str, f: F) -> ();
            fn get_menu_item(&self, name: &str) -> Option<gtk::MenuItem>;
            fn show_menu_item(&self, name: &str) -> Option<()>;
            fn hide_menu_item(&self, name: &str) -> Option<()>;
        }

        impl ContextMenu for gtk::Menu {
            fn get_menu_item(&self, name: &str) -> Option<gtk::MenuItem> {
                for child in self.get_children() {
                    if let Ok(menu_item) = child.downcast::<gtk::MenuItem>() {
                        if menu_item.get_buildable_name().unwrap() == name {
                            return Some(menu_item);
                        }
                    }
                }
                return None;
            }

            fn connect_activate_menu_item<F: Fn(&gtk::MenuItem) + 'static>(&self, name: &str, f: F) -> () {
                if let Some(menu_item) = self.get_menu_item(name) {
                    menu_item.connect_activate(f);
                }
            }

            fn show_menu_item(&self, name: &str) -> Option<()> {
                if let Some(menu_item) = self.get_menu_item(name) {
                    menu_item.set_visible(true);
                    return Some(());
                }
                None
            }

            fn hide_menu_item(&self, name: &str) -> Option<()> {
                if let Some(menu_item) = self.get_menu_item(name) {
                    menu_item.set_visible(false);
                    return Some(());
                }
                None
            }
        }

        trait SongRecords {
            fn get_selected_song_record(&self) -> Option<SongHistoryRecord>;
            fn get_song_record_at_mouse(&self, mouse_button: &EventButton) -> Option<SongHistoryRecord>;
        }

        impl SongRecords for gtk::TreeView {
            fn get_selected_song_record(&self) -> Option<SongHistoryRecord> {
                if let Some((tree_model, tree_iter)) = &self.get_selection().get_selected() {
                    Some(SongHistoryRecord {
                        song_name: tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap(),
                        album: tree_model.get_value(&tree_iter, 1).get().unwrap(),
                        track_key: tree_model.get_value(&tree_iter, 3).get().unwrap(), 
                        release_year: tree_model.get_value(&tree_iter, 4).get().unwrap(), 
                        genre: tree_model.get_value(&tree_iter, 5).get().unwrap(),
                        recognition_date: tree_model.get_value(&tree_iter, 2).get().unwrap().unwrap(),
                    })
                } else {
                    None
                }
            }

            fn get_song_record_at_mouse(&self, mouse_button: &EventButton) -> Option<SongHistoryRecord> {
                let (x, y) = mouse_button.get_position();
                if let Some((Some(path), _, _, _)) = self.get_path_at_pos(x as i32, y as i32) {
                    let tree_model = self.get_model().unwrap();
                    if let Some(tree_iter) = tree_model.get_iter(&path) {
                        return Some(SongHistoryRecord {
                            song_name: tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap(),
                            album: tree_model.get_value(&tree_iter, 1).get().unwrap(),
                            track_key: tree_model.get_value(&tree_iter, 3).get().unwrap(), 
                            release_year: tree_model.get_value(&tree_iter, 4).get().unwrap(), 
                            genre: tree_model.get_value(&tree_iter, 5).get().unwrap(),
                            recognition_date: tree_model.get_value(&tree_iter, 2).get().unwrap().unwrap(),
                        });
                    }
                }
                None
            }
        }

        trait RightClick {
            fn connect_right_click(&self, builder: &gtk::Menu, favorites_interface: &FavoritesInterface);
        }

        impl RightClick for gtk::TreeView {
            fn connect_right_click(&self, context_menu: &gtk::Menu, favorites_interface: &FavoritesInterface) {
                self.connect_button_press_event(clone!(@strong context_menu, @strong favorites_interface => move |tree_view, button| {
                    if button.get_event_type() == gdk::EventType::ButtonPress && button.get_button() == 3 { // Is this a single right click?
                        // Display the context menu
            
                        // For usage examples, see:
                        // https://github.com/search?l=Rust&q=set_property_attach_widget&type=Code
                        context_menu.set_property_attach_widget(Some(tree_view));
                        let is_favorite: &HashSet<Song> = favorites_interface.get_is_favorite();
                        if let Some(song_record) = tree_view.get_song_record_at_mouse(button) {
                            if is_favorite.contains(&song_record.get_song()) {
                                context_menu.hide_menu_item("add_to_favorites");
                                context_menu.show_menu_item("remove_from_favorites");
                            } else {
                                context_menu.show_menu_item("add_to_favorites");
                                context_menu.hide_menu_item("remove_from_favorites");
                            }
                        }
                        context_menu.popup_at_pointer(Some(button));
                    }
                    
                    Inhibit(false) // Ensure that focus is given to the clicked item
                }));
            }
        }

        trait TreeView {
            fn get_tree_view(&self) -> gtk::TreeView;
        }

        impl TreeView for gtk::Menu {
            fn get_tree_view(&self) -> gtk::TreeView{
                let widget: gtk::Widget= self.get_attach_widget().unwrap();
                let tree_view: gtk::TreeView = widget.downcast::<gtk::TreeView>().unwrap();
                tree_view
            }
        }

        impl TreeView for gtk::MenuItem {
            fn get_tree_view(&self) -> gtk::TreeView {
                let widget: gtk::Widget = self.get_parent().unwrap();
                let menu: gtk::Menu = widget.downcast::<gtk::Menu>().unwrap();
                menu.get_tree_view()
            }
        }

        // See here for getting the selected menu item: https://stackoverflow.com/a/7938561
        
        // Bind the context menu actions for the recognized songs history

        let copy_artist_and_track_fn = move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&song_record.song_name);
            }
        };
        history_context_menu.connect_activate_menu_item("copy_artist_and_track", copy_artist_and_track_fn);
        favorites_context_menu.connect_activate_menu_item("copy_artist_and_track", copy_artist_and_track_fn);

        let copy_artist_fn = move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();

            if let Some(song_record) = tree_view.get_selected_song_record() {
                let full_song_name_parts: Vec<&str> = song_record.song_name.splitn(2, " - ").collect();
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(full_song_name_parts[0]);
            }

        };
        history_context_menu.connect_activate_menu_item("copy_artist", copy_artist_fn);
        favorites_context_menu.connect_activate_menu_item("copy_artist", copy_artist_fn);
        
        let copy_track_name_fn = move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                let full_song_name_parts: Vec<&str> = song_record.song_name.splitn(2, " - ").collect();
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(full_song_name_parts[1]);
            }
        };
        history_context_menu.connect_activate_menu_item("copy_track_name", copy_track_name_fn);
        favorites_context_menu.connect_activate_menu_item("copy_track_name", copy_track_name_fn);

        let copy_album_fn = move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&song_record.album.unwrap_or(String::new()));
            }
        };
        history_context_menu.connect_activate_menu_item("copy_album", copy_album_fn);
        favorites_context_menu.connect_activate_menu_item("copy_album", copy_album_fn);

        let search_on_youtube_fn = move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                
                let mut encoded_search_term = utf8_percent_encode(&song_record.song_name, NON_ALPHANUMERIC).to_string();
                encoded_search_term = encoded_search_term.replace("%20", "+");
                
                let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_search_term);
                
                gtk::show_uri(None, &search_url, gtk::get_current_event_time()).unwrap();
            }
            
        };
        history_context_menu.connect_activate_menu_item("search_on_youtube", search_on_youtube_fn);
        favorites_context_menu.connect_activate_menu_item("search_on_youtube", search_on_youtube_fn);

        let add_to_favorites_fn = clone!(@strong gui_tx => move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                gui_tx.send(GUIMessage::AddFavorite(song_record)).unwrap();
            }
        });
        history_context_menu.connect_activate_menu_item("add_to_favorites",add_to_favorites_fn);

        let remove_from_favorites_fn = clone!(@strong gui_tx => move |menu_item: &gtk::MenuItem| {
            let tree_view = menu_item.get_tree_view();
            if let Some(song_record) = tree_view.get_selected_song_record() {
                gui_tx.send(GUIMessage::RemoveFavorite(song_record)).unwrap();
            }
        });
        history_context_menu.connect_activate_menu_item("remove_from_favorites",remove_from_favorites_fn.clone());
        favorites_context_menu.connect_activate_menu_item("remove_from_favorites",remove_from_favorites_fn);

        favorites_builder.connect_signals(clone!(@strong favorites_window => move |_builder, handler_name| {
            match handler_name {
                "__hide_window" => Box::new(clone! (@strong favorites_window => move |_| {
                    favorites_window.hide();
                    Some(true.to_value())
                })),
                _ => Box::new(|_| {None})
            }
        }));

        // Obtain items from vertical box layout with a file picker button,
        // and places for song recognition information
        
        let recognize_file_button: gtk::Button = main_builder.get_object("recognize_file_button").unwrap();
        let spinner: gtk::Spinner = main_builder.get_object("spinner").unwrap();
        let network_unreachable: gtk::Label = main_builder.get_object("network_unreachable").unwrap();
        
        let results_frame: gtk::Frame = main_builder.get_object("results_frame").unwrap();
        
        let recognized_song_name: gtk::Label = main_builder.get_object("recognized_song_name").unwrap();
        let recognized_song_cover: gtk::Image = main_builder.get_object("recognized_song_cover").unwrap();
        let cover_image: Rc<RefCell<Option<Pixbuf>>> = Rc::new(RefCell::new(None));
        let cover_image2 = cover_image.clone();

        // Resize the cover image when its container is resized - Ensure responsiveness

        let cover = recognized_song_cover.clone();
        recognized_song_cover.get_parent().unwrap()
            .connect_size_allocate(move |_widget: &gtk::Widget, allocation| {
                // Return early if image surface has not been set
                
                let pixbuf = match cover_image2.try_borrow() {
                    Ok(x) => x,
                    _ => return,
                };
                
                let pixbuf = match *pixbuf {
                    Some(ref p) => p,
                    None => return,
                };

                let width = pixbuf.get_width() as f64;
                let max_width = allocation.width.min(400) as f64;
                let width_scale = width / max_width;
                let height = pixbuf.get_height() as f64;
                let max_height = allocation.height.min(400) as f64;
                let height_scale = height / max_height;

                let scale = width_scale.max(height_scale);
                
                let pixbuf = pixbuf.scale_simple((width / scale) as i32, (height / scale) as i32, gdk_pixbuf::InterpType::Bilinear);

                // Defer resizing until after size allocation is done

                let cover = cover.clone();
                glib::idle_add_local(move || {
                    cover.set_from_pixbuf(pixbuf.as_ref());
                    return glib::Continue(false);
                });
            }
        );

        let microphone_button: gtk::Button = main_builder.get_object("microphone_button").unwrap();
        let microphone_stop_button: gtk::Button = main_builder.get_object("microphone_stop_button").unwrap();

        let notification_enable_checkbox: gtk::CheckButton = main_builder.get_object("notification_enable_checkbox").unwrap();

        let youtube_button: gtk::Button = main_builder.get_object("youtube_button").unwrap();
        let lure_button: gtk::Button = main_builder.get_object("lure_button").unwrap();
        
        let wipe_history_button: gtk::Button = main_builder.get_object("wipe_history_button").unwrap();
        let export_history_csv_button: gtk::Button = main_builder.get_object("export_history_csv_button").unwrap();
        let favorites_button: gtk::Button = main_builder.get_object("favorites_list_button").unwrap();

        let export_favorites_csv_button: gtk::Button = favorites_builder.get_object("export_favorites_csv_button").unwrap();

        #[cfg(feature = "mpris")]
        let mut mpris_obj = {
            let player = if enable_mpris_cli && _enable_mpris_box.get_active() {
                get_player()
            } else {
                None
            };
            if enable_mpris_cli && _enable_mpris_box.get_active() && player.is_none() {
                println!("{}", gettext("Unable to enable MPRIS support"))
            }
            player
        };
        
        // Thread-local variables to be passed across callbacks.
        
        let youtube_query: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let youtube_query_2 = youtube_query.clone();
        
        let current_signature: Rc<RefCell<Option<DecodedSignature>>> = Rc::new(RefCell::new(None));
        let current_signature_2 = current_signature.clone();
        
        // Remember about the saved last-used microphone device, if any

        if let Some(old_enable_notifications) = old_preferences.enable_notifications {
            notification_enable_checkbox.set_active(old_enable_notifications);
        }
        let old_device_name = old_preferences.current_device_name;

        // Handle selecting a microphone input devices in the appropriate combo box
        // (the combo box will be filed with device names when a "DevicesList"
        // inter-thread message will be received at the initialization of the
        // microphone thread, because CPAL which underlies Rodio can't be called
        // from the same thread as the microphone thread under Windows, see:
        //  - https://github.com/RustAudio/rodio/issues/270
        //  - https://github.com/RustAudio/rodio/issues/214 )
        
        let combo_box: gtk::ComboBox = main_builder.get_object("microphone_source_select_box").unwrap();
        let combo_box_model: gtk::ListStore = main_builder.get_object("input_devices_list_store").unwrap();
        
        let recognize_from_my_speakers_checkbox: gtk::CheckButton = main_builder.get_object("recognize_from_my_speakers_checkbox").unwrap();
        
        let current_volume_hbox: gtk::Box = main_builder.get_object("current_volume_hbox").unwrap();
        let current_volume_bar: gtk::ProgressBar = main_builder.get_object("current_volume_bar").unwrap();
        
        combo_box.connect_changed(clone!(@strong microphone_button, @strong microphone_stop_button, @strong combo_box,
            @strong combo_box_model, @strong recognize_from_my_speakers_checkbox, @strong gui_tx => move |_| {
            
            if let Some(active_item) = combo_box.get_active_iter() {
                let device_name_str: String = combo_box_model.get_value(&active_item, 1).get().unwrap().unwrap();
                let is_monitor: bool = combo_box_model.get_value(&active_item, 2).get().unwrap().unwrap();

                // Save the selected microphone device name so that it is
                // remembered after relaunching the app
                
                let mut new_preference = Preferences::new();
                new_preference.current_device_name = Some(device_name_str.to_string());
                gui_tx.send(GUIMessage::UpdatePreference(new_preference)).unwrap();

                // Sync the monitor check box
                if recognize_from_my_speakers_checkbox.is_visible() {
                    recognize_from_my_speakers_checkbox.set_active(is_monitor);

                    if is_monitor {
                        microphone_button.set_label(gettext("Turn on speakers recognition").as_str());
                        microphone_stop_button.set_label(gettext("Turn off speakers recognition").as_str());
                    }
                    else {
                        microphone_button.set_label(gettext("Turn on microphone recognition").as_str());
                        microphone_stop_button.set_label(gettext("Turn off microphone recognition").as_str());
                    }
                }

                if microphone_stop_button.is_visible() {
                    
                    // Re-launch the microphone recording with the new selected
                    // device
                    
                    microphone_tx_4.send(MicrophoneMessage::MicrophoneRecordStop).unwrap();
                    microphone_tx_4.send(MicrophoneMessage::MicrophoneRecordStart(
                        device_name_str.to_string()
                    )).unwrap();
                    
                }
            }

        }));
        
        // Handle various controls
        
        let processing_tx_3 = processing_tx.clone();

        recognize_file_button.connect_clicked(clone!(@strong main_window, @strong spinner, @strong recognize_file_button => move |_| {
            
            let file_chooser = gtk::FileChooserNative::new(
                Some(&gettext("Select a file to recognize")),
                Some(&main_window),
                gtk::FileChooserAction::Open,
                Some(&gettext("_Open")),
                Some(&gettext("_Cancel"))
            );
            
            if file_chooser.run() == ResponseType::Accept {
                recognize_file_button.hide();
                
                spinner.show();
                
                let input_file_path = file_chooser.get_filename().expect(&gettext("Couldn't get filename"));
                let input_file_string = input_file_path.to_str().unwrap().to_string();
                
                processing_tx_3.send(ProcessingMessage::ProcessAudioFile(input_file_string)).unwrap();
            };
        
        }));
        
        microphone_button.connect_clicked(clone!(@strong microphone_button, @strong microphone_stop_button,
            @strong combo_box_model, @strong current_volume_hbox, @strong combo_box => move |_| {
            
            if let Some(active_item) = combo_box.get_active_iter() {
                let device_name: String = combo_box_model.get_value(&active_item, 1).get().unwrap().unwrap();
                microphone_tx.send(MicrophoneMessage::MicrophoneRecordStart(
                    device_name.to_owned()
                )).unwrap();
                
                microphone_stop_button.show();
                current_volume_hbox.show();
                microphone_button.hide();
            }

        }));
        
        microphone_stop_button.connect_clicked(clone!(@strong microphone_button, @strong microphone_stop_button, @strong current_volume_hbox => move |_| {
            
            microphone_tx_2.send(MicrophoneMessage::MicrophoneRecordStop).unwrap();
            
            microphone_stop_button.hide();
            current_volume_hbox.hide();
            microphone_button.show();
            
        }));
        
        recognize_from_my_speakers_checkbox.connect_toggled(clone!(@strong recognize_from_my_speakers_checkbox,
                @strong microphone_button, @strong microphone_stop_button,
                @strong combo_box_model, @strong combo_box => move |_| {

            if let Some(active_item) = combo_box.get_active_iter() {
                let is_currently_monitor: bool = combo_box_model.get_value(&active_item, 2).get().unwrap().unwrap();

                if is_currently_monitor {
                    microphone_button.set_label(gettext("Turn on speakers recognition").as_str());
                    microphone_stop_button.set_label(gettext("Turn off speakers recognition").as_str());
                }
                else {
                    microphone_button.set_label(gettext("Turn on microphone recognition").as_str());
                    microphone_stop_button.set_label(gettext("Turn off microphone recognition").as_str());
                }

                if is_currently_monitor != recognize_from_my_speakers_checkbox.get_active() {

                    if let Some(iter) = combo_box_model.get_iter_first() {
                        loop {
                            let is_other_monitor: bool = combo_box_model.get_value(&iter, 2).get().unwrap().unwrap();

                            if is_other_monitor == recognize_from_my_speakers_checkbox.get_active() {
                                combo_box.set_active_iter(Some(&iter));
                                break;
                            }
                            if !combo_box_model.iter_next(&iter) {
                                break;
                            }
                        }
                    }

                }
            }
        }));
        
        youtube_button.connect_clicked(move |_| {
            
            let youtube_query_borrow = youtube_query_2.borrow();
            
            let mut encoded_search_term: String = youtube_query_borrow.as_ref().unwrap().to_string();
            encoded_search_term = utf8_percent_encode(&encoded_search_term, NON_ALPHANUMERIC).to_string();
            encoded_search_term = encoded_search_term.replace("%20", "+");
            
            let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_search_term);
            
            gtk::show_uri(None, &search_url, gtk::get_current_event_time()).unwrap();
            
        });
        
        lure_button.connect_clicked(move |_| {
            
            let current_signature_borrow = current_signature_2.borrow();
    
            let mixed_source = rodio::buffer::SamplesBuffer::new::<Vec<i16>>(1, 16000, current_signature_borrow.as_ref().unwrap().to_lure().unwrap());
    
            std::thread::Builder::new().spawn(move || {
                        
                let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
                let sink = rodio::Sink::try_new(&handle).unwrap();
                    
                sink.append(mixed_source);
                
                sink.sleep_until_end();
                
            }).unwrap();

        
        });
        
        wipe_history_button.connect_clicked(clone!(@strong gui_tx => move |_| {

            gui_tx.send(GUIMessage::WipeSongHistory).unwrap();

        }));
        
        favorites_button.connect_clicked(clone!(@strong gui_tx => move |_| {

            gui_tx.send(GUIMessage::ShowFavorites).unwrap();

        }));

        export_favorites_csv_button.connect_clicked(move |_| {

            #[cfg(not(windows))] {

                gtk::show_uri(None, &format!("file://{}", obtain_favorites_csv_path().unwrap()), gtk::get_current_event_time()).ok();
            }

            #[cfg(windows)]
            std::process::Command::new("cmd")
                .args(&["/c", &format!("start {}", obtain_favorites_csv_path().unwrap())])
                .creation_flags(0x00000008) // Set "CREATE_NO_WINDOW" on Windows
                .output().ok();

        });
        
        export_history_csv_button.connect_clicked(move |_| {

            #[cfg(not(windows))] {

                gtk::show_uri(None, &format!("file://{}", obtain_recognition_history_csv_path().unwrap()), gtk::get_current_event_time()).ok();
            }

            #[cfg(windows)]
            std::process::Command::new("cmd")
                .args(&["/c", &format!("start {}", obtain_favorites_csv_path().unwrap())])
                .creation_flags(0x00000008) // Set "CREATE_NO_WINDOW" on Windows
                .output().ok();

        });

        _enable_mpris_box.connect_toggled(clone!(@strong _enable_mpris_box, @strong gui_tx => move |_| {
            let mut new_preference: Preferences = Preferences::new();
            new_preference.enable_mpris = Some(_enable_mpris_box.get_active());
            gui_tx.send(GUIMessage::UpdatePreference(new_preference)).unwrap();
        }));

        notification_enable_checkbox.connect_toggled(clone!(@strong notification_enable_checkbox, @strong gui_tx => move |_| {
            let mut new_preference: Preferences = Preferences::new();
            new_preference.enable_notifications = Some(notification_enable_checkbox.get_active());
            gui_tx.send(GUIMessage::UpdatePreference(new_preference)).unwrap();
        }));

        gui_rx.attach(None, clone!(@strong application, @strong main_window, @strong results_frame,
                @strong current_volume_hbox, @strong spinner, @strong recognize_file_button,
                @strong network_unreachable, @strong microphone_stop_button, @strong combo_box,
                @strong recognize_from_my_speakers_checkbox, @strong _enable_mpris_box,
                @strong notification_enable_checkbox, @strong favorites_window => move |gui_message| {
            
            match gui_message {
                ErrorMessage(_) | NetworkStatus(_) | SongRecognized(_) => {
                    recognize_file_button.show();
                    spinner.hide();
                },
                _ =>  { }
            }

            match gui_message {
                UpdatePreference(new_preference) => {
                    preferences_interface.update(new_preference);
                    #[cfg(feature = "mpris")]
                    if mpris_obj.is_none() {
                        mpris_obj = {
                            let player = if enable_mpris_cli && _enable_mpris_box.get_active() {
                                get_player()
                            } else {
                                None
                            };
                            if enable_mpris_cli && _enable_mpris_box.get_active() && player.is_none() {
                                println!("{}", gettext("Unable to enable MPRIS support"))
                            }
                            player
                        };
                    }  
                },
                AddFavorite(song_record) => {
                    favorites_interface.add_row_and_save(song_record);
                }
                RemoveFavorite(song_record) => {
                    favorites_interface.remove(song_record);
                },
                ShowFavorites => {
                    favorites_window.show_all();
                },
                ErrorMessage(string) => {
                    if !(string == gettext("No match for this song") && microphone_stop_button.is_visible()) {
                        let dialog = gtk::MessageDialog::new(Some(&main_window),
                            gtk::DialogFlags::MODAL, gtk::MessageType::Error, gtk::ButtonsType::Ok, &string);
                        dialog.connect_response(|dialog, _| dialog.close());
                        dialog.show_all();
                    }
                },
                NetworkStatus(network_is_reachable) => {
                    if network_is_reachable {
                        network_unreachable.hide();
                    }
                    else {
                        network_unreachable.show_all();
                    }
                    #[cfg(feature = "mpris")] if _enable_mpris_box.get_active() {
                        let mpris_status = if network_is_reachable { PlaybackStatus::Playing } else { PlaybackStatus::Paused };

                        mpris_obj.as_ref().map(|p| p.set_playback_status(mpris_status));
                    }
                }
                // This message is sent once in the program execution for
                // the moment (maybe it should be updated automatically
                // later?):
                DevicesList(devices) => {
                    let mut old_device_index: u32 = 0;
                    let mut has_monitor_device = false;
                    let mut current_index: u32 = 0;

                    // Fill in the list of available devices, and
                    // set back the old device if it was recorded
                    
                    for device in devices.iter() {
                        combo_box_model.set(&combo_box_model.append(), &[0, 1, 2], &[
                            &device.display_name, &device.inner_name,
                            &device.is_monitor]);
                        
                        if device.is_monitor {
                            has_monitor_device = true;
                        }
                        
                        if old_device_name == Some(device.inner_name.to_string()) {
                            old_device_index = current_index;
                        }
                        current_index += 1;
                    }
                    
                    combo_box.set_active(Some(old_device_index));

                    if has_monitor_device {
                        recognize_from_my_speakers_checkbox.show_all();
                        recognize_from_my_speakers_checkbox.set_active(
                            devices[old_device_index as usize].is_monitor
                        );

                        if devices[old_device_index as usize].is_monitor {
                            microphone_button.set_label(gettext("Turn on speakers recognition").as_str());
                            microphone_stop_button.set_label(gettext("Turn off speakers recognition").as_str());
                        }
                        else {
                            microphone_button.set_label(gettext("Turn on microphone recognition").as_str());
                            microphone_stop_button.set_label(gettext("Turn off microphone recognition").as_str());
                        }
                    }
                    else {
                        recognize_from_my_speakers_checkbox.hide();
                    }
                    
                    // Should we start recording yet? (will depend of the possible
                    // command line flags of the application)

                    if recording {
                    
                        if let Some(active_item) = combo_box.get_active_iter() {
                            let device_name: String = combo_box_model.get_value(&active_item, 1).get().unwrap().unwrap();

                            microphone_tx_5.send(MicrophoneMessage::MicrophoneRecordStart(
                                device_name.to_owned()
                            )).unwrap();
                            
                            microphone_stop_button.show();
                            current_volume_hbox.show();
                            microphone_button.hide();
                        }
                    }
                },
                WipeSongHistory => {
                    song_history_interface.wipe_and_save();
                },
                MicrophoneRecording => { },
                MicrophoneVolumePercent(percent) => {
                    current_volume_bar.set_fraction((percent / 100.0) as f64);
                },
                SongRecognized(message) => {
                    let mut youtube_query_borrow = youtube_query.borrow_mut();

                    let song_name = Some(format!("{} - {}", message.artist_name, message.song_name));
        
                    if *youtube_query_borrow != song_name { // If this is already the last recognized song, don't update the display (if for example we recognized a lure we played, it would update the proposed lure to a lesser quality)

                        #[cfg(feature = "mpris")]
                        mpris_obj.as_ref().map(|p| update_song(p, &message));

                        let notification = gio::Notification::new(&gettext("Song recognized"));
                        notification.set_body(Some(song_name.as_ref().unwrap()));

                        song_history_interface.add_row_and_save(SongHistoryRecord {
                            song_name: song_name.as_ref().unwrap().to_string(),
                            album: Some(message.album_name.as_ref().unwrap_or(&"".to_string()).to_string()),
                            track_key: Some(message.track_key),
                            release_year: Some(message.release_year.as_ref().unwrap_or(&"".to_string()).to_string()),
                            genre: Some(message.genre.as_ref().unwrap_or(&"".to_string()).to_string()),
                            recognition_date: Local::now().format("%c").to_string(),
                            
                        });

                        recognized_song_name.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(song_name.as_ref().unwrap())));
                        *youtube_query_borrow = song_name;
                        
                        let mut current_signature_borrow = current_signature.borrow_mut();
                        *current_signature_borrow = Some(*message.signature);
                        
                        results_frame.show_all();
                        
                        match message.cover_image {
                            Some(cover_data) => {
                                let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&cover_data));
                                
                                match Pixbuf::from_stream::<_, gio::Cancellable>(&stream, None) {
                                
                                    Ok(pixbuf) => {
                                        // Ensure that the window is large enough so that the cover is
                                        // displayed without being downsized, if the current setup
                                        // allows it
                                        
                                        // let (window_width, window_height) = window.get_size();
                                        
                                        // if window_height < 768 && !window.is_maximized() {
                                        //     window.resize(window_width, 768);
                                        // }

                                        notification.set_icon(&pixbuf);
                                        // Display the cover image
                                        cover_image.replace(Some(pixbuf));

                                        match message.album_name {
                                            Some(value) => { recognized_song_cover.set_tooltip_text(Some(&value)) },
                                            None => { recognized_song_cover.set_tooltip_text(None) }
                                        };
                                    },
                                    Err(_) => {
                                        recognized_song_cover.hide();
                                    }
                                    
                                };
                                    
                            }
                            None => {
                                recognized_song_cover.hide();
                            }
                        };

                        if microphone_stop_button.is_visible() && notification_enable_checkbox.get_active() {
                            application.send_notification(Some("recognized-song"), &notification);
                        }
                        
                    }
                }
                
            }
            
            Continue(true)
        }));

        // Don't forget to make all widgets visible.
        
        main_window.show_all();

        results_frame.hide();
        
        recognize_from_my_speakers_checkbox.hide(); // This will be available only of PulseAudio is up and controllable

        spinner.hide();
        network_unreachable.hide();

        microphone_stop_button.hide();
        current_volume_hbox.hide();

    });
    

    application.connect_activate(move |application| {
        let main_window = &application.get_windows()[0];

        // Raise the existing window to the top whenever a second
        // GUI instance is attempted to be launched
        main_window.present();
        
        //Close all windows when main window is closed
        main_window.connect_delete_event(|_, _| {
            for window in gtk::Window::list_toplevels() {
                if let Ok(window) = window.downcast::<gtk::Window>() {
                    window.close();
                }
            }
            gtk::Inhibit(false) // Do not inhibit the default delete event behavior
        });
    });
    
    if let Some(input_file_string) = input_file {
        application.run(&["songrec".to_string(), input_file_string.to_string()]);
    }
    else {
        application.run(&[]);
    }
    
    Ok(())
}
