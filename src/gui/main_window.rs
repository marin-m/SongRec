use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::ResponseType;
use gettextrs::gettext;
use gdk_pixbuf::Pixbuf;
use std::error::Error;
use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
use chrono::Local;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use mpris_player::PlaybackStatus;

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::thread_messages::{*, GUIMessage::*};

use crate::utils::thread::spawn_big_thread;
use crate::utils::pulseaudio_loopback::PulseaudioLoopback;
use crate::utils::mpris_player::{get_player, update_song};

use crate::gui::song_history_interface::SongHistoryInterface;
use crate::gui::preferences::{PreferencesInterface, Preferences};
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::utils::filesystem_operations::obtain_song_history_csv_path;


#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::fingerprinting::signature_format::DecodedSignature;

pub fn gui_main(recording: bool, input_file: Option<&str>, enable_mpris: bool) -> Result<(), Box<dyn Error>> {
    
    let application = gtk::Application::new(Some("com.github.marinm.songrec"),
        gio::ApplicationFlags::HANDLES_OPEN)
        .expect(&gettext("Application::new failed"));

    application.connect_startup(move |application| {
        
        let glade_src = include_str!("interface.glade");
        let builder = gtk::Builder::from_string(glade_src);
        
        // We create the main window.
    
        let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
        
        window.set_application(Some(application));

        // We spawn required background threads, and create the
        // associated communication channels.
        let old_preferences: Preferences = PreferencesInterface::new().preferences;

        // Load preferences file.

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
        
        // We initialize the CSV file that will contain song history.

        let mut song_history_interface = SongHistoryInterface::new(builder.get_object("history_list_store").unwrap()).unwrap();
        let history_tree_view: gtk::TreeView = builder.get_object("history_tree_view").unwrap();
        
        // Add a context menu to the history tree view, in order to allow
        // users to copy or search items (see https://stackoverflow.com/a/49720383)
        
        let list_view_context_menu: gtk::Menu = builder.get_object("list_view_context_menu").unwrap();
        
        history_tree_view.connect_button_press_event(clone!(@strong list_view_context_menu, @strong history_tree_view => move |_, button| {
            
            if button.get_event_type() == gdk::EventType::ButtonPress && button.get_button() == 3 { // Is this a single right click?
                
                // Display the context menu
                
                // For usage examples, see:
                // https://github.com/search?l=Rust&q=set_property_attach_widget&type=Code
                
                list_view_context_menu.set_property_attach_widget(Some(&history_tree_view));
                
                list_view_context_menu.show_all();
                
                list_view_context_menu.popup_at_pointer(Some(button));
                
            }
            
            Inhibit(false) // Ensure that focus is given to the clicked item
            
        }));
        
        // See here for getting the selected menu item: https://stackoverflow.com/a/7938561
        
        // Bind the context menu actions for the recognized songs history

        let copy_artist_and_track: gtk::MenuItem = builder.get_object("copy_artist_and_track").unwrap();
        
        copy_artist_and_track.connect_activate(clone!(@strong history_tree_view => move |_| {
            
            if let Some((tree_model, tree_iter)) = history_tree_view.get_selection().get_selected() {
                let full_song_name: String = tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap();
                
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&full_song_name);
            }
            
        }));
        
        let copy_artist: gtk::MenuItem = builder.get_object("copy_artist").unwrap();
        
        copy_artist.connect_activate(clone!(@strong history_tree_view => move |_| {
            
            if let Some((tree_model, tree_iter)) = history_tree_view.get_selection().get_selected() {
                let full_song_name: String = tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap();
                
                let full_song_name_parts: Vec<&str> = full_song_name.splitn(2, " - ").collect();
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(full_song_name_parts[0]);
            }
            
        }));
        
        let copy_track_name: gtk::MenuItem = builder.get_object("copy_track_name").unwrap();
        
        copy_track_name.connect_activate(clone!(@strong history_tree_view => move |_| {
            
            if let Some((tree_model, tree_iter)) = history_tree_view.get_selection().get_selected() {
                let full_song_name: String = tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap();
                
                let full_song_name_parts: Vec<&str> = full_song_name.splitn(2, " - ").collect();
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(full_song_name_parts[1]);
            }
            
        }));
        
        let copy_album: gtk::MenuItem = builder.get_object("copy_album").unwrap();
        
        copy_album.connect_activate(clone!(@strong history_tree_view => move |_| {
            
            if let Some((tree_model, tree_iter)) = history_tree_view.get_selection().get_selected() {
                let album_name: String = tree_model.get_value(&tree_iter, 1).get().unwrap().unwrap();
                
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(&album_name);
            }
            
        }));
        
        let search_on_youtube: gtk::MenuItem = builder.get_object("search_on_youtube").unwrap();
        
        search_on_youtube.connect_activate(clone!(@strong history_tree_view => move |_| {
            
            if let Some((tree_model, tree_iter)) = history_tree_view.get_selection().get_selected() {
            
                let full_song_name: String = tree_model.get_value(&tree_iter, 0).get().unwrap().unwrap();
                
                let mut encoded_search_term = utf8_percent_encode(&full_song_name, NON_ALPHANUMERIC).to_string();
                encoded_search_term = encoded_search_term.replace("%20", "+");
                
                let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_search_term);
                
                gtk::show_uri(None, &search_url, gtk::get_current_event_time()).unwrap();
            }
            
        }));
        
        // Obtain items from vertical box layout with a file picker button,
        // and places for song recognition information
        
        let recognize_file_button: gtk::Button = builder.get_object("recognize_file_button").unwrap();
        let spinner: gtk::Spinner = builder.get_object("spinner").unwrap();
        let network_unreachable: gtk::Label = builder.get_object("network_unreachable").unwrap();
        
        let results_frame: gtk::Frame = builder.get_object("results_frame").unwrap();
        
        let recognized_song_name: gtk::Label = builder.get_object("recognized_song_name").unwrap();
        let recognized_song_cover: gtk::Image = builder.get_object("recognized_song_cover").unwrap();
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

        let microphone_button: gtk::Button = builder.get_object("microphone_button").unwrap();
        let microphone_stop_button: gtk::Button = builder.get_object("microphone_stop_button").unwrap();

        let notification_enable_checkbox: gtk::CheckButton = builder.get_object("notification_enable_checkbox").unwrap();

        let youtube_button: gtk::Button = builder.get_object("youtube_button").unwrap();
        let lure_button: gtk::Button = builder.get_object("lure_button").unwrap();
        
        let wipe_history_button: gtk::Button = builder.get_object("wipe_history_button").unwrap();
        let export_csv_button: gtk::Button = builder.get_object("export_csv_button").unwrap();

        let mpris_player = if enable_mpris { get_player() } else { None };
        if enable_mpris && mpris_player.is_none() {
            println!("{}", gettext("Unable to enable MPRIS support"))
        }
        
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
        
        let combo_box: gtk::ComboBox = builder.get_object("microphone_source_select_box").unwrap();
        let combo_box_model: gtk::ListStore = builder.get_object("input_devices_list_store").unwrap();
        
        let recognize_from_my_speakers_checkbox: gtk::CheckButton = builder.get_object("recognize_from_my_speakers_checkbox").unwrap();
        
        let current_volume_hbox: gtk::Box = builder.get_object("current_volume_hbox").unwrap();
        let current_volume_bar: gtk::ProgressBar = builder.get_object("current_volume_bar").unwrap();
        
        combo_box.connect_changed(clone!(@strong microphone_stop_button, @strong combo_box => move |_| {
            
            if let Some(device_name_str) = combo_box.get_active_id() {

                // Save the selected microphone device name so that it is
                // remembered after relaunching the app
                
                let mut preferences_interface = PreferencesInterface::new();
                let mut new_preferences = preferences_interface.preferences.clone();
                new_preferences.current_device_name = Some(device_name_str.to_string());
                preferences_interface.update(new_preferences);
                
                if microphone_stop_button.is_visible() {
                    
                    // Re-launch the microphone recording with the new selected
                    // device
                    
                    microphone_tx_4.send(MicrophoneMessage::MicrophoneRecordStop).unwrap();
                    microphone_tx_4.send(MicrophoneMessage::MicrophoneRecordStart(device_name_str.to_string())).unwrap();
                    
                }
            }

        }));
        
        // Handle various controls
        
        let processing_tx_3 = processing_tx.clone();

        recognize_file_button.connect_clicked(clone!(@strong window, @strong spinner, @strong recognize_file_button => move |_| {
            
            let file_chooser = gtk::FileChooserNative::new(
                Some(&gettext("Select a file to recognize")),
                Some(&window),
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
        
        microphone_button.connect_clicked(clone!(@strong microphone_button, @strong microphone_stop_button, @strong current_volume_hbox, @strong combo_box => move |_| {
            
            if let Some(device_name) = combo_box.get_active_id() {
                microphone_tx.send(MicrophoneMessage::MicrophoneRecordStart(device_name.to_owned())).unwrap();
                
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
        
        recognize_from_my_speakers_checkbox.connect_toggled(clone!(@strong recognize_from_my_speakers_checkbox => move |_| {
            PulseaudioLoopback::set_whether_audio_source_is_monitor(recognize_from_my_speakers_checkbox.get_active());
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
        
        export_csv_button.connect_clicked(move |_| {

            #[cfg(not(windows))] {

                gtk::show_uri(None, &format!("file://{}", obtain_song_history_csv_path().unwrap()), gtk::get_current_event_time()).ok();
            }

            #[cfg(windows)]
            std::process::Command::new("cmd")
                .args(&["/c", &format!("start {}", obtain_csv_path().unwrap())])
                .creation_flags(0x00000008) // Set "CREATE_NO_WINDOW" on Windows
                .output().ok();

        });
        
        notification_enable_checkbox.connect_toggled(clone!(@strong notification_enable_checkbox => move |_| {
            let mut preferences_interface = PreferencesInterface::new();
            let mut new_preferences = preferences_interface.preferences.clone();
            new_preferences.enable_notifications = Some(notification_enable_checkbox.get_active());
            preferences_interface.update(new_preferences);
        }));

        gui_rx.attach(None, clone!(@strong application, @strong window, @strong results_frame, @strong current_volume_hbox, @strong spinner, @strong recognize_file_button, @strong network_unreachable, @strong microphone_stop_button, @strong recognize_from_my_speakers_checkbox, @strong notification_enable_checkbox => move |gui_message| {
            
            match gui_message {
                ErrorMessage(_) | NetworkStatus(_) | SongRecognized(_) => {
                    recognize_file_button.show();
                    spinner.hide();
                },
                _ =>  { }
            }

            match gui_message {
                ErrorMessage(string) => {
                    if !(string == gettext("No match for this song") && microphone_stop_button.is_visible()) {
                        let dialog = gtk::MessageDialog::new(Some(&window),
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
                    let mpris_status = if network_is_reachable { PlaybackStatus::Playing } else { PlaybackStatus::Paused };

                    mpris_player.as_ref().map(|p| p.set_playback_status(mpris_status));
                }
                DevicesList(device_names) => {
                    let mut old_device_index = 0;
                    let mut current_index = 0;
                    
                    for device_name in device_names.iter() {
                        combo_box_model.set(&combo_box_model.append(), &[0], &[device_name]);
                        
                        if old_device_name == Some(device_name.to_string()) {
                            old_device_index = current_index;
                        }
                        current_index += 1;
                    }
                    
                    combo_box.set_active(Some(old_device_index));
                    
                    // Should we start recording yet? (will depend of the possible
                    // command line flags of the application)

                    if recording {
                    
                        if let Some(device_name) = combo_box.get_active_id() {
                            microphone_tx_5.send(MicrophoneMessage::MicrophoneRecordStart(device_name.to_owned())).unwrap();
                            
                            microphone_stop_button.show();
                            current_volume_hbox.show();
                            microphone_button.hide();
                        }
                    }
                },
                WipeSongHistory => {
                    song_history_interface.wipe_and_save();
                },
                MicrophoneVolumePercent(percent) => {
                    current_volume_bar.set_fraction((percent / 100.0) as f64);
                },
                MicrophoneRecording => {
                    
                    // Initally show the "Recognize from my speakers instead
                    // of microphone" checkbox if PulseAudio seems to be
                    // available, and we can see (supposedly) ourselves through
                    // PulseAudio

                    if PulseaudioLoopback::check_whether_pactl_is_available() {
                        
                        if PulseaudioLoopback::get_whether_audio_source_is_known() == Some(true) {
                            if let Some(audio_source_is_monitor) = PulseaudioLoopback::get_whether_audio_source_is_monitor() {
                                
                                recognize_from_my_speakers_checkbox.show_all();
                                
                                if audio_source_is_monitor {
                                    recognize_from_my_speakers_checkbox.set_active(true);
                                }
                            }
                        }
                    }
                },
                SongRecognized(message) => {
                    let mut youtube_query_borrow = youtube_query.borrow_mut();

                    let song_name = Some(format!("{} - {}", message.artist_name, message.song_name));
        
                    if *youtube_query_borrow != song_name { // If this is already the last recognized song, don't update the display (if for example we recognized a lure we played, it would update the proposed lure to a lesser quality)

                        mpris_player.as_ref().map(|p| update_song(p, &message));

                        let notification = gio::Notification::new(&gettext("Song recognized"));
                        notification.set_body(Some(song_name.as_ref().unwrap()));

                        song_history_interface.add_column_and_save(SongHistoryRecord {
                            song_name: song_name.as_ref().unwrap().to_string(),
                            album: message.album_name.as_ref().unwrap_or(&"".to_string()).to_string(),
                            recognition_date: Local::now().format("%c").to_string(),
                            track_key: message.track_key,
                            release_year: message.release_year.as_ref().unwrap_or(&"".to_string()).to_string(),
                            genre: message.genre.as_ref().unwrap_or(&"".to_string()).to_string(),
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
                                        
                                        let (window_width, window_height) = window.get_size();
                                        
                                        if window_height < 768 && !window.is_maximized() {
                                            window.resize(window_width, 768);
                                        }

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
        
        window.show_all();

        results_frame.hide();
        
        recognize_from_my_speakers_checkbox.hide(); // This will be available only of PulseAudio is up and controllable

        spinner.hide();
        network_unreachable.hide();

        microphone_stop_button.hide();
        current_volume_hbox.hide();
        
    });
    
    application.connect_activate(move |application| {
        // Raise the existing window to the top whenever a second
        // GUI instance is attempted to be launched
        
        application.get_windows()[0].present();
    });
    
    if let Some(input_file_string) = input_file {
        application.run(&["songrec".to_string(), input_file_string.to_string()]);
    }
    else {
        application.run(&[]);
    }
    
    Ok(())
}
