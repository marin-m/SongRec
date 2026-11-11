use base64::Engine;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;
use gettextrs::gettext;
use std::error::Error;
use std::io::{Cursor, Seek, SeekFrom, Write};

const DATA_URI_PREFIX: &str = "data:audio/vnd.shazam.sig;base64,";

pub struct FrequencyPeak {
    pub fft_pass_number: u32,
    pub peak_magnitude: u16,
    pub corrected_peak_frequency_bin: u16,
}

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum FrequencyBand {
    _250_520 = 0,
    _520_1450 = 1,
    _1450_3500 = 2,
    _3500_5500 = 3,
}

struct RawSignatureHeader {
    magic1: u32,                                  // Fixed 0xcafe2580 - 80 25 fe ca
    crc32: u32, // CRC-32 for all of the following (so excluding these first 8 bytes)
    size_minus_header: u32, // Total size of the message, minus the size of the current header (which is 48 bytes)
    magic2: u32,            // Fixed 0x94119c00 - 00 9c 11 94
    _void1: [u32; 3],       // Void
    shifted_sample_rate_id: u32, // A member of SampleRate (usually 3 for 16000 Hz), left-shifted by 27 (usually giving 0x18000000 - 00 00 00 18)
    _void2: [u32; 2],            // Void, or maybe used only in "rolling window" mode?
    number_samples_plus_divided_sample_rate: u32, // int(number_of_samples + sample_rate * 0.24) - As the sample rate is known thanks to the field above, it can be inferred and substracted so that we obtain the number of samples, and from the number of samples and sample rate we can obtain the length of the recording
    _fixed_value: u32, // Calculated as ((15 << 19) + 0x40000) - 0x7c0000 or 00 00 7c 00 - seems pretty constant, may be different in the "SigType.STREAMING" mode
}

pub struct DecodedSignature {
    pub sample_rate_hz: u32,
    pub number_samples: u32,
    pub frequency_band_to_sound_peaks: [Vec<FrequencyPeak>; 4],
}

fn invalid_signature(reason: &str) -> Box<dyn Error> {
    format!("{} ({reason})", gettext("Invalid Shazam signature")).into()
}

