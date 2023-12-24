use crate::fingerprinting::signature_format::DecodedSignature;
#[cfg(feature = "gui")]
use crate::gui::preferences::Preferences;
#[cfg(feature = "gui")]
use crate::utils::csv_song_history::SongHistoryRecord;

use std::thread;

/// This module contains code used from message-based communication between threads.

pub fn spawn_big_thread<F, T>(argument: F) -> ()
    where
        F: std::ops::FnOnce() -> T,
        F: std::marker::Send + 'static,
        T: std::marker::Send + 'static {
    thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(argument).unwrap();
}

pub struct SongRecognizedMessage {
    pub artist_name: String,
    pub album_name: Option<String>,
    pub song_name: String,
    pub cover_image: Option<Vec<u8>>,
    pub signature: Box<DecodedSignature>,

    // Used only in the CSV export for now:
    pub track_key: String,
    pub release_year: Option<String>,
    pub genre: Option<String>,

    pub shazam_json: String,
}

pub struct DeviceListItem {
    pub inner_name: String,
    pub display_name: String,
    // The checkbox option on the UI should select the first monitor
    // device present in the combo box, when specified
    pub is_monitor: bool
}

pub enum GUIMessage {
    ErrorMessage(String),
    // A list of audio devices, received from the microphone thread
    // because CPAL can't be called from the same thread as the GUI
    // under Windows
    DevicesList(Box<Vec<DeviceListItem>>),
    #[cfg(feature = "gui")]
    UpdatePreference(Preferences),
    #[cfg(feature = "gui")]
    AddFavorite(SongHistoryRecord),
    #[cfg(feature = "gui")]
    RemoveFavorite(SongHistoryRecord),
    #[cfg(feature = "gui")]
    ShowFavorites,
    NetworkStatus(bool), // Is the network reachable?
    #[cfg(feature = "gui")]
    WipeSongHistory,
    MicrophoneRecording,
    MicrophoneVolumePercent(f32),
    SongRecognized(Box<SongRecognizedMessage>),
}

pub enum MicrophoneMessage {
    MicrophoneRecordStart(String), // The argument is the audio device name
    MicrophoneRecordStop,
    ProcessingDone,
}

pub enum ProcessingMessage {
    ProcessAudioFile(String),
    ProcessAudioSamples(Box<Vec<i16>>), // Prefer to use heap across threads to avoid stack overflow
}

pub enum HTTPMessage {
    RecognizeSignature(Box<DecodedSignature>),
}
