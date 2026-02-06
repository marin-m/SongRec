// WIP:
// The item that follows should be stored in a Gio.ListStore
// in remplacement in the (device_name_str, is_monitor) tuple
// that we used in the former Rust code, and should be bound to
// the "audio_inputs" Adw.ComboRow from SongRec's new GTK-4 UI tuple

// See:
// https://gtk-rs.org/gtk4-rs/stable/latest/book/g_object_subclassing.html
// https://docs.gtk.org/gio/class.ListStore.html
// https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.2/class.ComboRow.html

// See:
// https://docs.gtk.org/gtk4/class.BuilderListItemFactory.html
// and
// https://docs.gtk.org/gtk4/class.SignalListItemFactory.html

use glib::Properties;
use glib::object::ObjectExt;
use glib::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::ObjectSubclass;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::ListedDevice)]
pub struct ListedDevice {
    #[property(get)]
    display_name: String,
    #[property(get)]
    inner_name: String,
    #[property(get)]
    is_monitor: bool
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ListedDevice {
    const NAME: &'static str = "ListedDevice";
    type Type = super::ListedDevice;
    type ParentType = glib::Object;
}

// Trait shared by all GObjects
impl ObjectImpl for ListedDevice {}


