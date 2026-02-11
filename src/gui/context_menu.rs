use gio::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gdk::{Key, ModifierType, Rectangle};
use gtk::glib::clone;

use std::error::Error;
use std::cell::RefCell;
use log::{error, info, debug, trace};

use crate::gui::song_history_interface::FavoritesInterface;

use crate::gui::song_history_interface::{SongRecordInterface, RecognitionHistoryInterface};
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::gui::history_entry::HistoryEntry;

pub struct ContextMenuUtil {
    last_selected_item: RefCell<Option<HistoryEntry>>
}

impl ContextMenuUtil {
    // XX WIP

    pub fn connect_menu(column_view: gtk::ColumnView, popover_menu: gtk::PopoverMenu) {
        let selection: gtk::SingleSelection = column_view.model().unwrap()
            .downcast::<gtk::SingleSelection>().unwrap();

        let click_handler = gtk::GestureClick::new();
        click_handler.set_button(3);
        click_handler.connect_released(clone!(#[weak] column_view, #[weak] popover_menu, #[weak] selection,
                move |_click_handler, _n, x, y| {
            // gesture.set_state(gtk::EventSequenceState::Claimed);
            info!("Selected: {:?}", selection.selected_item());
            popover_menu.unparent();
            popover_menu.set_parent(&column_view);
            popover_menu.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_menu.popup();
        }));
        column_view.add_controller(click_handler);

        // Call column_view.model().unwrap().unselect_all() when mouse hovers out of ColumnView

        let hover_handler = gtk::EventControllerMotion::new();
        hover_handler.connect_leave(clone!(#[weak] column_view, #[weak] popover_menu, #[weak] selection, move |hover_handler| {
            selection.unselect_all();
        }));
        column_view.add_controller(hover_handler);

        /*selection.connect_selection_changed(move |selection, _, _| {

        });*/
    }

    // See:
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L266 (right click)
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L349 (context menu key)
    // https://discourse.gnome.org/t/adding-a-context-menu-to-a-listview-using-gtk4-rs/19995/5
}