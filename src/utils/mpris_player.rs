use std::panic;
use std::sync::Arc;
use std::fs;

// TODO rewrite Cf. https://github.com/SeaDve/mpris-server/blob/main/examples/player.rs
// https://github.com/SeaDve/mpris-server/blob/main/examples/local_server.rs
// https://github.com/SeaDve/mpris-server/blob/main/examples/server.rs

use mpris_player::{MprisPlayer, PlaybackStatus, Metadata};

use crate::core::thread_messages::SongRecognizedMessage;
use std::time::SystemTime;
use std::os::unix::fs::MetadataExt;

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
        "re.fossplant.songrec.desktop".to_string()
    ));
    panic::set_hook(prev_hook);

    player.map(init_player).ok()
}

pub fn update_song(p: &MprisPlayer, m: &SongRecognizedMessage, last_cover_path: &mut Option<std::path::PathBuf>) {
    let mut metadata = Metadata::new();
    metadata.title = Some(m.song_name.clone());
    metadata.artist = Some(vec![m.artist_name.clone()]);
    metadata.album = m.album_name.clone();
    if let Some(ref genre) = m.genre { 
        metadata.genre = Some(vec![genre.clone()]);
    }

    // Clean up old cover file
    if let Some(path) = last_cover_path.take() {
        let _ = fs::remove_file(path);
    }

    if let Some(ref buf) = m.cover_image { 
        let (mime_ext, mime_type) = if buf.len() >= 4 && buf[0] == 0x89 && buf[1] == b'P' && buf[2] == b'N' && buf[3] == b'G' {
            ("png", "image/png")
        } else if buf.len() >= 3 && buf[0] == 0x47 && buf[1] == 0x49 && buf[2] == 0x46 {
            ("gif", "image/gif")
        } else if buf.len() >= 12 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
            ("webp", "image/webp")
        } else {
            // default to jpeg if unknown
            ("jpg", "image/jpeg")
        };
        let process_uid = std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0);
        let mut tmp = std::env::temp_dir();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_secs();
        tmp.push(format!("songrec_cover_{}_{}.{}", process_uid, timestamp, mime_ext));
        if fs::write(&tmp, buf).is_ok() {
            // Use file:// URL for better compatibility with MPRIS clients
            metadata.art_url = Some(format!("file://{}", tmp.display()));
            *last_cover_path = Some(tmp);
        } else {
            // Fallback to data URI (ensure we use the correct mime type)
            metadata.art_url = Some(format!("data:{};base64,{}", mime_type, base64::encode(buf)));
        }
    }
    p.set_metadata(metadata);
}
