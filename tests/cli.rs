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
