use gio::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gdk::{Key, ModifierType, Rectangle};
use gtk::glib::clone;

use std::error::Error;
use std::rc::Rc;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::cell::RefCell;
use log::{error, info, debug, trace};

use crate::gui::song_history_interface::FavoritesInterface;

use crate::gui::song_history_interface::SongRecordInterface;
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::gui::history_entry::HistoryEntry;

pub struct ContextMenuUtil;

impl ContextMenuUtil {
    // XX WIP

    pub fn connect_menu(
        builder: gtk::Builder,
        column_view: gtk::ColumnView, popover_menu: gtk::PopoverMenu,
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>,
        favorites: RefCell<FavoritesInterface>
    ) {
        let selection: gtk::SingleSelection = column_view.model().unwrap()
            .downcast::<gtk::SingleSelection>().unwrap();

        // let interface = history_interface.clone();

        let click_handler = gtk::GestureClick::new();
        click_handler.set_button(3);
        click_handler.connect_released(clone!(#[weak] column_view, #[weak] popover_menu, #[weak] selection,
                move |_click_handler, _n, x, y| {
            // gesture.set_state(gtk::EventSequenceState::Claimed);
            // let cached_record = interface.borrow().get_hovered_record();
            info!("Selected item: {:?}", selection.selected_item());
            // info!("Selected item (cached): {:?}", cached_record);
            if let Some(record) = selection.selected_item() {
                let record = record.downcast::<HistoryEntry>().unwrap();

                *ctx_selected_item.borrow_mut() = Some(record.clone());

                let unfaved_model: gio::Menu = builder.object("history_context_model").unwrap();
                let faved_model: gio::Menu = builder.object("history_context_model_faved").unwrap();
                if favorites.borrow().is_favorite(record.get_song()) {
                    popover_menu.set_menu_model(Some(&faved_model));
                }
                else {
                    popover_menu.set_menu_model(Some(&unfaved_model));
                }

                popover_menu.unparent();
                popover_menu.set_parent(&column_view);
                popover_menu.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                popover_menu.popup();

            }
        }));
        column_view.add_controller(click_handler);

        // Call column_view.model().unwrap().unselect_all() when mouse hovers out of ColumnView

        // let interface = history_interface.clone();
        let hover_handler = gtk::EventControllerMotion::new();
        hover_handler.connect_leave(clone!(#[weak] column_view, #[weak] popover_menu, #[weak] selection, move |hover_handler| {
            selection.unselect_all();
        }));
        column_view.add_controller(hover_handler);

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
        ctx_selected_item: Rc<RefCell<Option<HistoryEntry>>>
    ) {
        let item = ctx_selected_item.clone();
        let action_search_youtube = gio::ActionEntry::builder("search-on-youtube")
            .activate(clone!(#[weak] window, move |_, _, _| {
            if let Some(entry) = &*item.borrow() {
                let results_label = entry.song_name();

                let mut encoded_search_term = utf8_percent_encode(results_label.as_str(), NON_ALPHANUMERIC).to_string();
                encoded_search_term = encoded_search_term.replace("%20", "+");
                
                let search_url = format!("https://www.youtube.com/results?search_query={}", encoded_search_term);

                glib::spawn_future_local(async move {
            
                    info!("Launching URL: {}", search_url);
                    if let Err(err) = gtk::UriLauncher::new(&search_url)
                        .launch_future(Some(&window)).await
                    {
                        error!("Could not launch URL {}: {:?}", search_url, err);
                    }
                });
            }
        }))
        .build();

        let actions = gio::SimpleActionGroup::new();
        actions.add_action_entries([action_search_youtube]);
        window.insert_action_group("history-menu", Some(&actions));
    }

    // See:
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L266 (right click)
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L349 (context menu key)
    // https://discourse.gnome.org/t/adding-a-context-menu-to-a-listview-using-gtk4-rs/19995/5
}