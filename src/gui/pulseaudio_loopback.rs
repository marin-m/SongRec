use std::process::Command;
use serde::Deserialize;

// Structures useful for passing arguments to the functions below

enum PactlDevicesType {
    SourceOutputs,
    Sources
}

struct PactlItemInfo {
    is_monitor: bool,
    name: String,
    source_index: Option<u64>,
    index: u64
}

// Structures useful for parsing the TSV output from "pactl"
// using the BurntSushi "csv" module

// See here for the TSV format reference:
// - For "sources": https://gitlab.freedesktop.org/pulseaudio/pulseaudio/-/blob/stable-14.x/src/utils/pactl.c#L419
// - For "clients": https://gitlab.freedesktop.org/pulseaudio/pulseaudio/-/blob/stable-14.x/src/utils/pactl.c#L558
// - For "source-outputs": https://gitlab.freedesktop.org/pulseaudio/pulseaudio/-/blob/stable-14.x/src/utils/pactl.c#L755

#[derive(Deserialize)] // Parses the TSV output from "pactl list short sources"
struct PactlSourceTSVInfo {
    pub index: u64,
    pub name: String,
    pub driver_name: String,
    pub sample_spec: String,
    pub state: String
}

#[derive(Deserialize)]
struct PactlClientTSVInfo { // Parses the TSV output from "pactl list short clients"
    pub index: u64,
    pub driver_name: String,
    pub binary_process_name: String
}

#[derive(Deserialize)]
struct PactlSourceOutputTSVInfo { // Parses the TSV output from "pactl list short source-outputs"
    pub index: u64,
    pub source_index: u64,
    pub client_index_or_dash: String, // Should be Option<u64> if not parsed with an external wrapper
    pub driver_name: String,
    pub sample_spec: String
}

/**
 * The struct below handles setting the audio input source of the application
 * to "Monitor of Built-in Analog Audio Stereo" instead of "Built-in Analog Audio
 * Stereo", in order to be able to record from the audio output of the user
 * (e.g YouTube) instead of the microphone, if the host machine uses PulseAudio.
 * 
 * It will check whether executing the following commands is possible, then
 * executing these:
 * 
 *  pactl list short source-outputs (TSV-formatted output for "pacmd list-source-outputs")
 *  pactl list short sources (TSV-formatted output for "pacmd list-sources")
 *  pactl list short clients (TSV-formatted output for "pacmd list-clients")
 * 
 *  pactl move-source-output 26 0 (equivalent with "pacmd")
 *
 * Where "26" and "0" should match the respective source indexes for
 * the "ALSA plug-in [songrec]" source output and the first "monitor"
 * source according to the output of the two first commands.
 * 
 * If PulseAudio or control through "pactl" is not available or
 * possible, the first static method exposed by the structure
 * should let the program know so so that the "Recognize from my speakers
 * instead of microphone" checkbox of the program is not exposed to the
 * user, or is correctly greyed out.
 * 
 * See examples here for spawning subprocess in Rust, capturing the
 * output, checking the status, etc.:
 * https://doc.rust-lang.org/std/process/struct.Command.html#method.output
 * 
 * See here for a reference on TSV or smiliar parsing with the BurntSushi
 * "csv" library:
 * https://stackoverflow.com/a/43903357
 * https://docs.rs/csv/1.1.5/csv/struct.ReaderBuilder.html#method.from_reader
 */

pub struct PulseaudioLoopback;

impl PulseaudioLoopback {
    
    pub fn check_whether_pactl_is_available() -> bool {
        match Command::new("pactl").args(&["list", "short", "source-outputs"]).output() {
            Ok(output) => output.status.success(),
            _ => false // Fail silently if "pactl" is not available at all (as
                // we'll do in the rest of this file), so that we don't display
                // the corresponding checkbox in the interface
        }
    }
    
    fn get_pactl_client_process_name_from_index(client_index: u64) -> Option<String> {
        
        let tsv_output = match Command::new("pactl").args(&["list", "short", "clients"]).output() {
            Ok(output) => output.stdout,
            _ => { return None }
        };
        
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(tsv_output.as_slice());
        
        for result in reader.deserialize() {
            let record: PactlClientTSVInfo = match result {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("Note: Could not parse TSV output from \"pactl list short clients\": {:?}", error);
                    return None
                }
            };
            
            if record.index == client_index {
                return Some(record.binary_process_name);
            }
        }
        
