mod imp;

use crate::utils::csv_song_history::{Song, SongHistoryRecord};

glib::wrapper! {
    pub struct HistoryEntry(ObjectSubclass<imp::HistoryEntry>);
}

impl HistoryEntry {
    pub fn new(song: &SongHistoryRecord) -> Self {
        glib::Object::builder()
            .property("song_name", song.song_name.clone())
            .property("album", song.album.clone())
            .property("track_key", song.track_key.clone())
            .property("release_year", song.release_year.clone())
            .property("genre", song.genre.clone())
            .property("recognition_date", song.recognition_date.clone())
            .build()

        /*
            let mut builder = builder.property("song_name", song.song_name);
            if let Some(album) = song.album {
                builder = builder.property("album", album);
            }
            if let Some(track_key) = song.track_key {
                builder = builder.property("track_key", track_key);
            }
            if let Some(release_year) = song.release_year {
                builder = builder.property("release_year", release_year);
            }
            if let Some(genre) = song.genre {
                builder = builder.property("genre", genre);
            }
            builder = builder.property("recognition_date", song.recognition_date);
        */
    }

    pub fn get_song_history_record(&self) -> SongHistoryRecord {
        SongHistoryRecord {
            song_name: self.song_name(),
            album: self.album(),
            track_key: self.track_key(),
            release_year: self.release_year(),
            genre: self.genre(),
            recognition_date: self.recognition_date(),
        }
    }

    pub fn get_song(&self) -> Song {
        Song {
            song_name: self.song_name(),
            album: self.album(),
            track_key: self.track_key(),
            release_year: self.release_year(),
            genre: self.genre(),
        }
    }
}
