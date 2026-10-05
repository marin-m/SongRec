use realfft::RealFftPlanner;
use rodio::conversions::SampleTypeConverter;
use rodio::nz;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use std::error::Error;
use std::io::BufReader;

use crate::core::fingerprinting::hanning::HANNING_WINDOW_2048_MULTIPLIERS;
use crate::core::fingerprinting::signature_format::{
    DecodedSignature, FrequencyBand, FrequencyPeak,
};
#[cfg(feature = "ffmpeg")]
use crate::plugins::ffmpeg_wrapper::decode_with_ffmpeg;

/// Collect decoded samples without trusting the iterator's `size_hint()`.
///
/// Rodio's WAV decoder (at least up to 0.22.2) keeps incrementing its read
/// counter when polled past EOF, which the resampler does, so its `size_hint()`
/// underflows to ~2^32 right at the end of the stream. `collect()` and
/// `extend()` reserve from that hint, and a file whose length makes the `Vec`
/// reallocate at that moment aborts with a ~5.7 GB allocation request.
/// Pushing one by one only relies on the `Vec`'s own amortized growth.
fn collect_samples<I: Iterator<Item = f32>>(samples: I) -> Vec<f32> {
    let mut buffer = Vec::new();
    for sample in samples {
        buffer.push(sample);
    }
    buffer
}

pub struct SignatureGenerator {
    // Used when processing input:
    /// Ring buffer.
    ring_buffer_of_samples: Box<[i16; 2048]>,
    ring_buffer_of_samples_index: usize,

    /// Reordered, temporary version of the ring buffer above, with floats for
    /// precision because we applied Hanning window.
    reordered_ring_buffer_of_samples: Box<[f32; 2048]>,

    /// Temporary work buffer
    complex_fft_output: Box<[Complex<f32>; 1025]>,

    /// Ring buffer. Lists of 1025 floats, premultiplied with a Hanning
    /// function before being passed through FFT, computed from the ring
    /// buffer every new 128 samples
    fft_outputs: Box<[[f32; 1025]; 256]>,
    fft_outputs_index: u8,

    fft_object: RealFftPlanner<f32>,

    /// Ring buffer.
    spread_fft_outputs: Box<[[f32; 1025]; 256]>,
    spread_fft_outputs_index: u8,

    num_spread_ffts_done: u32,

    signature: DecodedSignature,
}

impl SignatureGenerator {
    pub fn make_signature_from_file(file_path: &str) -> Result<DecodedSignature, Box<dyn Error>> {
        // Decode the .WAV, .MP3, .OGG or .FLAC file

        #[cfg(not(feature = "ffmpeg"))]
        let decoder = rodio::Decoder::new(BufReader::new(std::fs::File::open(file_path)?));

        #[cfg(feature = "ffmpeg")]
        let decoder = {
            let mut decoder = rodio::Decoder::new(BufReader::new(std::fs::File::open(file_path)?));

            if let Err(ref _decoding_error) = decoder &&
                // Try to decode with FFMpeg, if available, in case of failure with
                // Rodio (most likely due to the use of a format unsupported by
                // Rodio, such as .WMA or .MP4/.AAC)

                let Some(new_decoder) = decode_with_ffmpeg(file_path)
            {
                decoder = Ok(new_decoder);
            }

            decoder
        };

        // Downsample the raw PCM samples to 16 KHz

        let converted_file =
            rodio::source::UniformSourceIterator::new(decoder?, nz!(1), nz!(16000));

        let mut raw_pcm_samples: Vec<f32> = collect_samples(converted_file);

        // Pad the input to at least 12 seconds in order to avoid missing data
        // at the end of the input

        if raw_pcm_samples.len() < 12 * 16000 {
            raw_pcm_samples.resize(12 * 16000, 0.0);
        }

        // Skip to the middle of the file in order to increase recognition
        // odds. Take 12 seconds of sample.

        let mut raw_pcm_samples_slice: &[f32] = &raw_pcm_samples;

        let slice_len = raw_pcm_samples_slice.len().min(12 * 16000);

        if raw_pcm_samples_slice.len() > 12 * 16000 {
            let middle = raw_pcm_samples.len() / 2;

            raw_pcm_samples_slice =
                &raw_pcm_samples_slice[middle - (6 * 16000)..middle + (6 * 16000)];
        }

        Ok(SignatureGenerator::make_signature_from_buffer(
            &raw_pcm_samples_slice[..slice_len],
        ))
    }

