
use std::cell::RefCell;

use glib::Properties;
use glib::object::ObjectExt;
use glib::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::ObjectSubclass;
use glib::subclass::object::DerivedObjectProperties;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::HistoryEntry)]
pub struct HistoryEntry {
    #[property(construct_only, get)]
    song_name: RefCell<String>,
    #[property(construct_only, get)]
    album: RefCell<Option<String>>,
    #[property(construct_only, get)]
    track_key: RefCell<Option<String>>,
    #[property(construct_only, get)]
    release_year: RefCell<Option<String>>,
    #[property(construct_only, get)]
    genre: RefCell<Option<String>>,
    #[property(construct_only, get)]
    recognition_date: RefCell<String>
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for HistoryEntry {
    const NAME: &'static str = "HistoryEntry";
    type Type = super::HistoryEntry;
    type ParentType = glib::Object;
}

// Trait shared by all GObjects
#[glib::derived_properties]
impl ObjectImpl for HistoryEntry {}

