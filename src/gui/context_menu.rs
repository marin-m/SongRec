use gdk::{Key, ModifierType, Rectangle};
use gio::prelude::*;
use glib::Propagation;
use gtk::glib::clone;
use gtk::prelude::*;

use log::{debug, error, info};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::cell::RefCell;
use std::rc::Rc;

use crate::gui::song_history_interface::FavoritesInterface;

use crate::gui::history_entry::HistoryEntry;
use crate::gui::song_history_interface::{RecognitionHistoryInterface, SongRecordInterface};

pub struct ContextMenuUtil;

impl ContextMenuUtil {
    pub fn connect_menu_mouse_actions(
        builder: gtk::Builder,
        cell: gtk::ColumnViewCell,
        label: gtk::Label,
        popover_menu: gtk::PopoverMenu,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
        favorites: Rc<RefCell<FavoritesInterface>>,
    ) {
        let touch_closure = clone!(
            #[weak]
            cell,
            #[weak]
            label,
            #[weak]
            popover_menu,
            move |_: &gtk::GestureClick, _n_press, x, y| {
                let entry = cell.item();
                // gesture.set_state(gtk::EventSequenceState::Claimed);
                debug!("Selected item: {:?}", entry);
                if let Some(record) = entry {
                    let record = record.downcast::<HistoryEntry>().unwrap();
                    debug!("  => {}", record.song_name());

                    *ctx_selected_item.borrow_mut() = Some(record.clone());

                    let unfaved_model: gio::Menu = builder.object("history_context_model").unwrap();
                    let faved_model: gio::Menu =
                        builder.object("history_context_model_faved").unwrap();
                    if favorites.borrow().is_favorite(record.get_song()) {
                        popover_menu.set_menu_model(Some(&faved_model));
                    } else {
                        popover_menu.set_menu_model(Some(&unfaved_model));
                    }

                    popover_menu.unparent();
                    popover_menu.set_has_arrow(true);
                    popover_menu.set_parent(&label);
                    popover_menu.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                    popover_menu.popup();
                }
            }
        );

        let touch_handler = gtk::GestureClick::new();
        touch_handler.set_button(1);
        touch_handler.set_touch_only(true);
        touch_handler.connect_pressed(touch_closure.clone());
        label.add_controller(touch_handler);

        let click_handler = gtk::GestureClick::new();
        click_handler.set_button(3);
        click_handler.connect_pressed(touch_closure);
        label.add_controller(click_handler);
    }

    pub fn connect_menu_key_actions(
        builder: gtk::Builder,
        column_view: gtk::ColumnView,
        popover_menu: gtk::PopoverMenu,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
        favorites: Rc<RefCell<FavoritesInterface>>,
    ) {
        // WIP BIND THE CONTEXT KEY + CTRL+C CLOSURES

        let controller = gtk::EventControllerKey::new();

        let selection: gtk::SingleSelection = column_view
            .model()
            .unwrap()
            .downcast::<gtk::SingleSelection>()
            .unwrap();

        controller.connect_key_pressed(clone!(
            #[weak]
            column_view,
            #[weak]
            popover_menu,
            #[weak]
            selection,
            #[upgrade_or]
            Propagation::Proceed,
            move |_event, key_val, _key_code, modifier| {
                // gesture.set_state(gtk::EventSequenceState::Claimed);
                if key_val == Key::Menu {
                    if let Some(record) = selection.selected_item() {
                        let record = record.downcast::<HistoryEntry>().unwrap();

                        *ctx_selected_item.borrow_mut() = Some(record.clone());

                        let unfaved_model: gio::Menu =
                            builder.object("history_context_model").unwrap();
                        let faved_model: gio::Menu =
                            builder.object("history_context_model_faved").unwrap();
                        if favorites.borrow().is_favorite(record.get_song()) {
                            popover_menu.set_menu_model(Some(&faved_model));
                        } else {
                            popover_menu.set_menu_model(Some(&unfaved_model));
                        }

                        popover_menu.unparent();
                        popover_menu.set_has_arrow(false);
                        popover_menu.set_parent(&column_view);
                        popover_menu.set_pointing_to(Some(&Rectangle::new(
                            0, // popover_menu.size(gtk::Orientation::Horizontal) as i32,
                            0, 1, 1,
                        )));
                        popover_menu.popup();
                    }
                    Propagation::Stop
                } else if (key_val == Key::C || key_val == Key::c)
                    && (modifier.contains(ModifierType::CONTROL_MASK)
                        || modifier.contains(ModifierType::META_MASK))
                {
                    if let Some(display) = gdk::Display::default() {
                        if let Some(record) = selection.selected_item() {
                            let record = record.downcast::<HistoryEntry>().unwrap();
                            display.clipboard().set(&record.song_name());
                        }
                    }
                    Propagation::Stop
                } else {
                    Propagation::Proceed
                }
            }
        ));
        column_view.add_controller(controller);

        /* selection.connect_selection_changed(move |selection, _, _| {
            if let Some(item) = selection.selected_item() {
                history_interface.borrow_mut().set_hovered_record(
                    item.downcast::<HistoryEntry>().unwrap()
                );
            }
        }); */
    }

