use std::panic;
use std::sync::Arc;

use mpris_player::{MprisPlayer, PlaybackStatus, Metadata};

use crate::core::thread_messages::SongRecognizedMessage;

fn init_player(p: Arc<MprisPlayer>) -> Arc<MprisPlayer> {
    p.set_can_quit(false);
    p.set_can_raise(false);
    p.set_can_set_fullscreen(false);

    p.set_can_control(false);
    p.set_can_seek(false);
    p.set_can_go_next(false);
    p.set_can_go_previous(false);
    p.set_can_play(true);
    p.set_can_pause(false);
    p.set_playback_status(PlaybackStatus::Playing);

    p
}

pub fn get_player() -> Option<Arc<MprisPlayer>> {
    // MprisPlayer::new may panic if DBus is unavailable,
    // so we need to mess around with panic::catch_unwind

    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let player = panic::catch_unwind(|| MprisPlayer::new(
        "SongRec".to_string(),
        "SongRec".to_string(),
        "com.github.marinm.songrec.desktop".to_string()
    ));
    panic::set_hook(prev_hook);

    player.map(init_player).ok()
}

pub fn update_song(p: &MprisPlayer, m: &SongRecognizedMessage) {
    let mut metadata = Metadata::new();
    metadata.title = Some(m.song_name.clone());
    metadata.artist = Some(vec![m.artist_name.clone()]);
    metadata.album = m.album_name.clone();
    if let Some(ref genre) = m.genre { 
        metadata.genre = Some(vec![genre.clone()]);
    }
    if let Some(ref buf) = m.cover_image { 
        metadata.art_url = Some(format!("data:image/jpeg;base64,{}", base64::encode(buf)));
    }
    p.set_metadata(metadata);
}