        None
    }
    
    fn get_pactl_devices_info(devices_type: PactlDevicesType) -> Option<Vec<PactlItemInfo>> {
        let source_type = match devices_type {
            PactlDevicesType::SourceOutputs => "source-outputs",
            PactlDevicesType::Sources => "sources"
        };
        
        let tsv_output = match Command::new("pactl").args(&["list", "short", source_type]).output() {
            Ok(output) => output.stdout,
            _ => { return None }
        };
        
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(tsv_output.as_slice());
        
        let mut output_items: Vec<PactlItemInfo> = vec![];
        
        match devices_type {
            PactlDevicesType::SourceOutputs => {
                for result in reader.deserialize() {
                    
                    let record: PactlSourceOutputTSVInfo = match result {
                        Ok(data) => data,
                        Err(error) => {
                            eprintln!("Note: Could not parse TSV output from \"pactl list short {}\": {:?}", source_type, error);
                            return None
                        }
                    };
                    
                    if record.client_index_or_dash != "-" {
                        let client_index = record.client_index_or_dash.parse::<u64>().unwrap();
                        
                        if let Some(process_name) = Self::get_pactl_client_process_name_from_index(client_index)
                        {
                            
                            output_items.push(PactlItemInfo {
                                is_monitor: false,
                                index: record.index,
                                source_index: Some(record.source_index),
                                name: process_name
                            });
                        }
                    }
                }
                
            },
            PactlDevicesType::Sources => {
                for result in reader.deserialize() {
                    let record: PactlSourceTSVInfo = match result {
                        Ok(data) => data,
                        _ => { return None }
                    };

                    output_items.push(PactlItemInfo {
                        is_monitor: record.name.contains(".monitor"),
                        index: record.index,
                        source_index: None,
                        name: record.name
                    });
                }
            }
        };
        
        Some(output_items)
    }
    
    fn get_songrec_source_output_index() -> Option<u64>
    {    
        let source_outputs = match Self::get_pactl_devices_info(PactlDevicesType::SourceOutputs) {
            Some(result) => result,
            _ => { return None }
        };
        
        for source_output in source_outputs.iter() {
            
            if source_output.name == "songrec" {
                return Some(source_output.index);
            }
        }
        
        None
    }
    
    fn get_audio_monitor_source_index() -> Option<u64>
    {
        let sources = match Self::get_pactl_devices_info(PactlDevicesType::Sources) {
            Some(result) => result,
            _ => { return None }
        };
        
        for source in sources.iter() {
            if source.is_monitor {
                return Some(source.index);
            }
        }
        
        None

    }
    
    fn get_audio_non_monitor_source_index() -> Option<u64> {
        let sources = match Self::get_pactl_devices_info(PactlDevicesType::Sources) {
            Some(result) => result,
            _ => { return None }
        };
        
        for source in sources.iter() {
            if !source.is_monitor {
                return Some(source.index);
            }
        }
        
        None
    }
    
    /// This function check whether SongRec is currently plugged towards what
    /// we identity as the default input or output
    
    pub fn get_whether_audio_source_is_known() -> Option<bool> {
        let source_outputs = match Self::get_pactl_devices_info(PactlDevicesType::SourceOutputs) {
            Some(result) => result,
            _ => { return None }
        };
        
        for source_output in source_outputs.iter() {
            if source_output.name == "songrec" {
                if let Some(source_index) = source_output.source_index {
                    if let Some(audio_monitor_source_index) = Self::get_audio_monitor_source_index() {
                        if let Some(audio_non_monitor_source_index) = Self::get_audio_non_monitor_source_index() {
                            return Some(source_index == audio_monitor_source_index || source_index == audio_non_monitor_source_index);
                         }
                    }
                }
            }
        }
        
        None
    }
    
    /// This function check whether SongRec is currently plugged towards what
    /// we identity as the default output (the PulseAudio monitor device)
    
    pub fn get_whether_audio_source_is_monitor() -> Option<bool> {
        let source_outputs = match Self::get_pactl_devices_info(PactlDevicesType::SourceOutputs) {
            Some(result) => result,
            _ => { return None }
        };
        
        for source_output in source_outputs.iter() {
            if source_output.name == "songrec" {
                if let Some(source_index) = source_output.source_index {
                    if let Some(audio_monitor_source_index) = Self::get_audio_monitor_source_index() {
                        return Some(source_index == audio_monitor_source_index);
                    }
                }
            }
        }
        
        None
    }
    
    /// This is used to apply toggling the "Recognize from my speakers instead
    /// of microphone" checkbox of the UI
    
    pub fn set_whether_audio_source_is_monitor(is_monitor: bool) {
        
        let pulseaudio_source = match is_monitor {
            true => Self::get_audio_monitor_source_index(),
            false => Self::get_audio_non_monitor_source_index()
        };
        
        if let Some(songrec_index) = Self::get_songrec_source_output_index() {
            
            if let Some(pulseaudio_source_index) = pulseaudio_source {
            
                Command::new("pactl")
                    .args(&[
                        "move-source-output",
                        &format!("{}", songrec_index),
                        &format!("{}", pulseaudio_source_index)
                    ]).status().unwrap();
            }
        }
    }
}
