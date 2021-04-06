use crate::fingerprinting::signature_format::DecodedSignature;

/// This module contains code used from message-based communication between threads.

pub struct SongRecognizedMessage {
    pub artist_name: String,
    pub album_name: Option<String>,
    pub song_name: String,
    pub cover_image: Option<Vec<u8>>,
    pub signature: Box<DecodedSignature>
}

pub enum GUIMessage {
    ErrorMessage(String),
    DevicesList(Box<Vec<String>>), // A list of audio devices, received from the microphone thread because CPAL can't be called from the same thread as the GUI under Windows
    NetworkStatus(bool), // Is the network reachable?
    WipeSongHistory,
    MicrophoneRecording,
    MicrophoneVolumePercent(f32),
    SongRecognized(Box<SongRecognizedMessage>)
}

pub enum MicrophoneMessage {
    MicrophoneRecordStart(String), // The argument is the audio device name
    MicrophoneRecordStop,
    ProcessingDone
}

pub enum ProcessingMessage {
    ProcessAudioFile(String),
    ProcessAudioSamples(Box<Vec<i16>>) // Prefer to use heap across threads to avoid stack overflow
}

pub enum HTTPMessage {
    RecognizeSignature(Box<DecodedSignature>)
}
