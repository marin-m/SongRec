/// The application uses a simple CSV format in order to store the list of the
/// songs discovered by the user that are displayed in the multi-column list
/// view. The CSV columns bear the same name as the GUI list view columns (in
/// snake case).
///
/// A difference is that entries are stored in chronological order in the CSV
/// file, while antichronological order is used on the GUI list view.
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct SongHistoryRecord {
    pub song_name: String,
    #[serde(default)]
    pub album: Option<String>,

    // The following fields have been added in version 0.3.0
    #[serde(default)]
    pub track_key: Option<String>,
    #[serde(default)]
    pub release_year: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    pub recognition_date: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Song {
    pub song_name: String,
    pub album: Option<String>,

    // The following fields have been added in version 0.3.0
    pub track_key: Option<String>,
    pub release_year: Option<String>,
    pub genre: Option<String>,
}

pub trait IsSong {
    fn get_song(self: Self) -> Song;
}

impl IsSong for SongHistoryRecord {
    fn get_song(self: Self) -> Song {
        return Song {
            song_name: self.song_name,
            album: self.album,
            track_key: self.track_key,
            release_year: self.release_year,
            genre: self.genre,
        };
    }
}