impl DecodedSignature {
    pub fn decode_from_binary(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        // A signature comes from the command line or a file, so every check
        // returns an error instead of panicking.
        if data.len() <= 48 + 8 {
            return Err(invalid_signature("too short"));
        }

        let mut cursor = Cursor::new(data);

        let header = RawSignatureHeader {
            magic1: cursor.read_u32::<LittleEndian>()?,
            crc32: cursor.read_u32::<LittleEndian>()?,
            size_minus_header: cursor.read_u32::<LittleEndian>()?,
            magic2: cursor.read_u32::<LittleEndian>()?,
            _void1: [
                cursor.read_u32::<LittleEndian>()?,
                cursor.read_u32::<LittleEndian>()?,
                cursor.read_u32::<LittleEndian>()?,
            ],
            shifted_sample_rate_id: cursor.read_u32::<LittleEndian>()?,
            _void2: [
                cursor.read_u32::<LittleEndian>()?,
                cursor.read_u32::<LittleEndian>()?,
            ],
            number_samples_plus_divided_sample_rate: cursor.read_u32::<LittleEndian>()?,
            _fixed_value: cursor.read_u32::<LittleEndian>()?,
        };

        let mut hasher = Hasher::new();
        hasher.update(&data[8..]);
        if header.magic1 != 0xcafe2580 || header.magic2 != 0x94119c00 {
            return Err(invalid_signature("wrong magic number"));
        }
        if header.size_minus_header as usize != data.len() - 48 {
            return Err(invalid_signature("wrong size"));
        }
        if header.crc32 != hasher.finalize() {
            return Err(invalid_signature("wrong checksum"));
        }

        let sample_rate_hz: u32 = match header.shifted_sample_rate_id >> 27 {
            1 => 8000,
            2 => 11025,
            3 => 16000,
            4 => 32000,
            5 => 44100,
            6 => 48000,
            _ => {
                return Err(gettext("Invalid sample rate in decoded Shazam packet").into());
            }
        };

        let number_samples: u32 = header
            .number_samples_plus_divided_sample_rate
            .checked_sub((sample_rate_hz as f32 * 0.24) as u32)
            .ok_or_else(|| invalid_signature("wrong number of samples"))?;

        // Read the type-length-value sequence that follows the header

        // The first chunk is fixed and has no value, but instead just repeats
        // the length of the message size minus the header:

        if cursor.read_u32::<LittleEndian>()? != 0x40000000
            || cursor.read_u32::<LittleEndian>()? as usize != data.len() - 48
        {
            return Err(invalid_signature("wrong first chunk"));
        }

        // Then, lists of frequency peaks for respective bands follow

        let mut frequency_band_to_sound_peaks: [Vec<FrequencyPeak>; 4] = Default::default();

        while cursor.position() < data.len() as u64 {
            let frequency_band_id = cursor.read_u32::<LittleEndian>()?;
            let frequency_peaks_size = cursor.read_u32::<LittleEndian>()?;

            let frequency_peaks_padding = (4 - frequency_peaks_size % 4) % 4;

            let peaks_start = cursor.position() as usize;
            let peaks = peaks_start
                .checked_add(frequency_peaks_size as usize)
                .and_then(|peaks_end| data.get(peaks_start..peaks_end))
                .ok_or_else(|| invalid_signature("frequency peaks past the end"))?;
            let mut frequency_peaks_cursor = Cursor::new(peaks);

            // Decode frequency peaks

            let frequency_band = match frequency_band_id.wrapping_sub(0x60030040) {
                0 => FrequencyBand::_250_520,
                1 => FrequencyBand::_520_1450,
                2 => FrequencyBand::_1450_3500,
                3 => FrequencyBand::_3500_5500,
                _ => {
                    return Err(gettext("Invalid frequency band in decoded Shazam packet").into());
                }
            };

            let mut fft_pass_number: u32 = 0;

            while frequency_peaks_cursor.position() < frequency_peaks_size as u64 {
                let fft_pass_offset = frequency_peaks_cursor.read_u8()?;

                match fft_pass_offset {
                    0xff => {
                        fft_pass_number = frequency_peaks_cursor.read_u32::<LittleEndian>()?;
                    }
                    _ => {
                        fft_pass_number = fft_pass_number
                            .checked_add(fft_pass_offset as u32)
                            .ok_or_else(|| invalid_signature("FFT pass number overflow"))?;

                        frequency_band_to_sound_peaks[frequency_band as usize].push(
                            FrequencyPeak {
                                fft_pass_number,
                                peak_magnitude: frequency_peaks_cursor
                                    .read_u16::<LittleEndian>()?,
                                corrected_peak_frequency_bin: frequency_peaks_cursor
                                    .read_u16::<LittleEndian>()?,
                            },
                        );
                    }
                };
            }

            cursor.seek(SeekFrom::Current(
                frequency_peaks_size as i64 + frequency_peaks_padding as i64,
            ))?;
        }

        // Return the decoded object

        Ok(DecodedSignature {
            sample_rate_hz,
            number_samples,
            frequency_band_to_sound_peaks,
        })
    }

    pub fn decode_from_uri(uri: &str) -> Result<Self, Box<dyn Error>> {
        if let Some(encoded) = uri.strip_prefix(DATA_URI_PREFIX) {
            DecodedSignature::decode_from_binary(&base64::prelude::BASE64_STANDARD.decode(encoded)?)
        } else {
            Err(gettext("A fingerprint must start with \"%s\"")
                .replace("%s", DATA_URI_PREFIX)
                .into())
        }
    }