    pub fn make_signature_from_buffer(f32_mono_16khz_buffer: &[f32]) -> DecodedSignature {
        let mut this = SignatureGenerator {
            ring_buffer_of_samples: Box::new([0i16; 2048]),
            ring_buffer_of_samples_index: 0,

            reordered_ring_buffer_of_samples: Box::new([0.0f32; 2048]),
            complex_fft_output: Box::new([Complex::zero(); 1025]),

            fft_outputs: Box::new([[0.0f32; 1025]; 256]),
            fft_outputs_index: 0u8,

            fft_object: RealFftPlanner::<f32>::new(),

            spread_fft_outputs: Box::new([[0.0f32; 1025]; 256]),
            spread_fft_outputs_index: 0u8,

            num_spread_ffts_done: 0,

            signature: DecodedSignature {
                sample_rate_hz: 16000,
                number_samples: f32_mono_16khz_buffer.len() as u32,
                frequency_band_to_sound_peaks: Default::default(),
            },
        };

        let s16_buffer: Vec<i16> =
            SampleTypeConverter::<_, i16>::new(f32_mono_16khz_buffer.iter().copied()).collect();

        for chunk in s16_buffer.as_chunks::<128>().0 {
            this.do_fft(chunk);

            this.do_peak_spreading();

            this.num_spread_ffts_done += 1;

            if this.num_spread_ffts_done >= 46 {
                this.do_peak_recognition();
            }
        }

        this.signature
    }

    fn do_fft(&mut self, s16_mono_16khz_buffer: &[i16; 128]) {
        // Copy the 128 input s16le samples to the local ring buffer

        self.ring_buffer_of_samples
            [self.ring_buffer_of_samples_index..self.ring_buffer_of_samples_index + 128]
            .copy_from_slice(s16_mono_16khz_buffer);

        self.ring_buffer_of_samples_index += 128;
        self.ring_buffer_of_samples_index &= 2047;

        // Reorder the items (put the latest data at end) and apply Hanning window

        for (index, multiplier) in HANNING_WINDOW_2048_MULTIPLIERS.iter().enumerate() {
            self.reordered_ring_buffer_of_samples[index] = self.ring_buffer_of_samples
                [(index + self.ring_buffer_of_samples_index) & 2047]
                as f32
                * multiplier;
        }

        // Perform Fast Fourier transform

        self.fft_object
            .plan_fft_forward(2048)
            .process(
                &mut *self.reordered_ring_buffer_of_samples,
                &mut *self.complex_fft_output,
            )
            .unwrap();

        // Turn complex into reals, and put the results into a local array

        let real_fft_results = &mut self.fft_outputs[self.fft_outputs_index as usize];

        for (result, complex) in real_fft_results
            .iter_mut()
            .zip(self.complex_fft_output.iter())
        {
            *result =
                ((complex.re.powi(2) + complex.im.powi(2)) / ((1 << 17) as f32)).max(0.0000000001);
        }

        self.fft_outputs_index = self.fft_outputs_index.wrapping_add(1);
    }

    fn do_peak_spreading(&mut self) {
        let real_fft_results = &self.fft_outputs[self.fft_outputs_index.wrapping_sub(1) as usize];

        let spread_fft_results =
            &mut self.spread_fft_outputs[self.spread_fft_outputs_index as usize];

        // Perform frequency-domain spreading of peak values

        spread_fft_results.copy_from_slice(real_fft_results);

        for position in 0..=1022 {
            spread_fft_results[position] = spread_fft_results[position]
                .max(spread_fft_results[position + 1])
                .max(spread_fft_results[position + 2]);
        }

        // Perform time-domain spreading of peak values

        let spread_fft_results_copy = *spread_fft_results; // Avoid mutable+mutable borrow of self.spread_fft_outputs

        for position in 0..=1024 {
            for former_fft_number in [1, 3, 6] {
                let former_fft_output = &mut self.spread_fft_outputs[self
                    .spread_fft_outputs_index
                    .wrapping_sub(former_fft_number)
                    as usize];

                former_fft_output[position] =
                    former_fft_output[position].max(spread_fft_results_copy[position]);
            }
        }

        self.spread_fft_outputs_index = self.spread_fft_outputs_index.wrapping_add(1);
    }

