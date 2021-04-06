use std::sync::mpsc;
use std::error::Error;
use serde_json::Value;

use crate::gui::thread_messages::*;

use crate::fingerprinting::signature_format::DecodedSignature;
use crate::fingerprinting::communication::{recognize_song_from_signature, obtain_raw_cover_image};

fn try_recognize_song(signature: DecodedSignature) -> Result<SongRecognizedMessage, Box<dyn Error>> {
    let json_object = recognize_song_from_signature(&signature)?;
    
    let mut album_name: Option<String> = None;
    
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
                                        break;
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
            _ => { return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No match for this song"))) }
        },
        album_name: album_name,
        song_name: match &json_object["track"]["title"] {
            Value::String(string) => string.to_string(),
            _ => { return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No match for this song"))) }
        },
        cover_image: match &json_object["track"]["images"]["coverart"] {
            Value::String(string) => Some(obtain_raw_cover_image(string)?),
            _ => None
        },
        signature: Box::new(signature)
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
                            "No match for this song" => {
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