    pub fn encode_to_binary(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut cursor = Cursor::new(vec![]);

        // Please see the RawSignatureHeader structure definition above for
        // information about the following fields.

        cursor.write_u32::<LittleEndian>(0xcafe2580)?; // magic1
        cursor.write_u32::<LittleEndian>(0)?; // crc32 - Will write later
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header - Will write later
        cursor.write_u32::<LittleEndian>(0x94119c00)?; // magic2
        cursor.write_u32::<LittleEndian>(0)?; // void1
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(
            match self.sample_rate_hz {
                8000 => 1,
                11025 => 2,
                16000 => 3,
                32000 => 4,
                44100 => 5,
                48000 => 6,
                _ => {
                    panic!(
                        "{}",
                        gettext("Invalid sample rate passed when encoding Shazam packet")
                    );
                }
            } << 27,
        )?; // shifted_sample_rate_id
        cursor.write_u32::<LittleEndian>(0)?; // void2
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(
            self.number_samples + (self.sample_rate_hz as f32 * 0.24) as u32,
        )?; // number_samples_plus_divided_sample_rate
        cursor.write_u32::<LittleEndian>((15 << 19) + 0x40000)?; // fixed_value

        cursor.write_u32::<LittleEndian>(0x40000000)?;
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header - Will write later

        for (frequency_band, frequency_peaks) in
            self.frequency_band_to_sound_peaks.iter().enumerate()
        {
            if frequency_peaks.is_empty() {
                continue;
            }
            let mut peaks_cursor = Cursor::new(vec![]);

            let mut fft_pass_number = 0;

            for frequency_peak in frequency_peaks {
                assert!(frequency_peak.fft_pass_number >= fft_pass_number);

                if frequency_peak.fft_pass_number - fft_pass_number >= 255 {
                    peaks_cursor.write_u8(0xff)?;
                    peaks_cursor.write_u32::<LittleEndian>(frequency_peak.fft_pass_number)?;

                    fft_pass_number = frequency_peak.fft_pass_number;
                }

                peaks_cursor.write_u8((frequency_peak.fft_pass_number - fft_pass_number) as u8)?;

                peaks_cursor.write_u16::<LittleEndian>(frequency_peak.peak_magnitude)?;
                peaks_cursor
                    .write_u16::<LittleEndian>(frequency_peak.corrected_peak_frequency_bin)?;

                fft_pass_number = frequency_peak.fft_pass_number;
            }

            let peaks_buffer = peaks_cursor.into_inner();

            cursor.write_u32::<LittleEndian>(0x60030040 + frequency_band as u32)?;
            cursor.write_u32::<LittleEndian>(peaks_buffer.len() as u32)?;
            cursor.write_all(&peaks_buffer)?;
            for _padding_index in 0..((4 - peaks_buffer.len() as u32 % 4) % 4) {
                cursor.write_u8(0)?;
            }
        }

        let buffer_size = cursor.position() as u32;

        cursor.seek(SeekFrom::Start(8))?;
        cursor.write_u32::<LittleEndian>(buffer_size - 48)?;

        cursor.seek(SeekFrom::Start(48 + 4))?;
        cursor.write_u32::<LittleEndian>(buffer_size - 48)?;

        cursor.seek(SeekFrom::Start(4))?;
        let mut hasher = Hasher::new();
        hasher.update(&cursor.get_ref()[8..]);
        cursor.write_u32::<LittleEndian>(hasher.finalize())?; // crc32

        Ok(cursor.into_inner())
    }

