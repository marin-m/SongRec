use std::error::Error;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, mpsc};

use glib;
use glib::clone;

use mpris_player::PlaybackStatus;

use crate::core::microphone_thread::microphone_thread;
use crate::core::processing_thread::processing_thread;
use crate::core::http_thread::http_thread;
use crate::core::thread_messages::{GUIMessage, MicrophoneMessage};

use crate::utils::mpris_player::{get_player, update_song};
use crate::utils::thread::spawn_big_thread;


pub fn cli_main(audio_device: Option<&str>) -> Result<(), Box<dyn Error>> {
    glib::MainContext::default().acquire();
    let main_loop = Arc::new(glib::MainLoop::new(None, false));

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

    let main_loop_gui = main_loop.clone();

    let audio_dev_name = audio_device.as_ref().map(|dev| dev.to_string());

    gui_rx.attach(None, move |gui_message| {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                let dev_name = if let Some(dev) = &audio_dev_name {
                    if !device_names.contains(dev) {
                        println!("Exiting: audio device not found");
                        main_loop_gui.quit();
                        return glib::Continue(false);
                    }
                    dev
                } else {
                    if device_names.is_empty() {
                        println!("Exiting: no audio devices found!");
                        main_loop_gui.quit();
                        return glib::Continue(false);
                    }
                    &device_names[0]
                };
                println!("Using device {}", dev_name);
                microphone_tx.send(MicrophoneMessage::MicrophoneRecordStart(dev_name.to_owned())).unwrap();
            },
            GUIMessage::NetworkStatus(reachable) => {
                let mpris_status = if reachable { PlaybackStatus::Playing } else { PlaybackStatus::Paused };
                mpris_player.as_ref().map(|p| p.set_playback_status(mpris_status));
            },
            GUIMessage::MicrophoneRecording => {
                println!("Recording started!");
            },
            GUIMessage::SongRecognized(message) => {
                let mut last_track_borrow = last_track.borrow_mut();
                let track_key = Some(message.track_key.clone());
                if *last_track_borrow != track_key {
                    mpris_player.as_ref().map(|p| update_song(p, &message));
                    *last_track_borrow = track_key;
                }
            },
            _ => { }
        }
        glib::Continue(true)
    });

    main_loop.run();
    Ok(())
}
