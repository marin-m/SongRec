#!/usr/bin/python3
#-*- encoding: Utf-8 -*-
from numpy import fft, array as nparray, maximum, log, hanning, mean, abs, round
from typing import Dict, List, Set, Sequence, Union, Optional, Any
from struct import pack, unpack
from enum import IntEnum
from copy import copy

HANNING_MATRIX = hanning(2050)[1:-1] # Wipe trailing and leading zeroes


from signature_format import DecodedMessage, FrequencyPeak, RawSignatureHeader, FrequencyBand


class RingBuffer(list):
    
    def __init__(self, buffer_size : int, default_value : Any = None):
        
        if default_value is not None:
            list.__init__(self, [copy(default_value) for position in range(buffer_size)])
        else:
            list.__init__(self, [None] * buffer_size)
        
        self.position : int = 0
        self.buffer_size : int = buffer_size
        self.num_written : int = 0
    
    def append(self, value : Any):
        
        self[self.position] = value
        
        self.position += 1
        self.position %= self.buffer_size
        self.num_written += 1
        
class SignatureGenerator:
    
    def __init__(self):
        
        # Used when storing input that will be processed when requiring to
        # generate a signature:
        
        self.input_pending_processing : List[int] = [] # Signed 16-bits, 16 KHz mono samples to be processed
        
        self.samples_processed : int = 0 # Number of samples processed out of "self.input_pending_processing"
        
        # Used when processing input:
        
        self.ring_buffer_of_samples : RingBuffer[int] = RingBuffer(buffer_size = 2048, default_value = 0)
        
        self.fft_outputs : RingBuffer[List[float]] = RingBuffer(buffer_size = 256, default_value = [0. * 1025]) # Lists of 1025 floats, premultiplied with a Hanning function before being passed through FFT, computed from the ring buffer every new 128 samples
        
        self.spread_ffts_output : RingBuffer[List[float]] = RingBuffer(buffer_size = 256, default_value = [0] * 1025)

        # How much data to send to Shazam at once?

        self.MAX_TIME_SECONDS = 3.1
        self.MAX_PEAKS = 255
        
        # The object that will hold information about the next fingerpring
        # to be produced
        
        self.next_signature = DecodedMessage()
        self.next_signature.sample_rate_hz = 16000
        self.next_signature.number_samples = 0
        self.next_signature.frequency_band_to_sound_peaks = {}
    
    """
        Add data to be generated a signature for, which will be
        processed when self.get_next_signature() is called. This
        function expects signed 16-bit 16 KHz mono PCM samples.
    """
    
    def feed_input(self, s16le_mono_samples : List[int]):
        
        self.input_pending_processing += s16le_mono_samples
    
    """
        Consume some of the samples fed to self.feed_input(), and return
        a Shazam signature (DecodedMessage object) to be sent to servers
        once "enough data has been gathered".
        
        Except if there are no more samples to be consumed, in this case
        we will return None.
    """
    
    def get_next_signature(self) -> Optional[DecodedMessage]:
        
        if len(self.input_pending_processing) - self.samples_processed < 128:
            return None
        
        while (len(self.input_pending_processing) - self.samples_processed >= 128 and
            (self.next_signature.number_samples / self.next_signature.sample_rate_hz < self.MAX_TIME_SECONDS or
            sum(len(peaks) for peaks in self.next_signature.frequency_band_to_sound_peaks.values()) < self.MAX_PEAKS
            )):
            
            self.process_input(self.input_pending_processing[self.samples_processed:self.samples_processed + 128])
            
            self.samples_processed += 128

        returned_signature = self.next_signature

        self.next_signature = DecodedMessage()
        self.next_signature.sample_rate_hz = 16000
        self.next_signature.number_samples = 0
        self.next_signature.frequency_band_to_sound_peaks = {}
        
        self.ring_buffer_of_samples : RingBuffer[int] = RingBuffer(buffer_size = 2048, default_value = 0)
        self.fft_outputs : RingBuffer[List[float]] = RingBuffer(buffer_size = 256, default_value = [0. * 1025])
        self.spread_ffts_output : RingBuffer[List[float]] = RingBuffer(buffer_size = 256, default_value = [0] * 1025)
        
        return returned_signature

    
    def process_input(self, s16le_mono_samples : List[int]):
    
        self.next_signature.number_samples += len(s16le_mono_samples)
        
        for position_of_chunk in range(0, len(s16le_mono_samples), 128):
            
            self.do_fft(s16le_mono_samples[position_of_chunk:position_of_chunk + 128])
            
            self.do_peak_spreading_and_recognition()
        
    def do_fft(self, batch_of_128_s16le_mono_samples):
        
        self.ring_buffer_of_samples[
            self.ring_buffer_of_samples.position:
            self.ring_buffer_of_samples.position + len(batch_of_128_s16le_mono_samples)
        ] = batch_of_128_s16le_mono_samples
        
        self.ring_buffer_of_samples.position += len(batch_of_128_s16le_mono_samples)
        self.ring_buffer_of_samples.position %= 2048
        self.ring_buffer_of_samples.num_written +=  len(batch_of_128_s16le_mono_samples)
        
        excerpt_from_ring_buffer : list = (
            self.ring_buffer_of_samples[self.ring_buffer_of_samples.position:] +
            self.ring_buffer_of_samples[:self.ring_buffer_of_samples.position]
        )
        
        # The premultiplication of the array is for applying a windowing function before the DFT (slighty rounded Hanning without zeros at edges)
        
        fft_results : nparray = fft.rfft(HANNING_MATRIX * excerpt_from_ring_buffer)

        assert len(fft_results) == 1025 and len(excerpt_from_ring_buffer) == 2048 == len(HANNING_MATRIX)
        
        fft_results = (fft_results.real ** 2 + fft_results.imag ** 2) / (1 << 17)
        fft_results = maximum(fft_results, 0.0000000001)
        
        self.fft_outputs.append(fft_results)
        
    
    def do_peak_spreading_and_recognition(self):
        
        self.do_peak_spreading()
        
        if self.spread_ffts_output.num_written >= 46:
            
            self.do_peak_recognition()
    
    def do_peak_spreading(self):
        
        origin_last_fft : List[float] = self.fft_outputs[self.fft_outputs.position - 1]
        
        spread_last_fft : List[float] = list(origin_last_fft)
        
        for position in range(1025):
            
            # Perform frequency-domain spreading of peak values
            
            if position < 1023:
            
                spread_last_fft[position] = max(spread_last_fft[position:position + 3])
            
            # Perform time-domain spreading of peak values
            
            max_value = spread_last_fft[position]
            
            for former_fft_num in [-1, -3, -6]:
                
                former_fft_output = self.spread_ffts_output[(self.spread_ffts_output.position + former_fft_num) % self.spread_ffts_output.buffer_size]
                
                former_fft_output[position] = max_value = max(former_fft_output[position], max_value)
                
        # Save output locally
        
        self.spread_ffts_output.append(spread_last_fft)
        
        pass
    
    def do_peak_recognition(self):
        
        fft_minus_46 = self.fft_outputs[(self.fft_outputs.position - 46) % self.fft_outputs.buffer_size]
        fft_minus_49 = self.spread_ffts_output[(self.spread_ffts_output.position - 49) % self.spread_ffts_output.buffer_size]
        fft_minus_53 = self.spread_ffts_output[(self.spread_ffts_output.position - 53) % self.spread_ffts_output.buffer_size]
        fft_minus_45 = self.spread_ffts_output[(self.spread_ffts_output.position - 45) % self.spread_ffts_output.buffer_size]
        
        for bin_position in range(10, 1015):
            
            # Ensure that the bin is large enough to be a peak
            
            if (fft_minus_46[bin_position] >= 1 / 64 and
                fft_minus_46[bin_position] >= fft_minus_49[bin_position - 1]):
                
                # Ensure that it is frequency-domain local minimum
                
                max_neighbor_in_fft_minus_49 = 0
                
                for neighbor_offset in [*range(-10, -3, 3), -3, 1, *range(2, 9, 3)]:
                
                    max_neighbor_in_fft_minus_49 = max(fft_minus_49[bin_position + neighbor_offset], max_neighbor_in_fft_minus_49)
                
                if fft_minus_46[bin_position] > max_neighbor_in_fft_minus_49:
                    
                    # Ensure that it is a time-domain local minimum
                    
                    max_neighbor_in_other_adjacent_ffts = max_neighbor_in_fft_minus_49
                    
                    for other_offset in [-53, -45, *range(165, 201, 7), *range(214, 250, 7)]:
                    
                        max_neighbor_in_other_adjacent_ffts = max(
                            self.spread_ffts_output[(self.spread_ffts_output.position + other_offset) % self.spread_ffts_output.buffer_size][bin_position - 1],
                            max_neighbor_in_other_adjacent_ffts
                        )
                    
                    if fft_minus_46[bin_position] > max_neighbor_in_other_adjacent_ffts:
                        
                        # This is a peak, store the peak
                        
                        fft_number = self.spread_ffts_output.num_written - 46
                        
                        peak_magnitude = log(max(1 / 64, fft_minus_46[bin_position])) * 1477.3 + 6144
                        peak_magnitude_before = log(max(1 / 64, fft_minus_46[bin_position - 1])) * 1477.3 + 6144
                        peak_magnitude_after = log(max(1 / 64, fft_minus_46[bin_position + 1])) * 1477.3 + 6144
                        
                        peak_variation_1 = peak_magnitude * 2 - peak_magnitude_before - peak_magnitude_after
                        peak_variation_2 = (peak_magnitude_after - peak_magnitude_before) * 32 / peak_variation_1
                        
                        corrected_peak_frequency_bin = bin_position * 64 + peak_variation_2
                        
                        assert peak_variation_1 > 0
                        
                        frequency_hz = corrected_peak_frequency_bin * (16000 / 2 / 1024 / 64)
                        
                        if frequency_hz < 250:
                            continue
                        elif frequency_hz < 520:
                            band = FrequencyBand._250_520
                        elif frequency_hz < 1450:
                            band = FrequencyBand._520_1450
                        elif frequency_hz < 3500:
                            band = FrequencyBand._1450_3500
                        elif frequency_hz <= 5500:
                            band = FrequencyBand._3500_5500
                        else:
                            continue
                        
                        if band not in self.next_signature.frequency_band_to_sound_peaks:
                            self.next_signature.frequency_band_to_sound_peaks[band] = []
                        
                        self.next_signature.frequency_band_to_sound_peaks[band].append(
                            FrequencyPeak(fft_number, int(peak_magnitude), int(corrected_peak_frequency_bin), 16000)
                        )
                        
