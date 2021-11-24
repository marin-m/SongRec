use std::sync::mpsc;
use std::error::Error;
use gettextrs::gettext;
use regex::Regex;
use serde_json::{Value, to_string_pretty};

use crate::core::thread_messages::*;

use crate::fingerprinting::signature_format::DecodedSignature;
use crate::fingerprinting::communication::{recognize_song_from_signature, obtain_raw_cover_image};

fn try_recognize_song(signature: DecodedSignature) -> Result<SongRecognizedMessage, Box<dyn Error>> {
    let json_object = recognize_song_from_signature(&signature)?;
    
    let mut album_name: Option<String> = None;
    let mut release_year: Option<String> = None;
    
    // Sometimes the idea of trying to write functional poetry hurts
    
    if let Value::Array(sections) = &json_object["track"]["sections"] {
        for section in sections {
            if let Value::String(string) = &section["type"] {
                if string == "SONG" {
                    if let Value::Array(metadata) = &section["metadata"] {
                        for metadatum in metadata {
                            if let Value::String(title) = &metadatum["title"] {
                                if title == "Album" {
                                    if let Value::String(text) = &metadatum["text"] {
                                        album_name = Some(text.to_string());
                                    }
                                }
                                else if title == "Released" {
                                    if let Value::String(text) = &metadatum["text"] {
                                        release_year = Some(text.to_string());
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    
    Ok(SongRecognizedMessage {
        artist_name: match &json_object["track"]["subtitle"] {
            Value::String(string) => string.to_string(),
            _ => { return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, gettext("No match for this song").as_str()))) }
        },
        album_name: album_name,
        song_name: match &json_object["track"]["title"] {
            Value::String(string) => string.to_string(),
            _ => { return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, gettext("No match for this song").as_str()))) }
        },
        cover_image: match &json_object["track"]["images"]["coverart"] {
            Value::String(string) => Some(obtain_raw_cover_image(string)?),
            _ => None
        },
        signature: Box::new(signature),
        track_key: match &json_object["track"]["key"] {
            Value::String(string) => string.to_string(),
            _ => { return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, gettext("No match for this song").as_str()))) }
        },
        release_year: release_year,
        genre: match &json_object["track"]["genres"]["primary"] {
            Value::String(string) => Some(string.to_string()),
            _ => None
        },
        shazam_json: Regex::new("\n *").unwrap().replace_all(&
            Regex::new("([,:])\n *").unwrap().replace_all(&
                to_string_pretty(&json_object).unwrap(), "$1 ").into_owned(),
            "").into_owned()
    })
}

pub fn http_thread(http_rx: mpsc::Receiver<HTTPMessage>, gui_tx: glib::Sender<GUIMessage>, microphone_tx: mpsc::Sender<MicrophoneMessage>) {
    
    for message in http_rx.iter() {
        match message {
            HTTPMessage::RecognizeSignature(signature) => {
                match try_recognize_song(*signature) {
                    Ok(recognized_song) => {
                        gui_tx.send(GUIMessage::SongRecognized(Box::new(recognized_song))).unwrap();
                        gui_tx.send(GUIMessage::NetworkStatus(true)).unwrap();
                    },
                    Err(error) => {
                        match error.to_string().as_str() {
                            a if a == gettext("No match for this song") => {
                                gui_tx.send(GUIMessage::ErrorMessage(error.to_string())).unwrap();
                                gui_tx.send(GUIMessage::NetworkStatus(true)).unwrap();
                            }
                            _ => {
                                gui_tx.send(GUIMessage::NetworkStatus(false)).unwrap();
                            }
                        }
                    }
                };
                
                microphone_tx.send(MicrophoneMessage::ProcessingDone).unwrap();
            }
        }
    }

}
