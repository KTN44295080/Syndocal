//! Allocation-stable streaming audio features for live lighting and video control.
//!
//! The analyzer owns all of its FFT, ring-buffer, filter-bank, and tempo scratch
//! storage. After construction, [`LiveAudioFeatureAnalyzer::push_samples`] does
//! not grow or replace that storage. The caller controls what happens to emitted
//! frames through a callback, so a real-time worker can copy the small fixed-size
//! frame into its own bounded queue without creating a `Vec` per analysis hop.

use thiserror::Error;

pub const MAX_LIVE_AUDIO_BANDS: usize = 16;
pub const MIN_LIVE_AUDIO_BANDS: usize = 8;
pub const MIN_TRACKED_BPM: f32 = 60.0;
pub const MAX_TRACKED_BPM: f32 = 200.0;

const MIN_FFT_SIZE: usize = 256;
const MAX_FFT_SIZE: usize = 8_192;
const FLUX_HISTORY_LEN: usize = 64;
const TEMPO_INTERVAL_HISTORY_LEN: usize = 32;
const MIN_TEMPO_INTERVALS: usize = 3;
const TEMPO_STALE_SECONDS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveAudioFeatureConfig {
    pub sample_rate: u32,
    pub fft_size: usize,
    pub hop_size: usize,
    pub band_count: usize,
    pub min_frequency_hz: f32,
    pub max_frequency_hz: f32,
    pub db_floor: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub onset_sensitivity: f32,
    pub onset_flux_floor: f32,
    pub onset_gate_db: f32,
    pub onset_refractory_ms: f32,
}

