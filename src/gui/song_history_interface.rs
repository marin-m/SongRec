/// This file contains code for interfacing between the CSV Song history
/// format defined within the "src/utils/csv_song_history.rs" file, the
/// GTK-rs GUI of SongRec and the filesystem while using the GUI.
use crate::gui::history_entry::HistoryEntry;
use crate::utils::csv_song_history::{HasSong, Song, SongHistoryRecord};
use gettextrs::gettext;
use gtk::prelude::*;
use log::error;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

trait SongHistoryRecordListStore {
    fn add_song_history_record(&mut self, to_add: &SongHistoryRecord);
    fn remove_song(&mut self, to_remove: Song);
    fn remove_song_history_record(&mut self, to_remove: SongHistoryRecord);
}

// Extend gio::ListStore to integrate with SongHistoryRecord
// Cf. https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/struct.ListStore.html

impl SongHistoryRecordListStore for gio::ListStore {
    // This function first will be the first interacting with the ListStore
    // to be called after installing a fresh copy of the app

    fn add_song_history_record(&mut self, to_add: &SongHistoryRecord) {
        self.insert(0, &HistoryEntry::new(to_add));
    }

    fn remove_song(&mut self, to_remove: Song) {
        // Cf. https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.ListStore.html#method.remove
        // (Note: Song is SongHistoryRecord minus the recognition date)
        // This removes all items with the matching Song footprint
        self.retain(|item| {
            let item = item.clone().downcast::<HistoryEntry>().unwrap();
            item.get_song() != to_remove
        })
    }

    fn remove_song_history_record(&mut self, to_remove: SongHistoryRecord) {
        self.remove_song(to_remove.get_song());
    }
}

#[derive(Debug, Clone)]
pub struct RecognitionHistoryInterface {
    csv_path: PathBuf,
    list_store: gio::ListStore,
}
#[derive(Debug, Clone)]
pub struct FavoritesInterface {
    csv_path: PathBuf,
    list_store: gio::ListStore,
    is_favorite: HashSet<Song>,
}

pub trait SongRecordInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<PathBuf, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn wipe_and_save(&mut self);
    fn add_row_and_save(&mut self, record: SongHistoryRecord);

    fn load(&mut self) -> Result<(), Box<dyn Error>>;
    fn remove(&mut self, record: SongHistoryRecord);
    fn save(&mut self);
}

impl dyn SongRecordInterface {}

/// Write every row of LIST_STORE to CSV_PATH. Saving runs from GUI
/// callbacks, so a file that cannot be written is reported to the caller
/// instead of panicking.
fn write_records(csv_path: &PathBuf, list_store: &gio::ListStore) -> Result<(), Box<dyn Error>> {
    let mut writer = csv::Writer::from_path(csv_path)?;
    for item in list_store.iter::<glib::Object>() {
        let item = item?
            .downcast::<HistoryEntry>()
            .map_err(|_| "unexpected item in the list")?;
        writer.serialize(item.get_song_history_record())?;
    }
    writer.flush()?;
    Ok(())
}

#[test]
fn test_item_date() {
    let s = "Sat Aug 17 22:44:43 2024";
    let parsed = chrono::NaiveDateTime::parse_from_str(s, "%c").unwrap();
    assert_eq!(&parsed.format("%c").to_string(), s);
}

impl SongRecordInterface for RecognitionHistoryInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<PathBuf, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = RecognitionHistoryInterface {
            csv_path: get_csv_path()?,
            list_store,
        };

        if let Err(error_info) = interface.load() {
            error!(
                "{} {}",
                gettext("Error when reading the song history on the disk:"),
                error_info
            );
        }

        Ok(interface)
    }

    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        if let Ok(mut reader) = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&self.csv_path)
        {
            let mut read = reader.deserialize().collect::<Vec<_>>();
            fn item_date(item: &csv::Result<SongHistoryRecord>) -> Option<chrono::NaiveDateTime> {
                let s = &item.as_ref().ok()?.recognition_date;
                chrono::NaiveDateTime::parse_from_str(s, "%c").ok()
            }
            read.sort_by_cached_key(item_date);
            for result in read {
                self.list_store.add_song_history_record(&result?)
            }
        }
        Ok(())
    }

    fn wipe_and_save(&mut self) {
        self.list_store.remove_all();
        self.save();
    }

    fn add_row_and_save(&mut self, record: SongHistoryRecord) {
        self.list_store.add_song_history_record(&record);

        self.save();
    }

    fn save(&mut self) {
        if let Err(error_info) = write_records(&self.csv_path, &self.list_store) {
            error!(
                "{} {}",
                gettext("Error while saving the song history on the disk:"),
                error_info
            );
        }
    }

    fn remove(&mut self, song_record: SongHistoryRecord) {
        self.list_store.remove_song_history_record(song_record);
        self.save()
    }
}

