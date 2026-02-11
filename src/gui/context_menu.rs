use gio::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gdk::{Key, ModifierType, Rectangle};
use gtk::glib::clone;

use std::error::Error;
use log::{error, info, debug, trace};

use crate::gui::song_history_interface::FavoritesInterface;

use crate::gui::song_history_interface::{SongRecordInterface, RecognitionHistoryInterface};
use crate::utils::csv_song_history::SongHistoryRecord;
use crate::gui::history_entry::HistoryEntry;

pub struct ContextMenuUtil;

impl ContextMenuUtil {
    // XX WIP

    pub fn connect_menu(column_view: gtk::ColumnView, popover_menu: gtk::PopoverMenu) {
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(clone!(#[weak] column_view, #[weak] popover_menu, move |gesture, _n, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            popover_menu.unparent();
            popover_menu.set_parent(&column_view);
            popover_menu.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_menu.popup();
        }));
        column_view.add_controller(gesture);
    }

    // See:
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L266 (right click)
    // https://github.com/shartrec/kelpie-flight-planner/blob/a5575a5/src/window/airport_view.rs#L349 (context menu key)
    // https://discourse.gnome.org/t/adding-a-context-menu-to-a-listview-using-gtk4-rs/19995/5
}