    pub fn encode_to_uri(&self) -> Result<String, Box<dyn Error>> {
        Ok(format!(
            "{}{}",
            DATA_URI_PREFIX,
            base64::prelude::BASE64_STANDARD.encode(self.encode_to_binary()?)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_signature() -> DecodedSignature {
        let peak = |fft_pass_number, peak_magnitude, corrected_peak_frequency_bin| FrequencyPeak {
            fft_pass_number,
            peak_magnitude,
            corrected_peak_frequency_bin,
        };
        DecodedSignature {
            sample_rate_hz: 16000,
            number_samples: 16000 * 3,
            frequency_band_to_sound_peaks: [
                vec![peak(1, 100, 30), peak(300, 200, 40)],
                vec![peak(2, 110, 60)],
                vec![],
                vec![peak(5, 120, 900), peak(6, 130, 901)],
            ],
        }
    }

    /// Recompute the CRC that covers everything after the first 8 bytes, so
    /// that a modified signature reaches the checks after the header.
    fn with_valid_crc(mut data: Vec<u8>) -> Vec<u8> {
        if data.len() >= 8 {
            let mut hasher = Hasher::new();
            hasher.update(&data[8..]);
            data[4..8].copy_from_slice(&hasher.finalize().to_le_bytes());
        }
        data
    }

    #[test]
    fn a_signature_survives_a_round_trip_through_binary() {
        let data = sample_signature().encode_to_binary().unwrap();
        let decoded = DecodedSignature::decode_from_binary(&data).unwrap();

        assert_eq!(decoded.sample_rate_hz, 16000);
        assert_eq!(decoded.number_samples, 16000 * 3);
        assert_eq!(decoded.frequency_band_to_sound_peaks[0].len(), 2);
        assert_eq!(
            decoded.frequency_band_to_sound_peaks[3][1].corrected_peak_frequency_bin,
            901
        );
    }

    #[test]
    fn truncated_signatures_are_errors() {
        let data = sample_signature().encode_to_binary().unwrap();
        for length in 0..data.len() {
            assert!(
                DecodedSignature::decode_from_binary(&data[..length]).is_err(),
                "length {length}"
            );
            // Fixing the CRC reaches the size checks.
            let truncated = with_valid_crc(data[..length].to_vec());
            assert!(
                DecodedSignature::decode_from_binary(&truncated).is_err(),
                "length {length} with a valid CRC"
            );
        }
    }

    #[test]
    fn corrupted_signatures_never_panic() {
        let data = sample_signature().encode_to_binary().unwrap();
        for index in 0..data.len() {
            for value in [0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
                let mut corrupted = data.clone();
                corrupted[index] = value;
                // Without and with a matching CRC: either way, decoding
                // returns a result instead of panicking.
                let _ = DecodedSignature::decode_from_binary(&corrupted);
                let _ = DecodedSignature::decode_from_binary(&with_valid_crc(corrupted));
            }
        }
    }

    #[test]
    fn each_header_check_is_an_error() {
        let data = sample_signature().encode_to_binary().unwrap();
        let set_u32 = |offset: usize, value: u32| {
            let mut changed = data.clone();
            changed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            with_valid_crc(changed)
        };
        for (what, changed) in [
            ("magic1", set_u32(0, 0)),
            ("size", set_u32(8, 1)),
            ("magic2", set_u32(12, 0)),
            ("sample rate", set_u32(28, 7 << 27)),
            ("number of samples", set_u32(40, 0)),
            ("first chunk type", set_u32(48, 0)),
            ("first chunk size", set_u32(52, 0)),
            ("frequency band", set_u32(56, 0x60030040 + 4)),
            ("frequency band below the first", set_u32(56, 0)),
            ("frequency peaks size", set_u32(60, u32::MAX)),
        ] {
            assert!(
                DecodedSignature::decode_from_binary(&changed).is_err(),
                "{what}"
            );
        }
        // A wrong CRC on otherwise valid data.
        let mut wrong_crc = data.clone();
        wrong_crc[4] ^= 1;
        assert!(DecodedSignature::decode_from_binary(&wrong_crc).is_err());
    }

    #[test]
    fn decoding_a_uri_without_the_data_uri_prefix_is_an_error() {
        let error = DecodedSignature::decode_from_uri("not a fingerprint")
            .err()
            .unwrap();
        assert!(error.to_string().contains(DATA_URI_PREFIX));
    }

    #[test]
    fn decoding_invalid_base64_after_the_prefix_is_an_error() {
        let uri = format!("{}not base64!", DATA_URI_PREFIX);
        assert!(DecodedSignature::decode_from_uri(&uri).is_err());
    }
}
