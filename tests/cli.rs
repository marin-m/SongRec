//! Run the songrec binary on inputs that cannot be recognized and check that
//! it reports an error and exits, instead of panicking and hanging.

use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn run_songrec(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_songrec"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "songrec {:?} was still running after 20 seconds, stderr: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.wait_with_output().unwrap()
}

fn assert_reports_an_error(args: &[&str]) {
    let output = run_songrec(args);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "songrec {:?} succeeded", args);
    assert!(
        !stderr.contains("panicked"),
        "songrec {:?} panicked: {}",
        args,
        stderr
    );
    assert!(
        output.stdout.is_empty(),
        "songrec {:?} wrote to stdout",
        args
    );
}

fn missing_file() -> String {
    std::env::temp_dir()
        .join(format!("songrec-missing-{}.mp3", std::process::id()))
        .to_string_lossy()
        .into_owned()
}

#[test]
fn recognize_reports_a_missing_file() {
    assert_reports_an_error(&["recognize", &missing_file()]);
}

#[test]
fn recognize_reports_a_file_that_is_not_audio() {
    let path = std::env::temp_dir().join(format!("songrec-not-audio-{}.mp3", std::process::id()));
    std::fs::write(&path, "not audio").unwrap();
    assert_reports_an_error(&["recognize", &path.to_string_lossy()]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn recognize_reports_an_unknown_audio_device() {
    assert_reports_an_error(&["recognize", "--audio-device", "songrec-no-such-device"]);
}

#[test]
fn audio_file_to_recognized_song_reports_a_missing_file() {
    assert_reports_an_error(&["audio-file-to-recognized-song", &missing_file()]);
}

#[test]
fn audio_file_to_fingerprint_reports_a_missing_file() {
    assert_reports_an_error(&["audio-file-to-fingerprint", &missing_file()]);
}

#[test]
fn fingerprint_to_recognized_song_reports_a_string_that_is_not_a_fingerprint() {
    assert_reports_an_error(&["fingerprint-to-recognized-song", "not a fingerprint"]);
}

#[test]
fn fingerprint_to_recognized_song_reports_an_invalid_base64_fingerprint() {
    assert_reports_an_error(&[
        "fingerprint-to-recognized-song",
        "data:audio/vnd.shazam.sig;base64,not base64!",
    ]);
}

#[test]
fn fingerprint_to_recognized_song_reports_a_fingerprint_that_is_not_a_signature() {
    assert_reports_an_error(&[
        "fingerprint-to-recognized-song",
        "data:audio/vnd.shazam.sig;base64,AAAA",
    ]);
}

/// A 3-second mono 16 kHz WAV file of two alternating tones
fn tone_wav() -> Vec<u8> {
    let samples: Vec<i16> = (0..3 * 16000)
        .map(|i| {
            let frequency = if i / 4000 % 2 == 0 { 440.0 } else { 660.0 };
            let t = i as f32 / 16000.0;
            (8000.0 * (2.0 * std::f32::consts::PI * frequency * t).sin()) as i16
        })
        .collect();
    let data_size = (samples.len() * 2) as u32;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&32000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// A fresh, empty directory for songrec's temporary files
fn empty_temp_dir(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("songrec-cli-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn assert_is_empty(directory: &std::path::Path, args: &[&str]) {
    let left: Vec<_> = std::fs::read_dir(directory).unwrap().collect();
    assert!(left.is_empty(), "songrec {:?} left {:?}", args, left);
}

/// Run songrec with STDIN as its standard input and TEMP_DIR as its
/// temporary directory
fn run_songrec_with_stdin(args: &[&str], stdin: &[u8], temp_dir: &std::path::Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_songrec"))
        .args(args)
        .env("TMPDIR", temp_dir)
        .env("TMP", temp_dir)
        .env("TEMP", temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Write from another thread, so that a songrec that doesn't read its
    // standard input fails the test instead of blocking it
    let mut child_stdin = child.stdin.take().unwrap();
    let stdin = stdin.to_vec();
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        let _ = child_stdin.write_all(&stdin);
    });

    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("songrec {:?} was still running after 20 seconds", args);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    writer.join().unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn audio_file_to_fingerprint_reads_the_standard_input_for_a_dash() {
    let wav = tone_wav();
    let path = std::env::temp_dir().join(format!("songrec-tone-{}.wav", std::process::id()));
    std::fs::write(&path, &wav).unwrap();
    let from_file = run_songrec(&["audio-file-to-fingerprint", &path.to_string_lossy()]);
    let _ = std::fs::remove_file(&path);
    assert!(from_file.status.success());

    let temp_dir = empty_temp_dir("fingerprint");
    let args = ["audio-file-to-fingerprint", "-"];
    let from_stdin = run_songrec_with_stdin(&args, &wav, &temp_dir);

    assert!(
        from_stdin.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&from_stdin.stderr)
    );
    assert!(
        from_stdin
            .stdout
            .starts_with(b"data:audio/vnd.shazam.sig;base64,")
    );
    assert_eq!(from_stdin.stdout, from_file.stdout);
    // The copy of the standard input is deleted
    assert_is_empty(&temp_dir, &args);
    std::fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn reading_the_standard_input_reports_an_input_that_is_not_audio() {
    for args in [
        &["recognize", "-"][..],
        &["audio-file-to-recognized-song", "-"],
        &["audio-file-to-fingerprint", "-"],
    ] {
        for stdin in [&b""[..], b"not audio"] {
            let temp_dir = empty_temp_dir("not-audio");
            let output = run_songrec_with_stdin(args, stdin, &temp_dir);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "songrec {:?} succeeded", args);
            assert!(
                !stderr.contains("panicked"),
                "songrec {:?}: {}",
                args,
                stderr
            );
            assert!(
                output.stdout.is_empty(),
                "songrec {:?} wrote to stdout",
                args
            );
            // The error is about the audio, not about a file named "-"
            assert!(
                !stderr.contains("No such file") && !stderr.contains("os error 2"),
                "songrec {:?} looked for a file named -: {}",
                args,
                stderr
            );
            // The copy of the standard input is deleted, even on an error
            assert_is_empty(&temp_dir, args);
            std::fs::remove_dir_all(temp_dir).unwrap();
        }
    }
}
