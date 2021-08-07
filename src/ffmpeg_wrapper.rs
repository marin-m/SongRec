
use std::io::BufReader;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use std::process::Command;

/// This function used to decode a file with FFMpeg, if it is installed on
/// the system, in the case where Rodio can't decode the concerned format
/// (for example with .WMA, .M4A, etc.).

pub fn decode_with_ffmpeg(file_path: &str) -> Option<rodio::Decoder<BufReader<std::fs::File>>> {

    // Find the path for FFMpeg, in the case where it is installed
    
    let mut possible_ffmpeg_paths: Vec<&str> = vec!["ffmpeg", "ffmpeg.exe"];
    
    let mut current_dir_ffmpeg_path = std::env::current_exe().unwrap();
    current_dir_ffmpeg_path.pop();
    current_dir_ffmpeg_path.push("ffmpeg.exe");
    
    possible_ffmpeg_paths.push(current_dir_ffmpeg_path.to_str().unwrap());
    
    let mut actual_ffmpeg_path: Option<&str> = None;
    
    for possible_path in possible_ffmpeg_paths {
        
        // Use .output() to execute the subprocess testing for FFMpeg
        // presence and correct execution, so that it does not pollute
        // the standard or error output in any way
        
        let mut command = Command::new(possible_path);
        let command = command.arg("-version");
        
        #[cfg(windows)]
        let command = command.creation_flags(0x00000008); // Set "CREATE_NO_WINDOW" on Windows
        
        if let Ok(process) = command.output() {
            if process.status.success() {
                actual_ffmpeg_path = Some(possible_path);
                break;
            }
        }
        
    }
    
    // If FFMpeg is available, use it to convert the input file
    // from whichever format to a .WAV (because Rodio has its
    // decoding support limited to .WAV, .FLAC, .OGG, .MP3, which
    // makes that .MP4/.AAC, .OPUS or .WMA are not supported, and
    // Rodio's minimp3 .MP3 decoder seems to crash on Windows anyway)
    
    if let Some(ffmpeg_path) = actual_ffmpeg_path {
        
        // Create a sink file for FFMpeg
        
        let sink_file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        
        let sink_file_path = sink_file.into_temp_path();
        
        // Try to convert the input video or audio file to a standard
        // .WAV s16le PCM file using FFMpeg, and pass it to Rodio
        // later in the case where it succeeded
        
        let mut command = Command::new(ffmpeg_path);
        
        let command = command.args(&["-y", "-i", file_path,
            sink_file_path.to_str().unwrap()]);
        
        // Set "CREATE_NO_WINDOW" on Windows, see
        // https://stackoverflow.com/a/60958956/662399
        #[cfg(windows)]
        let command = command.creation_flags(0x00000008);
        
        if let Ok(process) = command.output() {
            
            if process.status.success() {
                return Some(rodio::Decoder::new(
                    BufReader::new(
                        std::fs::File::open(
                            sink_file_path.to_str().unwrap()
                        ).unwrap()
                    )
                ).unwrap());
            }
        }
        
    }
    
    None
}
