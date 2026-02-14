/// This file contains code for interfacing between the CSV Song history
/// format defined within the "src/utils/csv_song_history.rs" file, the
/// GTK-rs GUI of SongRec and the filesystem while using the GUI.
use crate::gui::history_entry::HistoryEntry;
use crate::utils::csv_song_history::{HasSong, Song, SongHistoryRecord};
use gettextrs::gettext;
use gtk::prelude::*;
use std::collections::HashSet;
use std::error::Error;

trait SongHistoryRecordListStore {
    fn add_song_history_record(self: &mut Self, to_add: &SongHistoryRecord);
    fn remove_song(self: &mut Self, to_remove: Song);
    fn remove_song_history_record(self: &mut Self, to_remove: SongHistoryRecord);
}

// Extend gio::ListStore to integrate with SongHistoryRecord
// Cf. https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/struct.ListStore.html

impl SongHistoryRecordListStore for gio::ListStore {
    // This function first will be the first interacting with the ListStore
    // to be called after installing a fresh copy of the app

    fn add_song_history_record(self: &mut Self, to_add: &SongHistoryRecord) {
        self.insert(0, &HistoryEntry::new(to_add));
    }

    fn remove_song(self: &mut Self, to_remove: Song) {
        // Cf. https://gtk-rs.org/gtk-rs-core/git/docs/gio/struct.ListStore.html#method.remove
        // (Note: Song is SongHistoryRecord minus the recognition date)
        // This removes all items with the matching Song footprint
        self.retain(|item| {
            let item = item.clone().downcast::<HistoryEntry>().unwrap();
            item.get_song() != to_remove
        })
    }

    fn remove_song_history_record(self: &mut Self, to_remove: SongHistoryRecord) {
        self.remove_song(to_remove.get_song());
    }
}

#[derive(Debug, Clone)]
pub struct RecognitionHistoryInterface {
    csv_path: String,
    list_store: gio::ListStore,
}
#[derive(Debug, Clone)]
pub struct FavoritesInterface {
    csv_path: String,
    list_store: gio::ListStore,
    is_favorite: HashSet<Song>,
}

pub trait SongRecordInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn wipe_and_save(self: &mut Self);
    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord);

    fn load(self: &mut Self) -> Result<(), Box<dyn Error>>;
    fn remove(self: &mut Self, record: SongHistoryRecord);
    fn save(self: &mut Self);
}

impl dyn SongRecordInterface {}

#[test]
fn test_item_date() {
    let s = "Sat Aug 17 22:44:43 2024";
    let parsed = chrono::NaiveDateTime::parse_from_str(&s, "%c").unwrap();
    assert_eq!(&parsed.format("%c").to_string(), s);
}

impl SongRecordInterface for RecognitionHistoryInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = RecognitionHistoryInterface {
            csv_path: get_csv_path()?,
            list_store,
        };

        if let Err(error_info) = interface.load() {
            eprintln!(
                "{} {}",
                gettext("Error when reading the song history on the disk:"),
                error_info
            );
        }

        Ok(interface)
    }

    fn load(self: &mut Self) -> Result<(), Box<dyn Error>> {
        match csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&self.csv_path)
        {
            Ok(mut reader) => {
                let mut read = reader.deserialize().collect::<Vec<_>>();
                fn item_date(
                    item: &csv::Result<SongHistoryRecord>,
                ) -> Option<chrono::NaiveDateTime> {
                    let s = &item.as_ref().ok()?.recognition_date;
                    chrono::NaiveDateTime::parse_from_str(s, "%c").ok()
                }
                read.sort_by_cached_key(item_date);
                for result in read {
                    self.list_store.add_song_history_record(&result?)
                }
            }
            _ => {} // File does not exists, ignore
        };
        Ok(())
    }

    fn wipe_and_save(self: &mut Self) {
        self.list_store.remove_all();

        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        writer.flush().unwrap();
    }

    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord) {
        self.list_store.add_song_history_record(&record);

        self.save();
    }

    fn save(self: &mut Self) {
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        for item in self.list_store.iter::<glib::Object>() {
            let item = item.unwrap().downcast::<HistoryEntry>().unwrap();
            writer.serialize(item.get_song_history_record()).unwrap();
        }
        writer.flush().unwrap();
    }

    fn remove(self: &mut Self, song_record: SongHistoryRecord) {
        self.list_store.remove_song_history_record(song_record);
        self.save()
    }
}

impl SongRecordInterface for FavoritesInterface {
    fn new(
        list_store: gio::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = FavoritesInterface {
            csv_path: get_csv_path()?,
            list_store,
            is_favorite: HashSet::<Song>::new(),
        };

        if let Err(error_info) = interface.load() {
            eprintln!(
                "{} {}",
                gettext("Error when reading the favorites on the disk:"),
                error_info
            );
        }

        Ok(interface)
    }

    fn load(self: &mut Self) -> Result<(), Box<dyn Error>> {
        match csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&self.csv_path)
        {
            Ok(mut reader) => {
                for result in reader.deserialize() {
                    let record = result?;
                    self.list_store.add_song_history_record(&record);
                    self.is_favorite.insert(record.get_song());
                }
            }
            _ => {} // File does not exists, ignore
        };
        Ok(())
    }

    fn wipe_and_save(self: &mut Self) {
        self.list_store.remove_all();
        self.is_favorite.clear();
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        writer.flush().unwrap();
    }

    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord) {
        self.list_store.add_song_history_record(&record);
        self.is_favorite.insert(record.get_song());
        self.save();
    }

    fn save(self: &mut Self) {
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        for item in self.list_store.iter::<glib::Object>() {
            let item = item.unwrap().downcast::<HistoryEntry>().unwrap();
            writer.serialize(item.get_song_history_record()).unwrap();
        }
        writer.flush().unwrap();
    }

    fn remove(self: &mut Self, song_record: SongHistoryRecord) {
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
