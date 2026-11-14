use gdk::{Key, ModifierType, Rectangle};
use gio::prelude::*;
use glib::Propagation;
use gtk::glib::clone;
use gtk::prelude::*;

use log::{debug, error, info};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use crate::gui::song_history_interface::FavoritesInterface;

use crate::core::preferences::PreferencesInterface;
use crate::gui::history_entry::HistoryEntry;
use crate::gui::song_history_interface::{RecognitionHistoryInterface, SongRecordInterface};

/// The URL searching SONG_NAME on the website set in the preferences, or on
/// YouTube when none is set. A search URL field that was emptied in the
/// preferences counts as unset, as appending the song to an empty string
/// gives a URL that can't be opened.
pub fn website_search_url_for(custom_search_url: Option<&str>, song_name: &str) -> String {
    let search_url = custom_search_url
        .filter(|url| !url.trim().is_empty())
        .unwrap_or("https://www.youtube.com/results?search_query=");
    let encoded_search_term = utf8_percent_encode(song_name, NON_ALPHANUMERIC)
        .to_string()
        .replace("%20", "+");
    format!("{}{}", search_url, encoded_search_term)
}

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
        touch_handler.set_button(gdk::BUTTON_PRIMARY);
        touch_handler.set_touch_only(true);
        touch_handler.connect_pressed(touch_closure.clone());
        label.add_controller(touch_handler);

        let click_handler = gtk::GestureClick::new();
        click_handler.set_button(gdk::BUTTON_SECONDARY);
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
                    if let Some(display) = gdk::Display::default()
                        && let Some(record) = selection.selected_item()
                    {
                        let record = record.downcast::<HistoryEntry>().unwrap();
                        display.clipboard().set(&record.song_name());
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
        popover_menu: gtk::PopoverMenu,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
        history_interface: Rc<RefCell<RecognitionHistoryInterface>>,
        favorites_interface: Rc<RefCell<FavoritesInterface>>,
        preferences_interface: Arc<Mutex<PreferencesInterface>>,
    ) {
        let item = ctx_selected_item.clone();
        let action_copy_artist_track = gio::ActionEntry::builder("copy-artist-track")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow()
                    && let Some(display) = gdk::Display::default()
                {
                    display.clipboard().set(&entry.song_name());
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_artist = gio::ActionEntry::builder("copy-artist")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow()
                    && let Some(display) = gdk::Display::default()
                {
                    let song_name = entry.song_name();
                    if let Some((ref artist, _track_name)) = song_name.split_once(" - ") {
                        display.clipboard().set(artist);
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_track = gio::ActionEntry::builder("copy-track-name")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow()
                    && let Some(display) = gdk::Display::default()
                {
                    let song_name = entry.song_name();
                    if let Some((_artist, ref track_name)) = song_name.split_once(" - ") {
                        display.clipboard().set(track_name);
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let action_copy_album = gio::ActionEntry::builder("copy-album")
            .activate(move |_, _, _| {
                if let Some(entry) = &*item.borrow()
                    && let Some(display) = gdk::Display::default()
                {
                    if let Some(album) = entry.album() {
                        display.clipboard().set(&album);
                    } else {
                        display.clipboard().set(&"");
                    }
                }
            })
            .build();

        let item = ctx_selected_item.clone();
        let preferences = preferences_interface.clone();
        let action_search_youtube = gio::ActionEntry::builder("search-on-youtube")
            .activate(clone!(
                #[weak]
                window,
                move |_, _, _| {
                    if let Some(entry) = &*item.borrow() {
                        let website_search_url = {
                            let lock = preferences.lock().unwrap();
                            lock.preferences.website_search_url.clone()
                        };
                        let search_url = website_search_url_for(
                            website_search_url.as_deref(),
                            &entry.song_name(),
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
        let popover = popover_menu.clone();
        let history = history_interface.clone();
        let action_remove_history = gio::ActionEntry::builder("remove-from-history")
            .activate(move |_, _, _| {
                popover.unparent();
                if let Some(entry) = &*item.borrow() {
                    history.borrow_mut().remove(entry.get_song_history_record());
                }
                *item.borrow_mut() = None;
            })
            .build();

        let item = ctx_selected_item.clone();
        let popover = popover_menu.clone();
        let favorites = favorites_interface.clone();
        let action_remove_favorites = gio::ActionEntry::builder("remove-from-favorites")
            .activate(move |_, _, _| {
                popover.unparent();
                if let Some(entry) = &*item.borrow() {
                    favorites
                        .borrow_mut()
                        .remove(entry.get_song_history_record());
                }
                *item.borrow_mut() = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_are_searched_on_youtube_unless_a_search_url_is_set() {
        let youtube = "https://www.youtube.com/results?search_query=Artist+%2D+Title%21";
        assert_eq!(website_search_url_for(None, "Artist - Title!"), youtube);
        assert_eq!(website_search_url_for(Some(""), "Artist - Title!"), youtube);
        assert_eq!(
            website_search_url_for(Some("  "), "Artist - Title!"),
            youtube
        );
        assert_eq!(
            website_search_url_for(Some("https://example.com/?q="), "Artist - Title!"),
            "https://example.com/?q=Artist+%2D+Title%21"
        );
    }
}
