/// The application uses a simple CSV format in order to store the list of the
/// songs discovered by the user that are displayed in the multi-column list
/// view. The CSV columns bear the same name as the GUI list view columns (in
/// snake case).
///
/// A difference is that entries are stored in chronological order in the CSV
/// file, while antichronological order is used on the GUI list view.
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct SongHistoryRecord {
    #[serde(flatten)]
    pub song: Song,
    pub recognition_date: String,
}

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Song {
    pub song_name: String,
    pub album: String,

    // The following fields have been added in version 0.3.0
    #[serde(default)]
    pub track_key: Option<String>,
    #[serde(default)]
    pub release_year: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
}
