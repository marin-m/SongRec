use crate::utils::csv_song_history::{IsSong, Song, SongHistoryRecord};
use gettextrs::gettext;
use gtk::prelude::*;
use std::collections::HashSet;
use std::error::Error;
/// This file contains code for interfacing between the CSV Song history
/// format defined within the "src/utils/csv_song_history.rs" file, the
/// GTK-rs GUI of SongRec and the filesystem while using the GUI.

trait SongHistoryRecordListStore {
    fn remove_song_history_record(self: &mut Self, to_remove: SongHistoryRecord);
    fn remove_song(self: &mut Self, to_remove: Song);
    fn add_song_history_record(self: &mut Self, to_add: &SongHistoryRecord);
    fn add_song_history_records(self: &mut Self, to_add: &Vec<SongHistoryRecord>);
    fn get_song_history_record(self: &mut Self, iter: &gtk::TreeIter) -> Option<SongHistoryRecord>;
}

impl SongHistoryRecordListStore for gtk::ListStore {
    fn get_song_history_record(self: &mut Self, iter: &gtk::TreeIter) -> Option<SongHistoryRecord> {
        let song_name = self.get_value(&iter, 0).get::<String>().ok()??;
        let album = self.get_value(&iter, 1).get::<String>().ok()?;
        let recognition_date = self.get_value(&iter, 2).get::<String>().ok()??;
        let track_key = self.get_value(&iter, 3).get::<String>().ok()?;
        let release_year = self.get_value(&iter, 4).get::<String>().ok()?;
        let genre = self.get_value(&iter, 5).get::<String>().ok()?;

        Some(SongHistoryRecord {
            song_name,
            album,
            track_key,
            release_year,
            genre,
            recognition_date,
        })
    }

    fn remove_song(self: &mut Self, to_remove: Song) {
        if let Some(iter) = self.get_iter_first() {
            loop {
                if let Some(song_history_record) = self.get_song_history_record(&iter) {
                    if song_history_record.get_song() == to_remove {
                        self.remove(&iter);
                    }
                }
                if !self.iter_next(&iter) {
                    break;
                }
            }
        }
    }

    fn remove_song_history_record(self: &mut Self, to_remove: SongHistoryRecord) {
        let song_to_remove: Song = to_remove.get_song();
        if let Some(iter) = self.get_iter_first() {
            loop {
                if let Some(song_history_record) = self.get_song_history_record(&iter) {
                    if song_history_record.get_song() == song_to_remove {
                        self.remove(&iter);
                    }
                }
                if !self.iter_next(&iter) {
                    break;
                }
            }
        }
    }

    fn add_song_history_record(self: &mut Self, to_add: &SongHistoryRecord) {
        self.set(
            &self.insert(0),
            &[0, 1, 2, 3, 4, 5],
            &[
                &to_add.song_name,
                &to_add.album,
                &to_add.recognition_date,
                &to_add.track_key,
                &to_add.release_year,
                &to_add.genre,
            ],
        )
    }

    fn add_song_history_records(self: &mut Self, to_add: &Vec<SongHistoryRecord>) {
        for record in to_add {
            self.add_song_history_record(record)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecognitionHistoryInterface {
    csv_path: String,
    gtk_list_store: gtk::ListStore,
}
#[derive(Debug, Clone)]
pub struct FavoritesInterface {
    csv_path: String,
    gtk_list_store: gtk::ListStore,
    is_favorite: HashSet<Song>,
}

pub trait SongRecordInterface {
    fn new(
        gtk_list_store: gtk::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;
    fn load(self: &mut Self) -> Result<(), Box<dyn Error>>;
    fn wipe_and_save(self: &mut Self);
    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord);
    fn save(self: &mut Self);
    fn remove(self: &mut Self, record: SongHistoryRecord);
}

impl dyn SongRecordInterface {}

impl SongRecordInterface for RecognitionHistoryInterface {
    fn new(
        gtk_list_store: gtk::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = RecognitionHistoryInterface {
            csv_path: get_csv_path()?,
            gtk_list_store: gtk_list_store,
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
                for result in reader.deserialize() {
                    self.gtk_list_store.add_song_history_record(&result?)
                }
            }
            _ => {} // File does not exists, ignore
        };
        Ok(())
    }

    fn wipe_and_save(self: &mut Self) {
        self.gtk_list_store.clear();

        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        writer.flush().unwrap();
    }

    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord) {
        self.gtk_list_store.add_song_history_record(&record);

        self.save();
    }

    fn save(self: &mut Self) {
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        if let Some(iter) = self.gtk_list_store.get_iter_first() {
            loop {
                if let Some(song_history_record) =
                    self.gtk_list_store.get_song_history_record(&iter)
                {
                    writer.serialize(song_history_record).unwrap();
                }
                if !self.gtk_list_store.iter_next(&iter) {
                    break;
                }
            }
        }
        writer.flush().unwrap();
    }

    fn remove(self: &mut Self, song_record: SongHistoryRecord) {
        self.gtk_list_store.remove_song_history_record(song_record);
        self.save()
    }
}

impl SongRecordInterface for FavoritesInterface {
    fn new(
        gtk_list_store: gtk::ListStore,
        get_csv_path: fn() -> Result<String, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut interface = FavoritesInterface {
            csv_path: get_csv_path()?,
            gtk_list_store: gtk_list_store,
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
                    self.gtk_list_store.add_song_history_record(&record);
                    self.is_favorite.insert(record.get_song());
                }
            }
            _ => {} // File does not exists, ignore
        };
        Ok(())
    }

    fn wipe_and_save(self: &mut Self) {
        self.gtk_list_store.clear();
        self.is_favorite.clear();
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        writer.flush().unwrap();
    }

    fn add_row_and_save(self: &mut Self, record: SongHistoryRecord) {
        self.gtk_list_store.add_song_history_record(&record);
        self.is_favorite.insert(record.get_song());
        self.save();
    }

    fn save(self: &mut Self) {
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();

        if let Some(iter) = self.gtk_list_store.get_iter_first() {
            loop {
                if let Some(song_history_record) =
                    self.gtk_list_store.get_song_history_record(&iter)
                {
                    writer.serialize(song_history_record).unwrap();
                }
                if !self.gtk_list_store.iter_next(&iter) {
                    break;
                }
            }
        }
        writer.flush().unwrap();
    }

    fn remove(self: &mut Self, song_record: SongHistoryRecord) {
        let song = song_record.get_song();
        self.is_favorite.remove(&song);
        self.gtk_list_store.remove_song(song);
        self.save()
    }
}

impl FavoritesInterface {
    pub fn get_is_favorite(self: &Self) -> &HashSet<Song> {
        &self.is_favorite
    }
}
