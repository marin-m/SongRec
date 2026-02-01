use gio::prelude::*;
use gtk::prelude::*;
use glib::clone;
use adw::prelude::*;
use gettextrs::gettext;
use std::error::Error;
use std::sync::mpsc;

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::thread_messages::{*, GUIMessage::*};

use crate::gui::preferences::{PreferencesInterface, Preferences};

pub fn gui_main(recording: bool, input_file: Option<&str>, enable_mpris_cli: bool) -> Result<(), Box<dyn Error>> {
    
    let app = App::new();
    app.run();

    Ok(())
}

struct App {
    builder: gtk::Builder,
    preferences_interface: PreferencesInterface,
    old_preferences: Preferences
}

impl App {
    fn new() -> App {
        Self::load_resources();
        Self::setup_threads();

        gtk::init().unwrap();

        glib::set_prgname(Some("re.fossplant.songrec"));

        let builder = gtk::Builder::from_resource("/re/fossplant/songrec/interface.ui");

        let preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();

        App {
            builder,
            preferences_interface,
            old_preferences
        }
    }

    fn load_resources() {
        gio::resources_register_include!("compiled.gresource")
            .expect("Failed to register resources.");
    }

    fn setup_threads() {
        // TODO

        // NOTE: Dropping the removed glib::MainContext from legacy code:
        // https://discourse.gnome.org/t/help-required-to-migrate-from-dropped-maincontext-channel-api/20922
    }

    fn run(self) {
        let application = adw::Application::new(Some("re.fossplant.songrec"),
            gio::ApplicationFlags::HANDLES_OPEN);

        application.connect_activate(move |application| {
            self.on_activate(application);
        });
        application.run();
    }

    fn on_activate(&self, application: &adw::Application) {
        self.setup_actions();
        self.show_window(application);
    }

    fn setup_actions(&self) {
        // TODO
    }

    fn show_window(&self, application: &adw::Application) {
        let window: adw::ApplicationWindow = self.builder.object("window").unwrap();
        window.set_application(Some(application));

        /* let quit = gio::SimpleAction::new("quit", None);
        quit.connect_activate(glib::clone!(#[strong] application, move |_,_| {
            application.quit();
        })); 
        application.set_accels_for_action("app.quit", &["<Primary>Q"]);
        application.add_action(&quit);*/

        window.present();
    }
}