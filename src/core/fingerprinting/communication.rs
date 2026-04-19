use gettextrs::gettext;
use glib::source::Priority;
use log::{debug, error, trace};
use rand::prelude::IndexedRandom;
use serde_json::{json, Value};
use soup::prelude::SessionExt;
use std::error::Error;
use std::time::SystemTime;
use uuid::Uuid;

use crate::core::fingerprinting::signature_format::DecodedSignature;
use crate::core::fingerprinting::user_agent::USER_AGENTS;

fn log_request(message: &soup::Message, post_data: &str) {
    if let Some(headers) = message.request_headers() {
        let mut full_headers: Vec<(String, String)> = vec![];
        headers.foreach(|key, value| {
            full_headers.push((key.to_string(), value.to_string()));
        });
        trace!(
            "Sending request to Shazam: {:?} {:?} {:?}",
            message.uri().unwrap().to_str(),
            full_headers,
            post_data
        );
    } else {
        trace!(
            "Sending request to Shazam: {:?}",
            message.uri().unwrap().to_str()
        );
    }
}

fn log_response(message: &soup::Message, response: &str) {
    if let Some(headers) = message.response_headers() {
        let mut full_headers: Vec<(String, String)> = vec![];
        headers.foreach(|key, value| {
            full_headers.push((key.to_string(), value.to_string()));
        });
        let format_string = format!(
            "Received response from for Shazam for {}: {:?} {:?} {} {:?} {:?}",
            message.uri().unwrap().to_str(),
            message.status_code(),
            message.reason_phrase(),
            match message.http_version() {
                soup::HTTPVersion::Http10 => "HTTP/1.0".to_string(),
                soup::HTTPVersion::Http11 => "HTTP/1.1".to_string(),
                soup::HTTPVersion::Http20 => "HTTP/2.0".to_string(),
                _ => format!("{:?}", message.http_version()),
            },
            full_headers,
            response
        );
        if message.status_code() != 200 {
            error!("{}", format_string);
        } else {
            debug!("{}", format_string);
        }
    } else {
        trace!("Received response from Shazam: {:?}", message.status_code());
    }
}

pub async fn recognize_song_from_signature(
    session: &soup::Session,
    signature: &DecodedSignature,
) -> Result<Value, Box<dyn Error>> {
    session.set_user_agent(USER_AGENTS.choose(&mut rand::rng()).unwrap());

    let timestamp_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();

    let post_data = json!({
        "geolocation": {
            "altitude": 300,
            "latitude": 45,
            "longitude": 2
        },
        "signature": {
            "samplems": (signature.number_samples as f32 / signature.sample_rate_hz as f32 * 1000.) as u32,
            "timestamp": timestamp_ms as u32,
            "uri": signature.encode_to_uri()?
        },
        "timestamp": timestamp_ms as u32,
        "timezone": "Europe/Paris"
    }).to_string();

    let uuid_1 = Uuid::new_v4().hyphenated().to_string().to_uppercase();
    let uuid_2 = Uuid::new_v4().hyphenated().to_string();

    let url = format!(
        "https://amp.shazam.com/discovery/v5/en/US/android/-/tag/{}/{}\
?sync=true\
&webv3=true\
&sampling=true\
&connected=\
&shazamapiversion=v3\
&sharehub=true\
&video=v3",
        uuid_1, uuid_2
    );

    let message = soup::Message::from_encoded_form("POST", &url, post_data.clone().into())?;

    let headers = message.request_headers().unwrap();
    headers.append("Content-Language", "en_US");
    headers.set_content_type(Some("application/json"), None);

    log_request(&message, &post_data);

    let response = session
        .send_and_read_future(&message, Priority::DEFAULT)
        .await?;

    let decoded_resp = String::from_utf8_lossy(&response[..]);

    log_response(&message, &decoded_resp);

    if message.status_code() == 429 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::QuotaExceeded,
            gettext("Your IP has been rate-limited").as_str(),
        )));
    }

    Ok(serde_json::from_slice(&response[..])?)
}

pub async fn obtain_raw_cover_image(
    session: soup::Session,
    url: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let message = soup::Message::new("GET", url)?;
    session.set_user_agent(USER_AGENTS.choose(&mut rand::rng()).unwrap());
    let headers = message.request_headers().unwrap();
    headers.append("Content-Language", "en_US");

    log_request(&message, "");

    let response = session
        .send_and_read_future(&message, Priority::DEFAULT)
        .await?;

    let resp_header = format!("{:?}...", &response[..32]);
    log_response(&message, &resp_header);

    Ok(response[..].to_vec())
}
