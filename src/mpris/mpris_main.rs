use std::error::Error;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use glib;
use glib::clone;

use mpris_player::PlaybackStatus;

use crate::gui::microphone_thread::microphone_thread;
use crate::gui::processing_thread::processing_thread;
use crate::gui::http_thread::http_thread;
use crate::utils::mpris_player::{get_player, update_song};

use crate::gui::thread_messages::{GUIMessage, MicrophoneMessage};

use crate::gui::main_window::spawn_big_thread;

pub fn mpris_main() -> Result<(), Box<dyn Error>> {
    glib::MainContext::default().acquire();

    let (gui_tx, gui_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (microphone_tx, microphone_rx) = mpsc::channel();
    let (processing_tx, processing_rx) = mpsc::channel();
    let (http_tx, http_rx) = mpsc::channel();

    let microphone_http_tx = microphone_tx.clone();

    spawn_big_thread(clone!(@strong gui_tx => move || { // microphone_rx, processing_tx
        microphone_thread(microphone_rx, processing_tx, gui_tx);
    }));
    
    spawn_big_thread(clone!(@strong gui_tx => move || { // processing_rx, http_tx
        processing_thread(processing_rx, http_tx, gui_tx);
    }));
    
    spawn_big_thread(clone!(@strong gui_tx => move || { // http_rx
        http_thread(http_rx, gui_tx, microphone_http_tx);
    }));

    let mpris_player = get_player();
    let last_track: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    gui_rx.attach(None, move |gui_message| {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                println!("Using device {}", device_names[0]);
                microphone_tx.send(MicrophoneMessage::MicrophoneRecordStart(device_names[0].to_owned())).unwrap();
            },
            GUIMessage::NetworkStatus(reachable) => {
                let mpris_status = if reachable { PlaybackStatus::Playing } else { PlaybackStatus::Paused };
                mpris_player.set_playback_status(mpris_status);
            },
            GUIMessage::MicrophoneRecording => {
                println!("Recording started!!!");
            },
            GUIMessage::SongRecognized(message) => {
                let mut last_track_borrow = last_track.borrow_mut();
                let track_key = Some(message.track_key.clone());
                if *last_track_borrow != track_key {
                    update_song(&mpris_player, &message);
                    *last_track_borrow = track_key;
                }
            },
            _ => { }
        }
        glib::Continue(true)
    });

    glib::MainLoop::new(None, false).run();
    Ok(())
}
