mod imp;

use glib::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::ObjectSubclass;

glib::wrapper! {
    pub struct ListedDevice(ObjectSubclass<imp::ListedDevice>)
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl ListedDevice {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}