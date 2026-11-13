use log::{debug, error};
use std::io::BufReader;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use std::path::{Path, PathBuf};
use std::process::Command;

/// This function used to decode a file with FFMpeg, if it is installed on
/// the system, in the case where Rodio can't decode the concerned format
/// (for example with .WMA, .M4A, etc.).
pub fn decode_with_ffmpeg(file_path: &Path) -> Option<rodio::Decoder<BufReader<std::fs::File>>> {
    // Find the path for FFMpeg, in the case where it is installed

    // Paths are kept as `Path`, as the directory of SongRec may not be UTF-8
    let mut possible_ffmpeg_paths: Vec<PathBuf> = vec!["ffmpeg".into(), "ffmpeg.exe".into()];

    if let Ok(mut current_dir_ffmpeg_path) = std::env::current_exe() {
        current_dir_ffmpeg_path.pop();
        current_dir_ffmpeg_path.push("ffmpeg.exe");
        possible_ffmpeg_paths.push(current_dir_ffmpeg_path);
    }

    let mut actual_ffmpeg_path: Option<PathBuf> = None;

    for possible_path in possible_ffmpeg_paths {
        // Use .output() to execute the subprocess testing for FFMpeg
        // presence and correct execution, so that it does not pollute
        // the standard or error output in any way

        let mut command = Command::new(&possible_path);
        let command = command.arg("-version");

        #[cfg(windows)]
        let command = command.creation_flags(0x00000008); // Set "CREATE_NO_WINDOW" on Windows

        if let Ok(process) = command.output()
            && process.status.success()
        {
            actual_ffmpeg_path = Some(possible_path);
            break;
        }
    }

    // If FFMpeg is available, use it to convert the input file
    // from whichever format to a .WAV (because Rodio has its
    // decoding support limited to .WAV, .FLAC, .OGG, .MP3, which
    // makes that .MP4/.AAC, .OPUS or .WMA are not supported
    // by default)

    if let Some(ffmpeg_path) = actual_ffmpeg_path {
        // Create a sink file for FFMpeg

        let sink_file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();

        let sink_file_path = sink_file.into_temp_path();

        // Try to convert the input video or audio file to a standard
        // .WAV s16le PCM file using FFMpeg, and pass it to Rodio
        // later in the case where it succeeded

        let mut command = ffmpeg_command(&ffmpeg_path, file_path, &sink_file_path);
        let command = &mut command;

        debug!("Spawning ffmpeg: {:?}", command);

        // Set "CREATE_NO_WINDOW" on Windows, see
        // https://stackoverflow.com/a/60958956/662399
        #[cfg(windows)]
        let command = command.creation_flags(0x00000008);

        match command.output() {
            Ok(process) => {
                if process.status.success() {
                    let decoder = std::fs::File::open(&sink_file_path)
                        .map_err(|error| error.to_string())
                        .and_then(|file| {
                            rodio::Decoder::new(BufReader::new(file))
                                .map_err(|error| error.to_string())
                        });
                    match decoder {
                        Ok(decoder) => return Some(decoder),
                        Err(error) => error!("Could not decode the output of ffmpeg: {}", error),
                    }
                } else {
                    error!("ffmpeg returned an error: {:?}", process.status);
                }
            }
            Err(err) => {
                error!("Could not spawn ffmpeg: {:?}", err);
            }
        }
    }

    None
}

/// The FFMpeg command converting FILE_PATH to a .WAV file at SINK_PATH. Paths
/// are passed as they are, as they may not be UTF-8 on Linux
fn ffmpeg_command(ffmpeg_path: &Path, file_path: &Path, sink_path: &Path) -> Command {
    let mut command = Command::new(ffmpeg_path);
    command.args(["-y", "-i"]).arg(file_path).arg(sink_path);
    command
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn the_ffmpeg_command_accepts_paths_that_are_not_utf8() {
        // A temporary directory such as TMPDIR=/tmp/caf\xe9 on Linux
        let sink_path = Path::new(OsStr::from_bytes(b"/tmp/caf\xe9/sink.wav"));
        let ffmpeg_path = Path::new(OsStr::from_bytes(b"/opt/caf\xe9/ffmpeg"));

        let input_path = Path::new(OsStr::from_bytes(b"/home/caf\xe9/input.m4a"));

        let command = ffmpeg_command(ffmpeg_path, input_path, sink_path);

        assert_eq!(command.get_program(), ffmpeg_path.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                OsStr::new("-y"),
                OsStr::new("-i"),
                input_path.as_os_str(),
                sink_path.as_os_str()
            ]
        );
    }
}
