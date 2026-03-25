use log::{debug, error};
use std::fs;

// TODO rewrite Cf. https://github.com/SeaDve/mpris-server/blob/main/examples/player.rs
// https://github.com/SeaDve/mpris-server/blob/main/examples/local_server.rs
// https://github.com/SeaDve/mpris-server/blob/main/examples/server.rs

use mpris_server::{Metadata, PlaybackStatus, Player};

use crate::core::thread_messages::SongRecognizedMessage;
use crate::utils::filesystem_operations::obtain_cache_directory;
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;

pub async fn get_player(gui_mode: bool) -> Option<Player> {
    match Player::builder(match std::env::var("SNAP_NAME") {
        Ok(_) => "songrec",
        _ => "re.fossplant.songrec",
    })
    .can_quit(gui_mode)
    .can_raise(gui_mode)
    .can_seek(false)
    .can_go_next(false)
    .can_go_previous(false)
    .can_play(true)
    .can_pause(false)
    .playback_status(PlaybackStatus::Playing)
    .has_track_list(true)
    .identity("SongRec")
    .desktop_entry(match std::env::var("SNAP_NAME") {
        Ok(_) => "com.github.marinm.songrec",
        _ => "re.fossplant.songrec",
    })
    .build()
    .await
    {
        Ok(player) => {
            glib::spawn_future_local(player.run());
            Some(player)
        }
        Err(error) => {
            error!("Could not initialize MPRIS: {:?}", error);
            None
        }
    }
}

pub async fn update_song(
    player: &Player,
    message: &SongRecognizedMessage,
    last_cover_path: &mut Option<std::path::PathBuf>,
) {
    let mut metadata = Metadata::builder()
        .title(message.song_name.clone())
        .artist(vec![message.artist_name.clone()]);
    if let Some(ref album) = message.album_name {
        metadata = metadata.album(album.clone());
    }
    if let Some(ref genre) = message.genre {
        metadata = metadata.genre(vec![genre.clone()]);
    }

    // Clean up old cover file
    if let Some(path) = last_cover_path.take() {
        let _ = fs::remove_file(path);
    }

    if let Some(ref buf) = message.cover_image {
        let (mime_ext, mime_type) = if buf.len() >= 4
            && buf[0] == 0x89
            && buf[1] == b'P'
            && buf[2] == b'N'
            && buf[3] == b'G'
        {
            ("png", "image/png")
        } else if buf.len() >= 3 && buf[0] == 0x47 && buf[1] == 0x49 && buf[2] == 0x46 {
            ("gif", "image/gif")
        } else if buf.len() >= 12 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
            ("webp", "image/webp")
        } else {
            // default to jpeg if unknown
            ("jpg", "image/jpeg")
        };
        let process_uid = std::fs::metadata("/proc/self")
            .map(|message| message.uid())
            .unwrap_or(0);
        let mut tmp = obtain_cache_directory().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        tmp.push(format!(
            "songrec_cover_{}_{}.{}",
            process_uid, timestamp, mime_ext
        ));
        debug!("Writing cover file to {:?}", tmp);
        if fs::write(&tmp, buf).is_ok() {
            // Use file:// URL for better compatibility with MPRIS clients
            metadata = metadata.art_url(format!("file://{}", tmp.display()));
            *last_cover_path = Some(tmp);
        } else {
            // Fallback to data URI (ensure we use the correct mime type)
            metadata =
                metadata.art_url(format!("data:{};base64,{}", mime_type, base64::encode(buf)));
        }
    }
    if let Err(error) = player.set_metadata(metadata.build()).await {
        error!("Could not set MPRIS metadata: {:?}", error);
    }
}
