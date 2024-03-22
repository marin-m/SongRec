use crate::fingerprinting::user_agent;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use std::error::Error;
use std::sync::OnceLock;
use std::time::Duration;
use unicode_normalization::UnicodeNormalization;

pub struct LyricSearchInfo {
    pub artist_name: String,
    pub song_name: String,
}

pub fn fetch_genius_lyrics(info: &LyricSearchInfo) -> Result<String, Box<dyn Error>> {
    static RE_PAREN: OnceLock<Regex> = OnceLock::new();
    static RE_FEAT_PAREN: OnceLock<Regex> = OnceLock::new();
    static RE_FEAT_BRACK: OnceLock<Regex> = OnceLock::new();
    static RE_TAG_START: OnceLock<Regex> = OnceLock::new();
    static RE_TAG_END: OnceLock<Regex> = OnceLock::new();

    let re_paren = RE_PAREN.get_or_init(|| Regex::new(r#"\(.*?\)"#).unwrap());
    let re_feat_paren = RE_FEAT_PAREN.get_or_init(|| Regex::new(r#"\(.*?(?:feat\.|ft\.).*?\)"#).unwrap());
    let re_feat_brack = RE_FEAT_BRACK.get_or_init(|| Regex::new(r#"\[.*?(?:feat\.|ft\.).*?\]"#).unwrap());
    let re_tag_start = RE_TAG_START.get_or_init(|| Regex::new(r#"<.+?>"#).unwrap());
    let re_tag_end = RE_TAG_END.get_or_init(|| Regex::new(r#"<.+?/>"#).unwrap());

    // Remove parens/brackets with feat. or ft. in them e.g. Song Title (feat. XXX).
    let song = re_feat_paren.replace_all(&info.song_name, "");
    let song = re_feat_brack.replace_all(&song, "");

    let url = make_url(&format!("{}-{}", info.artist_name, song));

    let html = match fetch_lyrics_html(&url)? {
        Some(lyrics) => Some(lyrics),
        None => {
            // Try one more time.
            if let Some(index) = info.artist_name.find(|c| c == ',' || c == '&') {
                // If the artist name contains a comma or a & what comes after is probably another
                // artist name so we remove that as genius doesn't put featuring artists in the url.
                let artist_name = &info.artist_name[..index];
                let url = make_url(&format!("{}-{}", artist_name, song));
                fetch_lyrics_html(&url)?
            } else if song.contains('(') {
                // Removing all parenthesis from the song title sometimes works.
                let song = re_paren.replace_all(&song, "");
                let url = make_url(&format!("{}-{}", info.artist_name, song));
                fetch_lyrics_html(&url)?
            } else {
                None
            }
        }
    }
    .ok_or("lyrics not found")?;

    // Reduce the amount of text we need to look at to find the lyrics. Lyrics are in between
    // the <div id="lyrics-root> and <div class="LyricsFooter"> tags.
    let root = &html[html
        .find("id=\"lyrics-root\"")
        .ok_or("lyrics-root not found")?
        ..html
            .find("class=\"LyricsFooter")
            .ok_or("LyricsFooter not found")?];

    let mut lyrics = String::new();

    for container in root.split("data-lyrics-container=\"true\"").skip(1) {
        let container = container.trim().replace("<br/>", "\n");

        for line in container.lines() {
            // Remove all opening and closing HTML tags.
            let replaced = re_tag_start.replace_all(line, "").to_string();
            let replaced = re_tag_end.replace_all(&replaced, "").to_string();
            // Clean up some remaining garbage.
            let replaced = replaced.replace("<div", "");
            let replaced = replaced.split("\">").last().unwrap();

            // Exclude annotation lines.
            if replaced.get(0..1) != Some("[") {
                lyrics.push_str(&html_escape::decode_html_entities(&replaced));
                lyrics.push('\n');
            }
        }
    }
    Ok(lyrics.trim().to_string())
}

fn fetch_lyrics_html(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", user_agent::random().parse()?);
    headers.insert("Content-Language", "en_US".parse()?);

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url)
        .timeout(Duration::from_secs(20))
        .headers(headers)
        .send()?;

    if response.status() == StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        Ok(Some(response.text()?))
    }
}

fn make_url(query: &str) -> String {
    // Convert accents and umlauts etc. to plain ascii as otherwise the lyric lookup fails.
    let query = query.nfd().filter(char::is_ascii).collect::<String>();

    // Other replacements.
    let query = query.replace('&', "and");
    let query = query.replace('_', "-");

    let lower = query.to_lowercase();
    let mut chars = lower.chars();
    let mut mangled = String::new();
    let Some(first) = chars.next() else {
        return mangled;
    };
    mangled.extend(first.to_uppercase());

    let mut skip = false;
    for char in chars {
        if char.is_whitespace() || char == '-' {
            if !skip {
                mangled.push('-');
                skip = true;
            }
        } else if char.is_ascii_alphanumeric() {
            mangled.push(char);
            skip = false;
        }
    }
    let last = mangled.pop().unwrap();
    if last != '-' {
        mangled.push(last);
    }
    format!("https://genius.com/{mangled}-lyrics")
}