impl SongRecordInterface for FavoritesInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<PathBuf, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = FavoritesInterface {
            csv_path: get_csv_path()?,
            list_store,
            is_favorite: HashSet::<Song>::new(),
        };

        if let Err(error_info) = interface.load() {
            error!(
                "{} {}",
                gettext("Error when reading the favorites on the disk:"),
                error_info
            );
        }

        Ok(interface)
    }

    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        if let Ok(mut reader) = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&self.csv_path)
        {
            for result in reader.deserialize() {
                let record = result?;
                self.list_store.add_song_history_record(&record);
                self.is_favorite.insert(record.get_song());
            }
        }
        Ok(())
    }

    fn wipe_and_save(&mut self) {
        self.list_store.remove_all();
        self.is_favorite.clear();
        self.save();
    }

    fn add_row_and_save(&mut self, record: SongHistoryRecord) {
        self.list_store.add_song_history_record(&record);
        self.is_favorite.insert(record.get_song());
        self.save();
    }

    fn save(&mut self) {
        if let Err(error_info) = write_records(&self.csv_path, &self.list_store) {
            error!(
                "{} {}",
                gettext("Error while saving the favorites on the disk:"),
                error_info
            );
        }
    }

    fn remove(&mut self, song_record: SongHistoryRecord) {
        let song = song_record.get_song();
        self.is_favorite.remove(&song);
        self.list_store.remove_song(song);
        self.save()
    }
}

impl FavoritesInterface {
    pub fn is_favorite<T: HasSong>(&self, has_song: T) -> bool {
        self.is_favorite.contains(&has_song.get_song())
    }
}

#[cfg(test)]
mod save_tests {
    use super::*;

    fn record(song_name: &str) -> SongHistoryRecord {
        SongHistoryRecord {
            song_name: song_name.to_string(),
            album: Some("Album".to_string()),
            track_key: Some("1".to_string()),
            release_year: Some("2020".to_string()),
            genre: Some("Pop".to_string()),
            recognition_date: "Sat Aug 17 22:44:43 2024".to_string(),
        }
    }

    /// A CSV path whose directory does not exist, so writing it fails.
    fn unwritable_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("songrec-missing-dir-{}-{name}", std::process::id()))
            .join("song_history.csv")
    }

    #[test]
    fn history_keeps_working_when_the_csv_cannot_be_written() {
        let mut history = RecognitionHistoryInterface {
            csv_path: unwritable_path("history"),
            list_store: gio::ListStore::new::<HistoryEntry>(),
        };

        history.add_row_and_save(record("Artist - Title"));
        history.add_row_and_save(record("Artist - Other"));
        history.remove(record("Artist - Title"));
        assert_eq!(history.list_store.n_items(), 1);
        history.wipe_and_save();
        assert_eq!(history.list_store.n_items(), 0);
    }

    #[test]
    fn favorites_keep_working_when_the_csv_cannot_be_written() {
        let mut favorites = FavoritesInterface {
            csv_path: unwritable_path("favorites"),
            list_store: gio::ListStore::new::<HistoryEntry>(),
            is_favorite: HashSet::new(),
        };

        favorites.add_row_and_save(record("Artist - Title"));
        assert!(favorites.is_favorite(record("Artist - Title").get_song()));
        favorites.remove(record("Artist - Title"));
        assert!(!favorites.is_favorite(record("Artist - Title").get_song()));
        favorites.wipe_and_save();
        assert_eq!(favorites.list_store.n_items(), 0);
    }

    #[test]
    fn history_is_saved_and_loaded_back() {
        let csv_path =
            std::env::temp_dir().join(format!("songrec-history-{}.csv", std::process::id()));
        let mut history = RecognitionHistoryInterface {
            csv_path: csv_path.clone(),
            list_store: gio::ListStore::new::<HistoryEntry>(),
        };
        history.add_row_and_save(record("Artist - Title"));

        let mut loaded = RecognitionHistoryInterface {
            csv_path: csv_path.clone(),
            list_store: gio::ListStore::new::<HistoryEntry>(),
        };
        loaded.load().unwrap();
        assert_eq!(loaded.list_store.n_items(), 1);
        let _ = std::fs::remove_file(csv_path);
    }
}
