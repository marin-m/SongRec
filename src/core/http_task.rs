use gettextrs::gettext;
use serde_json::Value;
use soup::prelude::SessionExt;
use std::error::Error;

use crate::core::thread_messages::*;

use crate::core::fingerprinting::communication::{
    obtain_raw_cover_image, recognize_song_from_signature,
};
use crate::core::fingerprinting::signature_format::DecodedSignature;

async fn try_recognize_song(
    session: &soup::Session,
    signature: DecodedSignature,
) -> Result<SongRecognizedMessage, Box<dyn Error>> {
    let json_object = recognize_song_from_signature(session, &signature).await?;

    let mut album_name: Option<String> = None;
    let mut release_year: Option<String> = None;

    // Sometimes the idea of trying to write functional poetry hurts

    if let Value::Array(sections) = &json_object["track"]["sections"] {
        for section in sections {
            if let Value::String(string) = &section["type"]
                && string == "SONG"
            {
                if let Value::Array(metadata) = &section["metadata"] {
                    for metadatum in metadata {
                        if let Value::String(title) = &metadatum["title"] {
                            if title == "Album"
                                && let Value::String(text) = &metadatum["text"]
                            {
                                album_name = Some(text.to_string());
                            } else if title == "Released"
                                && let Value::String(text) = &metadatum["text"]
                            {
                                release_year = Some(text.to_string());
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    Ok(SongRecognizedMessage {
        artist_name: match &json_object["track"]["subtitle"] {
            Value::String(string) => string.to_string(),
            _ => {
                return Err(Box::new(std::io::Error::other(
                    gettext("No match for this song").as_str(),
                )));
            }
        },
        album_name,
        song_name: match &json_object["track"]["title"] {
            Value::String(string) => string.to_string(),
            _ => {
                return Err(Box::new(std::io::Error::other(
                    gettext("No match for this song").as_str(),
                )));
            }
        },
        cover_image: obtain_cover_image(session, &json_object).await,
        track_key: match &json_object["track"]["key"] {
            Value::String(string) => string.to_string(),
            _ => {
                return Err(Box::new(std::io::Error::other(
                    gettext("No match for this song").as_str(),
                )));
            }
        },
        release_year,
        genre: match &json_object["track"]["genres"]["primary"] {
            Value::String(string) => Some(string.to_string()),
            _ => None,
        },
        shazam_json: serde_json::to_string(&json_object).unwrap(),
    })
}

/// Download the cover art of a recognized song, if it has one. The song was
/// matched already, so a cover that can't be downloaded is logged and left
/// out rather than turning the match into an error.
async fn obtain_cover_image(session: &soup::Session, json_object: &Value) -> Option<Vec<u8>> {
    let Value::String(url) = &json_object["track"]["images"]["coverart"] else {
        return None;
    };
    match obtain_raw_cover_image(session, url).await {
        Ok(cover_image) => Some(cover_image),
        Err(error) => {
            log::warn!("Could not download the cover art from {}: {}", url, error);
            None
        }
    }
}

pub async fn http_task(
    http_rx: async_channel::Receiver<HTTPMessage>,
    gui_tx: async_channel::Sender<GUIMessage>,
    microphone_tx: async_channel::Sender<MicrophoneMessage>,
) {
    let session = soup::Session::new();
    session.set_timeout(20);
    session.set_idle_timeout(2);

    while let Ok(message) = http_rx.recv().await {
        // XX USE SOUP3 CF. https://github.com/marin-m/SongRec/issues/223
        match message {
            HTTPMessage::RecognizeSignature(signature) => {
                match try_recognize_song(&session, *signature).await {
                    Ok(recognized_song) => {
                        gui_tx
                            .try_send(GUIMessage::SongRecognized(Box::new(recognized_song)))
                            .unwrap();
                        gui_tx.try_send(GUIMessage::NetworkStatus(true)).unwrap();
                        gui_tx.try_send(GUIMessage::RateLimitState(false)).unwrap();
                    }
                    Err(error) => match error.to_string().as_str() {
                        a if a == gettext("No match for this song") => {
                            gui_tx
                                .try_send(GUIMessage::ErrorMessage(error.to_string()))
                                .unwrap();
                            gui_tx.try_send(GUIMessage::NetworkStatus(true)).unwrap();
                            gui_tx.try_send(GUIMessage::RateLimitState(false)).unwrap();
                        }
                        a if a == gettext("Your IP has been rate-limited") => {
                            gui_tx.try_send(GUIMessage::RateLimitState(true)).unwrap();
                        }
                        _ => {
                            log::error!("Network reach error: {:?}", error);
                            gui_tx.try_send(GUIMessage::NetworkStatus(false)).unwrap();
                        }
                    },
                };

                microphone_tx
                    .try_send(MicrophoneMessage::ProcessingDone)
                    .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Serve one HTTP response with `status` and `body` on a local port, and
    /// return its URL
    fn serve_once(status: &'static str, body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/cover.jpg", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        });
        url
    }

    /// A URL on a local port that nothing listens on
    fn refused_url() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}/cover.jpg", listener.local_addr().unwrap())
    }

    fn cover_for(url: Option<String>) -> Option<Vec<u8>> {
        let track = match url {
            Some(url) => json!({"track": {"images": {"coverart": url}}}),
            None => json!({"track": {}}),
        };
        // A main context of its own, so that tests running in parallel don't
        // compete for the default one
        let context = glib::MainContext::new();
        context
            .with_thread_default(|| {
                let session = soup::Session::new();
                session.set_timeout(5);
                context.block_on(obtain_cover_image(&session, &track))
            })
            .unwrap()
    }

    #[test]
    fn a_cover_that_cannot_be_downloaded_is_left_out() {
        assert_eq!(
            cover_for(Some(serve_once("200 OK", b"image"))),
            Some(b"image".to_vec())
        );
        assert_eq!(cover_for(None), None);
        assert_eq!(cover_for(Some(refused_url())), None);
        assert_eq!(
            cover_for(Some(serve_once("404 Not Found", b"<html>"))),
            None
        );
        assert_eq!(cover_for(Some("not a url".to_string())), None);
    }
}
