use std::error::Error;
use gettextrs::gettext;
use gtk::prelude::*;

use crate::utils::csv_song_history::SongHistoryRecord;
use crate::utils::filesystem_operations::obtain_song_history_csv_path;
/// This file contains code for interfacing between the CSV Song history
/// format defined within the "src/utils/csv_song_history.rs" file, the
/// GTK-rs GUI of SongRec and the filesystem while using the GUI.

pub struct SongHistoryInterface {
    csv_path: String,
    gtk_list_store: gtk::ListStore,
    chronological_records: Vec<SongHistoryRecord>
}

impl SongHistoryInterface {
    
    pub fn new(gtk_list_store: gtk::ListStore) -> Result<Self, Box<dyn Error>> {

        let mut interface = SongHistoryInterface {
            csv_path: obtain_song_history_csv_path()?,
            gtk_list_store: gtk_list_store,
            chronological_records: vec![]
        };
        
        if let Err(error_info) = interface.load() {
            eprintln!("{} {}", gettext("Error when reading the song history on the disk:"), error_info);
        }
        
        Ok(interface)
    }
    
    
    /// All the code displaying the initial state of the song recognition
    /// history is stored here.
    
    fn load(self: &mut Self) -> Result<(), Box<dyn Error>> {
        match csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(&self.csv_path) {
            Ok(mut reader) => {
                    
                for result in reader.deserialize() {
                    
                    let record: SongHistoryRecord = result?;

                    self.gtk_list_store.set(&self.gtk_list_store.insert(0), &[0, 1, 2], &[&record.song_name, &record.album, &record.recognition_date]);

                    self.chronological_records.push(record);
                                        
                };
            },
            _ => { } // File does not exists, ignore
        };
        Ok(())
    }
    
    pub fn wipe_and_save(self: &mut Self) {
        self.chronological_records.clear();
        
        self.gtk_list_store.clear();
        
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();
        
        writer.flush().unwrap();
    }
    
    pub fn add_column_and_save(self: &mut Self, record: SongHistoryRecord) {
        self.gtk_list_store.set(&self.gtk_list_store.insert(0), &[0, 1, 2], &[&record.song_name, &record.album, &record.recognition_date]);

        self.chronological_records.push(record);
        
        let mut writer = csv::Writer::from_path(&self.csv_path).unwrap();
        
        for record in self.chronological_records.iter() {
            writer.serialize(record).unwrap();
        }
        
        writer.flush().unwrap();
    }
    
}
