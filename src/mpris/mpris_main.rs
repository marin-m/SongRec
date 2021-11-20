use std::error::Error;
use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;

use glib;
use glib::clone;

use mpris_player::{MprisPlayer, PlaybackStatus, Metadata};

use crate::gui::microphone_thread::microphone_thread;
use crate::gui::processing_thread::processing_thread;
use crate::gui::http_thread::http_thread;
use crate::gui::thread_messages::{GUIMessage, MicrophoneMessage, SongRecognizedMessage};

use crate::gui::main_window::spawn_big_thread;


fn init_mpris_player(p: &MprisPlayer) {
    p.set_can_quit(false);
    p.set_can_raise(false);
    p.set_can_set_fullscreen(false);

    p.set_can_control(false);
    p.set_can_seek(false);
    p.set_can_go_next(false);
    p.set_can_go_previous(false);
    p.set_can_play(true);
    p.set_can_pause(false);
    p.set_playback_status(PlaybackStatus::Stopped);
}

fn mpris_update_song(p: &MprisPlayer, m: &SongRecognizedMessage) {
    let mut metadata = Metadata::new();
    println!("{}", m.track_key);
    metadata.title = Some(m.song_name.clone());
    metadata.artist = Some(vec![m.artist_name.clone()]);
    metadata.album = m.album_name.clone();
    match m.genre {
        Some(ref genre) =>
            metadata.genre = Some(vec![genre.clone()]),
        None => {},
    }
    match m.cover_image {
        Some(ref buf) =>
            metadata.art_url = Some(format!("data:image/jpeg;base64,{}", base64::encode(buf))),
        None => {},
    }
    p.set_metadata(metadata);
}

pub fn mpris_main() -> Result<(), Box<dyn Error>> {
    glib::MainContext::default().acquire();
    let p = MprisPlayer::new(
        "SongRec".to_string(),
        "SongRec".to_string(),
        "com.github.marinm.songrec.desktop".to_string()
    );
    init_mpris_player(&p);

    let (mpris_tx, mpris_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (microphone_tx, microphone_rx) = mpsc::channel();
    let (processing_tx, processing_rx) = mpsc::channel();
    let (http_tx, http_rx) = mpsc::channel();

    let microphone_http_tx = microphone_tx.clone();

    spawn_big_thread(clone!(@strong mpris_tx => move || { // microphone_rx, processing_tx
        microphone_thread(microphone_rx, processing_tx, mpris_tx);
    }));
    
    spawn_big_thread(clone!(@strong mpris_tx => move || { // processing_rx, http_tx
        processing_thread(processing_rx, http_tx, mpris_tx);
    }));
    
    spawn_big_thread(clone!(@strong mpris_tx => move || { // http_rx
        http_thread(http_rx, mpris_tx, microphone_http_tx);
    }));

    let last_track: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    mpris_rx.attach(None, move |gui_message| {
        match gui_message {
            GUIMessage::DevicesList(device_names) => {
                println!("Using device {}", device_names[0]);
                p.set_playback_status(PlaybackStatus::Playing);
                microphone_tx.send(MicrophoneMessage::MicrophoneRecordStart(device_names[0].to_owned())).unwrap();
            },
            GUIMessage::NetworkStatus(reachable) => {
                let status = if reachable { PlaybackStatus::Playing } else { PlaybackStatus::Paused };
                p.set_playback_status(status);
            },
            GUIMessage::MicrophoneRecording => {
                println!("Recording started!!!");
            },
            GUIMessage::SongRecognized(message) => {
                let mut last_track_borrow = last_track.borrow_mut();
                let track_key = Some(message.track_key.clone());
                if *last_track_borrow != track_key {
                    mpris_update_song(&p, &message);
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