    pub fn bind_actions(
        window: adw::ApplicationWindow,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
        history_interface: Rc<RefCell<RecognitionHistoryInterface>>,
        favorites_interface: Rc<RefCell<FavoritesInterface>>,
    ) {
        let item = ctx_selected_item.clone();
        let action_copy_artist_track = gio::ActionEntry::builder("copy-artist-track")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set(&entry.song_name());
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_artist = gio::ActionEntry::builder("copy-artist")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    if let Some(display) = gdk::Display::default() {
                        let song_name = entry.song_name();
                        let full_song_name_parts: Vec<&str> = song_name.splitn(2, " - ").collect();
                        display.clipboard().set(&full_song_name_parts[0]);
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_track = gio::ActionEntry::builder("copy-track-name")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    if let Some(display) = gdk::Display::default() {
                        let song_name = entry.song_name();
                        let full_song_name_parts: Vec<&str> = song_name.splitn(2, " - ").collect();
                        display.clipboard().set(&full_song_name_parts[1]);
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_album = gio::ActionEntry::builder("copy-album")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    if let Some(display) = gdk::Display::default() {
                        if let Some(album) = entry.album() {
                            display.clipboard().set(&album);
                        } else {
                            display.clipboard().set(&"");
                        }
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_search_youtube = gio::ActionEntry::builder("search-on-youtube")
            .activate(clone!(
                #[weak]
                window,
                move |_, _, _| {
                    if let Some(entry) = &*item.borrow() {
                        let results_label = entry.song_name();

                        let mut encoded_search_term =
                            utf8_percent_encode(results_label.as_str(), NON_ALPHANUMERIC)
                                .to_string();
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
                    }
                }
            ))
            .build();

        let item = ctx_selected_item.clone();
        let favorites = favorites_interface.clone();
        let action_add_favorites = gio::ActionEntry::builder("add-to-favorites")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    favorites
                        .borrow_mut()
                        .add_row_and_save(entry.get_song_history_record());
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let history = history_interface.clone();
        let action_remove_history = gio::ActionEntry::builder("remove-from-history")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    history.borrow_mut().remove(entry.get_song_history_record());
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let favorites = favorites_interface.clone();
        let action_remove_favorites = gio::ActionEntry::builder("remove-from-favorites")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow() {
                    favorites
                        .borrow_mut()
                        .remove(entry.get_song_history_record());
                }
            })
            .build();

        let actions = gio::SimpleActionGroup::new();
        actions.add_action_entries([
            action_copy_artist_track,
            action_copy_artist,
            action_copy_track,
            action_copy_album,
            action_add_favorites,
            action_remove_history,
            action_remove_favorites,
            action_search_youtube,
        ]);
        window.insert_action_group("history-menu", Some(&actions));
    }

    // See:
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L266 (right click)
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L349 (context menu key)
    // https://discourse.gnome.org/t/adding-a-context-menu-to-a-listview-using-gtk4-rs/19995/5
}
