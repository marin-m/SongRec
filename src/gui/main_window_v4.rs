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
    old_preferences: Preferences,
    gui_tx: async_channel::Sender<GUIMessage>,
    gui_rx: async_channel::Receiver<GUIMessage>
}

impl App {
    fn new() -> App {
        let (gui_tx, gui_rx) = async_channel::unbounded();

        Self::load_resources();

        gtk::init().unwrap();
        glib::set_prgname(Some("re.fossplant.songrec"));

        let builder = gtk::Builder::from_resource("/re/fossplant/songrec/interface.ui");

        let preferences_interface: PreferencesInterface = PreferencesInterface::new();
        let old_preferences: Preferences = preferences_interface.preferences.clone();

        App {
            builder,
            preferences_interface,
            old_preferences,
            gui_tx,
            gui_rx
        }
    }

    fn load_resources() {
        gio::resources_register_include!("compiled.gresource")
            .expect("Failed to register resources.");
    }

    fn setup_intercom(&self) {
        // WIP: Setup threads + smol-rs/async-channel::unbounded listener

        // NOTE: Dropping the removed glib::MainContext from legacy code:
        // https://discourse.gnome.org/t/help-required-to-migrate-from-dropped-maincontext-channel-api/20922
        // + https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html#how-to-avoid-blocking-the-main-loop

        glib::spawn_future_local(async move {

        });
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
        self.setup_intercom();
        self.setup_actions();
        self.show_window(application);
    }

    fn setup_actions(&self) {
        let window: adw::ApplicationWindow = self.builder.object("window").unwrap();
        let about_dialog: adw::AboutDialog = self.builder.object("about_dialog").unwrap();

        let action_show_about = gio::ActionEntry::builder("show-about")
            .activate(
                move |window, _, _| {
                    about_dialog.present(Some(window));
                }
            )
            .build();
        
        #[cfg(feature = "mpris")]
        let action_mpris_setting = gio::ActionEntry::builder("mpris-setting")
            .state(self.old_preferences.enable_mpris.unwrap().to_variant())
            .activate(|_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state; // toggle
                action.set_state(&new_state.to_variant());

                let mut new_preference: Preferences = Preferences::new();
                new_preference.enable_mpris = Some(new_state);
                // gui_tx.send_blocking(GUIMessage::UpdatePreference(new_preference)).unwrap(); // WIP

            })
            .build();

        window.add_action_entries([
            action_show_about,
            #[cfg(feature = "mpris")]
            action_mpris_setting, // DON'T FORGET to put a tooltip for this
            // WIP xx
        ]);
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