    fn do_peak_recognition(&mut self) {
        // Note: when substracting an array index, casting to signed is needed
        // to avoid underflow panics at runtime.

        let fft_minus_46 = &self.fft_outputs[self.fft_outputs_index.wrapping_sub(46) as usize];
        let fft_minus_49 =
            &self.spread_fft_outputs[self.spread_fft_outputs_index.wrapping_sub(49) as usize];

        for bin_position in 10..=1014 {
            // Ensure that the bin is large enough to be a peak

            if fft_minus_46[bin_position] >= 1.0 / 64.0
                && fft_minus_46[bin_position] >= fft_minus_49[bin_position - 1]
            {
                // Ensure that it is frequency-domain local minimum

                let mut max_neighbor_in_fft_minus_49: f32 = 0.0;

                for neighbor_offset in &[-10, -7, -4, -3, 1, 2, 5, 8] {
                    max_neighbor_in_fft_minus_49 = max_neighbor_in_fft_minus_49
                        .max(fft_minus_49[(bin_position as i32 + *neighbor_offset) as usize]);
                }

                if fft_minus_46[bin_position] > max_neighbor_in_fft_minus_49 {
                    // Ensure that it is a time-domain local minimum

                    let mut max_neighbor_in_other_adjacent_ffts = max_neighbor_in_fft_minus_49;

                    for other_offset in [
                        -53, -45, 165, 172, 179, 186, 193, 200, 214, 221, 228, 235, 242, 249,
                    ] {
                        let other_fft = &self.spread_fft_outputs[((self.spread_fft_outputs_index
                            as i32
                            + other_offset)
                            & 255)
                            as usize];

                        max_neighbor_in_other_adjacent_ffts =
                            max_neighbor_in_other_adjacent_ffts.max(other_fft[bin_position - 1]);
                    }

                    if fft_minus_46[bin_position] > max_neighbor_in_other_adjacent_ffts {
                        // This is a peak, store the peak

                        let fft_pass_number = self.num_spread_ffts_done - 46;

                        let peak_magnitude: f32 =
                            fft_minus_46[bin_position].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;
                        let peak_magnitude_before: f32 =
                            fft_minus_46[bin_position - 1].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;
                        let peak_magnitude_after: f32 =
                            fft_minus_46[bin_position + 1].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;

                        let peak_variation_1: f32 =
                            peak_magnitude * 2.0 - peak_magnitude_before - peak_magnitude_after;
                        let peak_variation_2: f32 = (peak_magnitude_after - peak_magnitude_before)
                            * 32.0
                            / peak_variation_1;

                        let corrected_peak_frequency_bin: u16 =
                            ((bin_position as i32 * 64) + (peak_variation_2 as i32)) as u16;

                        assert!(peak_variation_1 >= 0.0);

                        // Convert back a FFT bin to a frequency, given a 16 KHz sample
                        // rate, 1024 useful bins and the multiplication by 64 made before
                        // storing the information

                        let frequency_hz: f32 =
                            corrected_peak_frequency_bin as f32 * (16000.0 / 2.0 / 1024.0 / 64.0);

                        // Ignore peaks outside the 250 Hz-5.5 KHz range, store them into
                        // a lookup table that will be used to generate the binary fingerprint
                        // otherwise

                        let frequency_band = match frequency_hz as i32 {
                            250..=519 => FrequencyBand::_250_520,
                            520..=1449 => FrequencyBand::_520_1450,
                            1450..=3499 => FrequencyBand::_1450_3500,
                            3500..=5500 => FrequencyBand::_3500_5500,
                            _ => {
                                continue;
                            }
                        };

                        self.signature.frequency_band_to_sound_peaks[frequency_band as usize].push(
                            FrequencyPeak {
                                fft_pass_number,
                                peak_magnitude: peak_magnitude as u16,
                                corrected_peak_frequency_bin,
                            },
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_mono_48khz_wav(path: &std::path::Path, num_samples: u32) {
        let data_len = num_samples * 2;
        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // Mono
        bytes.extend_from_slice(&48000u32.to_le_bytes());
        bytes.extend_from_slice(&(48000u32 * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for index in 0..num_samples {
            bytes.extend_from_slice(&((index % 200) as i16 * 100).to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    fn uniform_source(path: &std::path::Path) -> impl Iterator<Item = f32> {
        let decoder = rodio::Decoder::new(BufReader::new(std::fs::File::open(path).unwrap()));
        rodio::source::UniformSourceIterator::new(decoder.unwrap(), nz!(1), nz!(16000))
    }

    #[test]
    fn collecting_samples_does_not_trust_size_hint() {
        let path = std::env::temp_dir().join(format!("songrec-test-{}.wav", std::process::id()));

        // Cover every residue of the 3:1 resampling ratio around a few lengths,
        // including 474067 samples which made collect() request 5727255148 bytes
        for num_samples in (47990..48010).chain(96000..96010).chain(474060..474070) {
            write_mono_48khz_wav(&path, num_samples);

            let samples = collect_samples(uniform_source(&path));
            assert!(samples.len() <= num_samples as usize / 3 + 2);
            assert!(samples.capacity() <= 2 * (num_samples as usize / 3 + 2));
        }

        let _ = std::fs::remove_file(&path);
    }
}