impl LiveAudioFeatureConfig {
    /// Builds a 16-band configuration with an approximately 43 ms FFT window
    /// and 21 ms hop at common sample rates.
    pub fn for_sample_rate(sample_rate: u32) -> Self {
        let target_fft_size = (sample_rate as usize / 24).max(MIN_FFT_SIZE);
        let fft_size = target_fft_size
            .checked_next_power_of_two()
            .unwrap_or(MAX_FFT_SIZE)
            .clamp(1_024, MAX_FFT_SIZE);
        let nyquist = sample_rate as f32 * 0.5;
        Self {
            sample_rate,
            fft_size,
            hop_size: fft_size / 2,
            band_count: MAX_LIVE_AUDIO_BANDS,
            min_frequency_hz: 30.0,
            max_frequency_hz: 18_000.0_f32.min(nyquist * 0.95),
            db_floor: -72.0,
            attack_ms: 25.0,
            release_ms: 180.0,
            onset_sensitivity: 1.5,
            onset_flux_floor: 0.06,
            onset_gate_db: -60.0,
            onset_refractory_ms: 100.0,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LiveAudioFeatureError {
    #[error("live audio feature sample rate must be greater than zero")]
    InvalidSampleRate,
    #[error("live audio feature FFT size must be a power of two between 256 and 8192")]
    InvalidFftSize,
    #[error("live audio feature hop size must be between 1 and the FFT size")]
    InvalidHopSize,
    #[error("live audio feature band count must be between 8 and 16")]
    InvalidBandCount,
    #[error("live audio feature FFT resolution leaves one or more configured bands empty")]
    InsufficientFrequencyResolution,
    #[error(
        "live audio feature frequency range must be finite, positive, ordered, and below Nyquist"
    )]
    InvalidFrequencyRange,
    #[error("live audio feature dB floor must be finite and below 0 dBFS")]
    InvalidDbFloor,
    #[error("live audio feature envelope times must be finite and positive")]
    InvalidEnvelope,
    #[error("live audio onset settings are outside their supported finite ranges")]
    InvalidOnsetSettings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveAudioFeatureFrame {
    /// Zero-based analysis-frame sequence number.
    pub sequence: u64,
    /// Exclusive mono input sample index at the end of this FFT window.
    pub end_sample: u64,
    pub sample_rate: u32,
    /// Linear full-window RMS in the range 0..=1 after finite-value sanitizing.
    pub rms: f32,
    /// Linear full-window absolute peak in the range 0..=1.
    pub peak: f32,
    /// Attack/release-smoothed logarithmic filter-bank levels in the range 0..=1.
    pub bands: [f32; MAX_LIVE_AUDIO_BANDS],
    pub band_count: usize,
    /// Half-wave spectral change normalized by current spectral magnitude, 0..=1.
    pub spectral_flux: f32,
    /// True once for an adaptive-threshold crossing outside the refractory period.
    pub onset: bool,
    /// Adaptive onset margin in the range 0..=1, or zero when no onset was emitted.
    pub onset_strength: f32,
    /// Robust inter-onset tempo estimate folded into 60..=200 BPM.
    pub bpm: Option<f32>,
    /// Tempo maturity, interval consistency, and staleness confidence, 0..=1.
    pub bpm_confidence: f32,
}

impl LiveAudioFeatureFrame {
    pub fn bands(&self) -> &[f32] {
        &self.bands[..self.band_count]
    }

    pub fn end_time_seconds(&self) -> f64 {
        self.end_sample as f64 / f64::from(self.sample_rate)
    }
}

#[derive(Debug)]
pub struct LiveAudioFeatureAnalyzer {
    config: LiveAudioFeatureConfig,
    sample_ring: Vec<f32>,
    write_index: usize,
    collected_samples: usize,
    samples_since_frame: usize,
    total_samples: u64,
    sequence: u64,
    hann: Vec<f32>,
    fft_real: Vec<f32>,
    fft_imaginary: Vec<f32>,
    magnitudes: Vec<f32>,
    previous_magnitudes: Vec<f32>,
    band_for_bin: Vec<u8>,
    band_edges_hz: [f32; MAX_LIVE_AUDIO_BANDS + 1],
    smoothed_bands: [f32; MAX_LIVE_AUDIO_BANDS],
    fft_coherent_scale: f32,
    attack_coefficient: f32,
    release_coefficient: f32,
    onset_gate_linear: f32,
    onset_refractory_samples: u64,
    flux_history: FluxHistory,
    previous_flux: f32,
    last_onset_sample: Option<u64>,
    tempo: TempoTracker,
}

impl LiveAudioFeatureAnalyzer {
    pub fn new(config: LiveAudioFeatureConfig) -> Result<Self, LiveAudioFeatureError> {
        validate_config(config)?;

        let mut hann = vec![0.0; config.fft_size];
        for (index, value) in hann.iter_mut().enumerate() {
            let phase =
                std::f32::consts::TAU * index as f32 / config.fft_size.saturating_sub(1) as f32;
            *value = 0.5 - 0.5 * phase.cos();
        }
        let fft_coherent_scale = 2.0 / hann.iter().sum::<f32>().max(f32::EPSILON);

        let mut band_edges_hz = [0.0; MAX_LIVE_AUDIO_BANDS + 1];
        let frequency_ratio = (config.max_frequency_hz / config.min_frequency_hz)
            .powf(1.0 / config.band_count as f32);
        for (index, edge) in band_edges_hz
            .iter_mut()
            .enumerate()
            .take(config.band_count + 1)
        {
            *edge = if index == config.band_count {
                config.max_frequency_hz
            } else {
                config.min_frequency_hz * frequency_ratio.powf(index as f32)
            };
        }

        let magnitude_len = config.fft_size / 2 + 1;
        let mut band_for_bin = vec![u8::MAX; magnitude_len];
        for (bin, band) in band_for_bin.iter_mut().enumerate().skip(1) {
            let frequency = bin as f32 * config.sample_rate as f32 / config.fft_size as f32;
            if frequency > config.max_frequency_hz {
                continue;
            }
            // The first non-DC bin may sit just below the requested lower edge.
            // Folding that single bin into band zero avoids a permanently empty
            // sub-bass band without admitting DC offset into the feature set.
            let frequency = frequency.max(config.min_frequency_hz);
            let index = band_edges_hz[..=config.band_count]
                .partition_point(|edge| *edge <= frequency)
                .saturating_sub(1)
                .min(config.band_count - 1);
            *band = index as u8;
        }
        if (0..config.band_count).any(|band| !band_for_bin.contains(&(band as u8))) {
            return Err(LiveAudioFeatureError::InsufficientFrequencyResolution);
        }

        let hop_seconds = config.hop_size as f32 / config.sample_rate as f32;
        let coefficient = |milliseconds: f32| (-hop_seconds / (milliseconds * 0.001)).exp();
        Ok(Self {
            config,
            sample_ring: vec![0.0; config.fft_size],
            write_index: 0,
            collected_samples: 0,
            samples_since_frame: 0,
            total_samples: 0,
            sequence: 0,
            hann,
            fft_real: vec![0.0; config.fft_size],
            fft_imaginary: vec![0.0; config.fft_size],
            magnitudes: vec![0.0; magnitude_len],
            previous_magnitudes: vec![0.0; magnitude_len],
            band_for_bin,
            band_edges_hz,
            smoothed_bands: [0.0; MAX_LIVE_AUDIO_BANDS],
            fft_coherent_scale,
            attack_coefficient: coefficient(config.attack_ms),
            release_coefficient: coefficient(config.release_ms),
            onset_gate_linear: 10.0_f32.powf(config.onset_gate_db / 20.0),
            onset_refractory_samples: (config.sample_rate as f32
                * config.onset_refractory_ms
                * 0.001)
                .round() as u64,
            flux_history: FluxHistory::default(),
            previous_flux: 0.0,
            last_onset_sample: None,
            tempo: TempoTracker::default(),
        })
    }

    pub fn config(&self) -> LiveAudioFeatureConfig {
        self.config
    }

    pub fn band_edges_hz(&self) -> &[f32] {
        &self.band_edges_hz[..=self.config.band_count]
    }

    pub fn analysis_rate_hz(&self) -> f32 {
        self.config.sample_rate as f32 / self.config.hop_size as f32
    }

    /// Streams mono samples into the analyzer and emits zero or more fixed-size
    /// feature frames. Non-finite input is treated as silence.
    pub fn push_samples<F>(&mut self, samples: &[f32], mut emit: F) -> usize
    where
        F: FnMut(LiveAudioFeatureFrame),
    {
        let mut emitted = 0;
        for sample in samples {
            self.sample_ring[self.write_index] = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            self.write_index = (self.write_index + 1) % self.config.fft_size;
            self.total_samples = self.total_samples.saturating_add(1);

            let should_analyze = if self.collected_samples < self.config.fft_size {
                self.collected_samples += 1;
                self.collected_samples == self.config.fft_size
            } else {
                self.samples_since_frame += 1;
                self.samples_since_frame >= self.config.hop_size
            };
            if should_analyze {
                self.samples_since_frame = 0;
                let frame = self.analyze_window();
                emit(frame);
                emitted += 1;
            }
        }
        emitted
    }

    /// Clears streaming and adaptive state while retaining every allocation.
    pub fn reset(&mut self) {
        self.sample_ring.fill(0.0);
        self.write_index = 0;
        self.collected_samples = 0;
        self.samples_since_frame = 0;
        self.total_samples = 0;
        self.sequence = 0;
        self.fft_real.fill(0.0);
        self.fft_imaginary.fill(0.0);
        self.magnitudes.fill(0.0);
        self.previous_magnitudes.fill(0.0);
        self.smoothed_bands.fill(0.0);
        self.flux_history = FluxHistory::default();
        self.previous_flux = 0.0;
        self.last_onset_sample = None;
        self.tempo = TempoTracker::default();
    }

    fn analyze_window(&mut self) -> LiveAudioFeatureFrame {
        let mut sum_squares = 0.0_f32;
        let mut peak = 0.0_f32;
        for index in 0..self.config.fft_size {
            let sample = self.sample_ring[(self.write_index + index) % self.config.fft_size];
            sum_squares += sample * sample;
            peak = peak.max(sample.abs());
            self.fft_real[index] = sample * self.hann[index];
            self.fft_imaginary[index] = 0.0;
        }
        let rms = (sum_squares / self.config.fft_size as f32)
            .sqrt()
            .clamp(0.0, 1.0);

        super::fft_in_place(&mut self.fft_real, &mut self.fft_imaginary);
        for bin in 0..self.magnitudes.len() {
            self.magnitudes[bin] = (self.fft_real[bin] * self.fft_real[bin]
                + self.fft_imaginary[bin] * self.fft_imaginary[bin])
                .sqrt()
                * self.fft_coherent_scale;
        }

        let mut positive_change = 0.0_f32;
        let mut current_magnitude = 0.0_f32;
        let mut band_squares = [0.0_f32; MAX_LIVE_AUDIO_BANDS];
        let mut band_bins = [0_u32; MAX_LIVE_AUDIO_BANDS];
        for bin in 1..self.magnitudes.len() {
            let band = self.band_for_bin[bin];
            if band == u8::MAX {
                continue;
            }
            let magnitude = self.magnitudes[bin];
            positive_change += (magnitude - self.previous_magnitudes[bin]).max(0.0);
            current_magnitude += magnitude;
            let band = band as usize;
            band_squares[band] += magnitude * magnitude;
            band_bins[band] += 1;
        }
        let spectral_flux = if current_magnitude <= f32::EPSILON {
            0.0
        } else {
            (positive_change / current_magnitude).clamp(0.0, 1.0)
        };
        self.previous_magnitudes.copy_from_slice(&self.magnitudes);

        for band in 0..self.config.band_count {
            let magnitude = if band_bins[band] == 0 {
                0.0
            } else {
                (band_squares[band] / band_bins[band] as f32).sqrt()
            };
            let db = 20.0 * magnitude.max(1.0e-12).log10();
            let target = ((db - self.config.db_floor) / -self.config.db_floor).clamp(0.0, 1.0);
            let coefficient = if target > self.smoothed_bands[band] {
                self.attack_coefficient
            } else {
                self.release_coefficient
            };
            self.smoothed_bands[band] = (coefficient * self.smoothed_bands[band]
                + (1.0 - coefficient) * target)
                .clamp(0.0, 1.0);
        }

        let (flux_mean, flux_deviation) = self.flux_history.mean_and_deviation();
        let threshold = self
            .config
            .onset_flux_floor
            .max(flux_mean + self.config.onset_sensitivity * flux_deviation)
            .clamp(0.0, 0.95);
        let outside_refractory = self
            .last_onset_sample
            .map(|sample| {
                self.total_samples.saturating_sub(sample) >= self.onset_refractory_samples
            })
            .unwrap_or(true);
        let crossed_threshold = spectral_flux > threshold && self.previous_flux <= threshold;
        let onset = rms >= self.onset_gate_linear && outside_refractory && crossed_threshold;
        let onset_strength = if onset {
            ((spectral_flux - threshold) / (1.0 - threshold).max(f32::EPSILON))
                .sqrt()
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        if onset {
            self.last_onset_sample = Some(self.total_samples);
            self.tempo
                .record_onset(self.total_samples, self.config.sample_rate);
        }
        self.flux_history.push(spectral_flux);
        self.previous_flux = spectral_flux;

        let (bpm, bpm_confidence) = self
            .tempo
            .snapshot(self.total_samples, self.config.sample_rate);
        let mut bands = [0.0; MAX_LIVE_AUDIO_BANDS];
        bands[..self.config.band_count]
            .copy_from_slice(&self.smoothed_bands[..self.config.band_count]);
        let frame = LiveAudioFeatureFrame {
            sequence: self.sequence,
            end_sample: self.total_samples,
            sample_rate: self.config.sample_rate,
            rms,
            peak,
            bands,
            band_count: self.config.band_count,
            spectral_flux,
            onset,
            onset_strength,
            bpm,
            bpm_confidence,
        };
        self.sequence = self.sequence.saturating_add(1);
        frame
    }
}

fn validate_config(config: LiveAudioFeatureConfig) -> Result<(), LiveAudioFeatureError> {
    if config.sample_rate == 0 {
        return Err(LiveAudioFeatureError::InvalidSampleRate);
    }
    if !config.fft_size.is_power_of_two()
        || !(MIN_FFT_SIZE..=MAX_FFT_SIZE).contains(&config.fft_size)
    {
        return Err(LiveAudioFeatureError::InvalidFftSize);
    }
    if config.hop_size == 0 || config.hop_size > config.fft_size {
        return Err(LiveAudioFeatureError::InvalidHopSize);
    }
    if !(MIN_LIVE_AUDIO_BANDS..=MAX_LIVE_AUDIO_BANDS).contains(&config.band_count) {
        return Err(LiveAudioFeatureError::InvalidBandCount);
    }
    let nyquist = config.sample_rate as f32 * 0.5;
    if !config.min_frequency_hz.is_finite()
        || !config.max_frequency_hz.is_finite()
        || config.min_frequency_hz <= 0.0
        || config.max_frequency_hz <= config.min_frequency_hz
        || config.max_frequency_hz > nyquist
    {
        return Err(LiveAudioFeatureError::InvalidFrequencyRange);
    }
    if !config.db_floor.is_finite() || config.db_floor >= 0.0 {
        return Err(LiveAudioFeatureError::InvalidDbFloor);
    }
    if !config.attack_ms.is_finite()
        || !config.release_ms.is_finite()
        || config.attack_ms <= 0.0
        || config.release_ms <= 0.0
    {
        return Err(LiveAudioFeatureError::InvalidEnvelope);
    }
    if !config.onset_sensitivity.is_finite()
        || config.onset_sensitivity < 0.0
        || !config.onset_flux_floor.is_finite()
        || !(0.0..1.0).contains(&config.onset_flux_floor)
        || !config.onset_gate_db.is_finite()
        || config.onset_gate_db >= 0.0
        || !config.onset_refractory_ms.is_finite()
        || config.onset_refractory_ms <= 0.0
    {
        return Err(LiveAudioFeatureError::InvalidOnsetSettings);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct FluxHistory {
    values: [f32; FLUX_HISTORY_LEN],
    count: usize,
    write_index: usize,
}

impl Default for FluxHistory {
    fn default() -> Self {
        Self {
            values: [0.0; FLUX_HISTORY_LEN],
            count: 0,
            write_index: 0,
        }
    }
}

impl FluxHistory {
    fn push(&mut self, value: f32) {
        self.values[self.write_index] = value;
        self.write_index = (self.write_index + 1) % FLUX_HISTORY_LEN;
        self.count = (self.count + 1).min(FLUX_HISTORY_LEN);
    }

    fn mean_and_deviation(&self) -> (f32, f32) {
        if self.count == 0 {
            return (0.0, 0.0);
        }
        let values = &self.values[..self.count];
        let mean = values.iter().sum::<f32>() / self.count as f32;
        let variance = values
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f32>()
            / self.count as f32;
        (mean, variance.sqrt())
    }
}

#[derive(Debug, Clone, Copy)]
struct TempoTracker {
    intervals: [u64; TEMPO_INTERVAL_HISTORY_LEN],
    interval_count: usize,
    write_index: usize,
    last_onset_sample: Option<u64>,
    bpm: Option<f32>,
    confidence: f32,
}

impl Default for TempoTracker {
    fn default() -> Self {
        Self {
            intervals: [0; TEMPO_INTERVAL_HISTORY_LEN],
            interval_count: 0,
            write_index: 0,
            last_onset_sample: None,
            bpm: None,
            confidence: 0.0,
        }
    }
}

impl TempoTracker {
    fn record_onset(&mut self, sample: u64, sample_rate: u32) {
        let Some(previous) = self.last_onset_sample else {
            self.last_onset_sample = Some(sample);
            return;
        };
        let interval = sample.saturating_sub(previous);
        // Permit one FFT-hop worth of timestamp quantization near the 200 BPM
        // boundary. The later robust interval filter still rejects inconsistent
        // double triggers.
        let minimum_interval = (sample_rate as f32 * 60.0 / MAX_TRACKED_BPM * 0.9).round() as u64;
        if interval < minimum_interval {
            return;
        }
        let maximum_gap = u64::from(sample_rate) * 2;
        self.last_onset_sample = Some(sample);
        if interval > maximum_gap {
            self.intervals.fill(0);
            self.interval_count = 0;
            self.write_index = 0;
            self.bpm = None;
            self.confidence = 0.0;
            return;
        }

        self.intervals[self.write_index] = interval;
        self.write_index = (self.write_index + 1) % TEMPO_INTERVAL_HISTORY_LEN;
        self.interval_count = (self.interval_count + 1).min(TEMPO_INTERVAL_HISTORY_LEN);
        self.estimate(sample_rate);
    }

    fn estimate(&mut self, sample_rate: u32) {
        if self.interval_count < MIN_TEMPO_INTERVALS {
            return;
        }
        let mut sorted = [0_u64; TEMPO_INTERVAL_HISTORY_LEN];
        sorted[..self.interval_count].copy_from_slice(&self.intervals[..self.interval_count]);
        sorted[..self.interval_count].sort_unstable();
        let median = sorted[self.interval_count / 2] as f32;
        let tolerance = median * 0.2;
        let mut inlier_sum = 0.0_f32;
        let mut inlier_squares = 0.0_f32;
        let mut inliers = 0_usize;
        for interval in &self.intervals[..self.interval_count] {
            let interval = *interval as f32;
            if (interval - median).abs() <= tolerance {
                inlier_sum += interval;
                inlier_squares += interval * interval;
                inliers += 1;
            }
        }
        if inliers < MIN_TEMPO_INTERVALS {
            return;
        }
        let mean_interval = inlier_sum / inliers as f32;
        let mut bpm = 60.0 * sample_rate as f32 / mean_interval.max(1.0);
        // Do not octave-flip a valid boundary tempo because hop quantization made
        // a 60 BPM estimate read 59.x or a 200 BPM estimate read 200.x.
        while bpm < MIN_TRACKED_BPM * 0.9 {
            bpm *= 2.0;
        }
        while bpm > MAX_TRACKED_BPM * 1.1 {
            bpm *= 0.5;
        }
        bpm = bpm.clamp(MIN_TRACKED_BPM, MAX_TRACKED_BPM);
        if !bpm.is_finite() {
            return;
        }

        let variance = (inlier_squares / inliers as f32 - mean_interval * mean_interval).max(0.0);
        let coefficient_of_variation = variance.sqrt() / mean_interval.max(1.0);
        // The FFT-hop timestamp quantizes otherwise perfect pulses by up to one hop.
        // A 12% variation budget keeps confidence honest for live drummers while not
        // penalizing the expected 23/24-hop alternation of a 120 BPM pulse at 48 kHz.
        let consistency = (1.0 - coefficient_of_variation / 0.12).clamp(0.0, 1.0);
        let inlier_fraction = inliers as f32 / self.interval_count as f32;
        let maturity = (self.interval_count as f32 / 8.0).clamp(0.0, 1.0);
        let confidence = (consistency * inlier_fraction * maturity).clamp(0.0, 1.0);
        self.bpm = Some(match self.bpm {
            Some(previous) if (previous - bpm).abs() / previous.max(1.0) <= 0.12 => {
                previous * 0.7 + bpm * 0.3
            }
            Some(previous) if confidence < 0.65 => previous,
            _ => bpm,
        });
        self.confidence = confidence;
    }

    fn snapshot(&self, sample: u64, sample_rate: u32) -> (Option<f32>, f32) {
        let Some(last_onset) = self.last_onset_sample else {
            return (None, 0.0);
        };
        let age_seconds = sample.saturating_sub(last_onset) as f32 / sample_rate as f32;
        if age_seconds >= TEMPO_STALE_SECONDS {
            return (None, 0.0);
        }
        let freshness = if age_seconds <= TEMPO_STALE_SECONDS * 0.5 {
            1.0
        } else {
            1.0 - (age_seconds - TEMPO_STALE_SECONDS * 0.5) / (TEMPO_STALE_SECONDS * 0.5)
        };
        (self.bpm, (self.confidence * freshness).clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(sample_rate: u32, frequency: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let sample_count = (sample_rate as f32 * seconds).round() as usize;
        (0..sample_count)
            .map(|index| {
                (std::f32::consts::TAU * frequency * index as f32 / sample_rate as f32).sin()
                    * amplitude
            })
            .collect()
    }

    fn click_track(sample_rate: u32, bpm: f32, seconds: f32) -> Vec<f32> {
        let sample_count = (sample_rate as f32 * seconds).round() as usize;
        let beat_samples = (sample_rate as f32 * 60.0 / bpm).round() as usize;
        let mut samples = vec![0.0; sample_count];
        let mut state = 0x1234_5678_u32;
        for beat in (beat_samples / 2..sample_count).step_by(beat_samples) {
            for offset in 0..256.min(sample_count - beat) {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = ((state >> 8) as f32 / 16_777_215.0) * 2.0 - 1.0;
                let envelope = 1.0 - offset as f32 / 256.0;
                samples[beat + offset] = noise * envelope * 0.9;
            }
        }
        samples
    }

    #[test]
    fn default_configuration_has_stable_low_latency_geometry() {
        for sample_rate in [44_100_u32, 48_000, 96_000, 192_000] {
            let config = LiveAudioFeatureConfig::for_sample_rate(sample_rate);
            let analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
            assert_eq!(config.band_count, 16);
            assert_eq!(analyzer.band_edges_hz().len(), 17);
            assert!(analyzer
                .band_edges_hz()
                .windows(2)
                .all(|edge| edge[0] < edge[1]));
            assert!(
                (0..config.band_count).all(|band| analyzer.band_for_bin.contains(&(band as u8)))
            );
        }
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        assert_eq!(config.fft_size, 2_048);
        assert_eq!(config.hop_size, 1_024);
        assert!((analyzer.analysis_rate_hz() - 46.875).abs() < 0.001);
    }

    #[test]
    fn rejects_unsupported_geometry_and_ranges() {
        let mut config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        config.band_count = 7;
        assert_eq!(
            LiveAudioFeatureAnalyzer::new(config).unwrap_err(),
            LiveAudioFeatureError::InvalidBandCount
        );
        config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        config.fft_size = 1_000;
        assert_eq!(
            LiveAudioFeatureAnalyzer::new(config).unwrap_err(),
            LiveAudioFeatureError::InvalidFftSize
        );
        config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        config.max_frequency_hz = 25_000.0;
        assert_eq!(
            LiveAudioFeatureAnalyzer::new(config).unwrap_err(),
            LiveAudioFeatureError::InvalidFrequencyRange
        );
        config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        config.fft_size = 256;
        config.hop_size = 128;
        assert_eq!(
            LiveAudioFeatureAnalyzer::new(config).unwrap_err(),
            LiveAudioFeatureError::InsufficientFrequencyResolution
        );
    }

    #[test]
    fn supports_eight_through_sixteen_logarithmic_bands() {
        for band_count in 8..=16 {
            let mut config = LiveAudioFeatureConfig::for_sample_rate(48_000);
            config.band_count = band_count;
            let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
            let samples = sine(48_000, 1_000.0, 0.2, 0.8);
            let mut last = None;
            analyzer.push_samples(&samples, |frame| last = Some(frame));
            let frame = last.unwrap();
            assert_eq!(frame.bands().len(), band_count);
            assert!(frame.bands().iter().all(|value| value.is_finite()));
            assert!(frame
                .bands()
                .iter()
                .all(|value| (0.0..=1.0).contains(value)));
        }
    }

    #[test]
    fn tones_are_localized_to_the_expected_log_band() {
        for expected_band in [2_usize, 8, 13] {
            let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
            let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
            let edges = analyzer.band_edges_hz();
            let frequency = (edges[expected_band] * edges[expected_band + 1]).sqrt();
            let samples = sine(48_000, frequency, 0.5, 0.8);
            let mut last = None;
            analyzer.push_samples(&samples, |frame| last = Some(frame));
            let frame = last.unwrap();
            let dominant_band = frame
                .bands()
                .iter()
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(right.1))
                .unwrap()
                .0;
            assert_eq!(dominant_band, expected_band, "frequency {frequency}");
        }
    }

    #[test]
    fn click_track_produces_onsets_and_converges_on_bpm() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let samples = click_track(48_000, 120.0, 10.0);
        let mut onset_count = 0;
        let mut last = None;
        analyzer.push_samples(&samples, |frame| {
            onset_count += usize::from(frame.onset);
            last = Some(frame);
        });
        let frame = last.unwrap();
        assert!((15..=21).contains(&onset_count), "onsets {onset_count}");
        assert!(
            (frame.bpm.unwrap() - 120.0).abs() < 2.0,
            "bpm {:?}",
            frame.bpm
        );
        assert!(
            frame.bpm_confidence >= 0.75,
            "confidence {}",
            frame.bpm_confidence
        );
    }

    #[test]
    fn tempo_range_boundaries_do_not_octave_flip() {
        for expected_bpm in [60.0_f32, 200.0] {
            let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
            let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
            let samples = click_track(48_000, expected_bpm, 10.0);
            let mut last = None;
            analyzer.push_samples(&samples, |frame| last = Some(frame));
            let frame = last.unwrap();
            assert!(
                (frame.bpm.unwrap() - expected_bpm).abs() < 3.0,
                "expected {expected_bpm}, got {:?}",
                frame.bpm
            );
            assert!(frame.bpm_confidence >= 0.6, "frame {frame:?}");
        }
    }

    #[test]
    fn tempo_confidence_expires_after_four_seconds_without_onsets() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let clicks = click_track(48_000, 120.0, 8.0);
        let silence = vec![0.0; 48_000 * 5];
        let mut before_silence = None;
        analyzer.push_samples(&clicks, |frame| before_silence = Some(frame));
        assert!(before_silence.unwrap().bpm.is_some());
        let mut after_silence = None;
        analyzer.push_samples(&silence, |frame| after_silence = Some(frame));
        let frame = after_silence.unwrap();
        assert_eq!(frame.bpm, None);
        assert_eq!(frame.bpm_confidence, 0.0);
    }

    #[test]
    fn rms_and_peak_remain_linear() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let samples = sine(48_000, 1_125.0, config.fft_size as f32 / 48_000.0, 0.5);
        let mut frame = None;
        analyzer.push_samples(&samples, |value| frame = Some(value));
        let frame = frame.unwrap();
        assert!((frame.rms - 0.5 / 2.0_f32.sqrt()).abs() < 0.002);
        assert!((frame.peak - 0.5).abs() < 0.001);
    }

    #[test]
    fn steady_tone_does_not_retrigger_as_a_beat_train() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let samples = sine(48_000, 440.0, 5.0, 0.7);
        let mut onset_count = 0;
        let mut maximum_flux = 0.0_f32;
        analyzer.push_samples(&samples, |frame| {
            onset_count += usize::from(frame.onset);
            maximum_flux = maximum_flux.max(frame.spectral_flux);
        });
        assert!(onset_count <= 1, "onsets {onset_count}");
        assert!(maximum_flux <= 1.0);
    }

    #[test]
    fn results_do_not_depend_on_input_chunk_boundaries() {
        let samples = click_track(48_000, 128.0, 6.0);
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut contiguous = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let mut chunked = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let mut contiguous_frames = Vec::new();
        let mut chunked_frames = Vec::new();
        contiguous.push_samples(&samples, |frame| contiguous_frames.push(frame));
        let mut start = 0;
        let chunk_pattern = [1, 17, 480, 1_024, 333, 2_047];
        let mut pattern_index = 0;
        while start < samples.len() {
            let end =
                (start + chunk_pattern[pattern_index % chunk_pattern.len()]).min(samples.len());
            chunked.push_samples(&samples[start..end], |frame| chunked_frames.push(frame));
            start = end;
            pattern_index += 1;
        }
        assert_eq!(contiguous_frames, chunked_frames);
    }

    #[test]
    fn non_finite_input_is_silence_and_storage_is_reused() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let storage = (
            analyzer.sample_ring.as_ptr(),
            analyzer.hann.as_ptr(),
            analyzer.fft_real.as_ptr(),
            analyzer.fft_imaginary.as_ptr(),
            analyzer.magnitudes.as_ptr(),
            analyzer.previous_magnitudes.as_ptr(),
            analyzer.band_for_bin.as_ptr(),
        );
        let samples = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY]
            .into_iter()
            .cycle()
            .take(config.fft_size + config.hop_size * 4)
            .collect::<Vec<_>>();
        let mut frames = 0;
        analyzer.push_samples(&samples, |frame| {
            frames += 1;
            assert_eq!(frame.rms, 0.0);
            assert_eq!(frame.peak, 0.0);
            assert_eq!(frame.spectral_flux, 0.0);
            assert!(!frame.onset);
            assert!(frame.bands().iter().all(|level| *level == 0.0));
        });
        assert_eq!(frames, 5);
        assert_eq!(
            storage,
            (
                analyzer.sample_ring.as_ptr(),
                analyzer.hann.as_ptr(),
                analyzer.fft_real.as_ptr(),
                analyzer.fft_imaginary.as_ptr(),
                analyzer.magnitudes.as_ptr(),
                analyzer.previous_magnitudes.as_ptr(),
                analyzer.band_for_bin.as_ptr(),
            )
        );
    }

    #[test]
    fn reset_retains_allocations_and_restarts_sequence() {
        let config = LiveAudioFeatureConfig::for_sample_rate(48_000);
        let mut analyzer = LiveAudioFeatureAnalyzer::new(config).unwrap();
        let ring = analyzer.sample_ring.as_ptr();
        let samples = sine(48_000, 1_000.0, 0.1, 0.5);
        analyzer.push_samples(&samples, |_| {});
        analyzer.reset();
        assert_eq!(ring, analyzer.sample_ring.as_ptr());
        let mut first = None;
        analyzer.push_samples(&samples, |frame| {
            first.get_or_insert(frame);
        });
        assert_eq!(first.unwrap().sequence, 0);
    }
}
