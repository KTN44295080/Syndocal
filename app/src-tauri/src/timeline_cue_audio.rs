//! Sample-frame Timeline click/Guide mixer and machine-local settings core.
//!
//! The audio callback owns the frame counter and fixed voice slots. Producers
//! may allocate, decode, persist settings, and publish Copy-only descriptors,
//! but [`TimelineCueAudioSource::next`] performs no filesystem/device/engine
//! work and takes no blocking lock.

use rodio::{ChannelCount, SampleRate, Source};
use serde::{Deserialize, Serialize};
use std::{
    cell::UnsafeCell,
    ffi::OsStr,
    fs,
    io::{Cursor, Read, Write},
    mem::MaybeUninit,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const TIMELINE_CUE_CANONICAL_SAMPLE_RATE: u32 = 48_000;
pub const TIMELINE_CUE_SETTINGS_FILE: &str = "timeline-cue-audio-settings.json";
pub const MAX_TIMELINE_CUE_SETTINGS_BYTES: u64 = 16 * 1024;
pub const MAX_TIMELINE_CUE_DEVICE_NAME_BYTES: usize = 512;
pub const MAX_TIMELINE_CUE_TOPOLOGY_FINGERPRINT_BYTES: usize = 128;
pub const MAX_TIMELINE_CUE_QUEUE_CAPACITY: usize = 64;
pub const MIN_TIMELINE_CUE_OUTPUT_SAMPLE_RATE: u32 = 8_000;
pub const MAX_TIMELINE_CUE_OUTPUT_SAMPLE_RATE: u32 = 384_000;
pub const MAX_TIMELINE_CUE_OUTPUT_CHANNELS: u16 = 32;
pub const MAX_TIMELINE_GUIDE_DURATION_SECONDS: u64 = 30;
const MAX_CLICK_VOICES: usize = 8;
// Guide words are a single logical voice.  Additional events are serialized
// through the fixed pending array instead of being mixed on top of one another.
const MAX_GUIDE_VOICES: usize = 1;
const MAX_PENDING_GUIDE_EVENTS: usize = MAX_TIMELINE_CUE_QUEUE_CAPACITY;
const MAX_DUE_EVENTS_PER_FRAME: usize = MAX_TIMELINE_CUE_QUEUE_CAPACITY + 1;
const CLICK_DURATION_MS: u64 = 50;
const CLICK_DECAY_MS: u64 = 45;
const GAIN_SLEW_MS: u64 = 10;
const CLICK_ACCENT_FREQUENCY_HZ: f64 = 1_320.0;
const CLICK_REGULAR_FREQUENCY_HZ: f64 = 920.0;
const CLICK_ENVELOPE_START: f32 = 0.1;
const CLICK_ENVELOPE_END: f32 = 0.0001;
pub const TIMELINE_GUIDE_PLAYBACK_RATE_MIN_MILLI: u16 = 920;
pub const TIMELINE_GUIDE_PLAYBACK_RATE_MAX_MILLI: u16 = 1080;
pub const TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI: u16 = 1000;
#[cfg(test)]
const GUIDE_PLAYBACK_RATE_MIN_MILLI: u16 = TIMELINE_GUIDE_PLAYBACK_RATE_MIN_MILLI;
#[cfg(test)]
const GUIDE_PLAYBACK_RATE_MAX_MILLI: u16 = TIMELINE_GUIDE_PLAYBACK_RATE_MAX_MILLI;
#[cfg(test)]
const GUIDE_PLAYBACK_RATE_DEFAULT_MILLI: u16 = TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI;
// Leave one complete callback frame between the producer's linearization
// snapshot and an event's due frame.  `next_output_frame` names the next frame
// the callback will render, so two frames are required here: one may be
// consumed concurrently while the Copy-only batch is being published and one
// remains schedulable after publication.
const MIN_EVENT_LOOKAHEAD_FRAMES: u64 = 2;
const TIMELINE_GUIDE_ASSET_COUNT: usize = 12;
const MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS: f32 = 0.6;
// The authored metronome emits one click per timeline frame.  Duplicate click
// events are still bounded by MAX_CLICK_VOICES and are checked at render time.
const THEORETICAL_CLICK_PEAK: f64 = CLICK_ENVELOPE_START as f64;
static TIMELINE_CUE_SETTINGS_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineCueAudioRoute {
    FollowProgram,
    ExplicitDevice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineTimelineCueAudioSettingsV1 {
    pub version: u32,
    pub route: TimelineCueAudioRoute,
    pub device_name: Option<String>,
    /// Fingerprint of the exact output-device name multiset selected by the
    /// operator. Older v1 files decode with `None` and remain fail-closed on
    /// ExplicitDevice until the operator explicitly reselects the endpoint.
    #[serde(default)]
    pub topology_fingerprint: Option<String>,
    pub click_gain: f32,
    pub guide_gain: f32,
}

impl Default for MachineTimelineCueAudioSettingsV1 {
    fn default() -> Self {
        Self {
            version: 1,
            route: TimelineCueAudioRoute::FollowProgram,
            device_name: None,
            topology_fingerprint: None,
            click_gain: 1.0,
            guide_gain: 0.85,
        }
    }
}

impl MachineTimelineCueAudioSettingsV1 {
    pub fn validated(self) -> Result<Self, String> {
        if self.version != 1 {
            return Err(format!(
                "Unsupported Timeline cue audio settings version {}",
                self.version
            ));
        }
        validate_gain("Timeline click", self.click_gain)?;
        validate_gain("Timeline Guide", self.guide_gain)?;
        validate_mixer_headroom(
            self.click_gain,
            self.guide_gain,
            MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS,
        )?;
        if let Some(name) = self.device_name.as_deref() {
            if name.is_empty() || name.trim() != name {
                return Err(
                    "Timeline cue audio device name must be non-empty and canonical".to_string(),
                );
            }
            if name.len() > MAX_TIMELINE_CUE_DEVICE_NAME_BYTES {
                return Err(format!(
                    "Timeline cue audio device name exceeds {} bytes",
                    MAX_TIMELINE_CUE_DEVICE_NAME_BYTES
                ));
            }
            if name.chars().any(char::is_control) {
                return Err(
                    "Timeline cue audio device name contains a control character".to_string(),
                );
            }
        }
        if let Some(fingerprint) = self.topology_fingerprint.as_deref() {
            if fingerprint.is_empty() || fingerprint.trim() != fingerprint {
                return Err(
                    "Timeline cue audio topology fingerprint must be non-empty and canonical"
                        .to_string(),
                );
            }
            if fingerprint.len() > MAX_TIMELINE_CUE_TOPOLOGY_FINGERPRINT_BYTES
                || fingerprint.chars().any(char::is_control)
            {
                return Err("Timeline cue audio topology fingerprint is invalid".to_string());
            }
        }
        match (
            self.route,
            self.device_name.is_some(),
            self.topology_fingerprint.is_some(),
        ) {
            (TimelineCueAudioRoute::FollowProgram, false, false)
            | (TimelineCueAudioRoute::ExplicitDevice, true, _) => Ok(self),
            (TimelineCueAudioRoute::FollowProgram, _, _) => Err(
                "Follow Program Timeline cue audio route must not name a separate device"
                    .to_string(),
            ),
            (TimelineCueAudioRoute::ExplicitDevice, false, _) => {
                Err("Explicit Timeline cue audio route requires an exact device name".to_string())
            }
        }
    }
}

fn validate_gain(label: &str, gain: f32) -> Result<(), String> {
    if !gain.is_finite() {
        return Err(format!("{label} gain must be finite"));
    }
    if !(0.0..=2.0).contains(&gain) {
        return Err(format!("{label} gain must be between 0 and 2"));
    }
    Ok(())
}

fn validate_mixer_headroom(
    click_gain: f32,
    guide_gain: f32,
    guide_peak_abs: f32,
) -> Result<(), String> {
    validate_gain("Timeline click", click_gain)?;
    validate_gain("Timeline Guide", guide_gain)?;
    if !guide_peak_abs.is_finite() || guide_peak_abs < 0.0 {
        return Err("Timeline Guide bank peak must be finite and non-negative".to_string());
    }
    let theoretical_peak = THEORETICAL_CLICK_PEAK * f64::from(click_gain)
        + f64::from(guide_peak_abs) * f64::from(guide_gain);
    if !theoretical_peak.is_finite() {
        return Err("Timeline cue audio theoretical peak is non-finite".to_string());
    }
    if theoretical_peak > 1.0 {
        return Err(format!(
            "Timeline cue audio gains exceed safe headroom (theoretical peak {theoretical_peak:.6})"
        ));
    }
    Ok(())
}

pub fn timeline_cue_audio_settings_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join(TIMELINE_CUE_SETTINGS_FILE)
}

pub fn load_timeline_cue_audio_settings_from_path(
    path: &Path,
) -> Result<MachineTimelineCueAudioSettingsV1, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MachineTimelineCueAudioSettingsV1::default())
        }
        Err(error) => {
            return Err(format!(
                "Unable to read Timeline cue audio settings {}: {error}",
                path.display()
            ))
        }
    };
    let declared_len = file
        .metadata()
        .map_err(|error| {
            format!(
                "Unable to inspect Timeline cue audio settings {}: {error}",
                path.display()
            )
        })?
        .len();
    if declared_len > MAX_TIMELINE_CUE_SETTINGS_BYTES {
        return Err(format!(
            "Timeline cue audio settings {} exceed the {} byte safety limit",
            path.display(),
            MAX_TIMELINE_CUE_SETTINGS_BYTES
        ));
    }
    let read_limit = MAX_TIMELINE_CUE_SETTINGS_BYTES
        .checked_add(1)
        .ok_or_else(|| "Timeline cue audio settings byte limit is exhausted".to_string())?;
    let mut bytes = Vec::with_capacity(usize::try_from(declared_len).unwrap_or(0));
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "Unable to read Timeline cue audio settings {}: {error}",
                path.display()
            )
        })?;
    if bytes.len() as u64 > MAX_TIMELINE_CUE_SETTINGS_BYTES {
        return Err(format!(
            "Timeline cue audio settings {} grew beyond the {} byte safety limit",
            path.display(),
            MAX_TIMELINE_CUE_SETTINGS_BYTES
        ));
    }
    serde_json::from_slice::<MachineTimelineCueAudioSettingsV1>(&bytes)
        .map_err(|error| {
            format!(
                "Timeline cue audio settings {} are corrupt: {error}",
                path.display()
            )
        })?
        .validated()
}

pub fn persist_timeline_cue_audio_settings_to_path(
    path: &Path,
    settings: &MachineTimelineCueAudioSettingsV1,
) -> Result<(), String> {
    persist_timeline_cue_audio_settings_to_path_with(path, settings, |temporary, target| {
        super::replace_file_atomically(temporary, target)
    })
}

pub fn persist_timeline_cue_audio_settings_to_path_with(
    path: &Path,
    settings: &MachineTimelineCueAudioSettingsV1,
    replace: impl FnOnce(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let settings = settings.clone().validated()?;
    let bytes = serde_json::to_vec(&settings)
        .map_err(|error| format!("Unable to encode Timeline cue audio settings: {error}"))?;
    if bytes.len() as u64 > MAX_TIMELINE_CUE_SETTINGS_BYTES {
        return Err(format!(
            "Timeline cue audio settings exceed the {} byte safety limit",
            MAX_TIMELINE_CUE_SETTINGS_BYTES
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Timeline cue audio settings path has no parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Unable to create Timeline cue audio settings directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(TIMELINE_CUE_SETTINGS_FILE);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        TIMELINE_CUE_SETTINGS_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "Unable to create temporary Timeline cue audio settings {}: {error}",
                    temporary.display()
                )
            })?;
        file.write_all(&bytes).map_err(|error| {
            format!(
                "Unable to write temporary Timeline cue audio settings {}: {error}",
                temporary.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "Unable to flush temporary Timeline cue audio settings {}: {error}",
                temporary.display()
            )
        })?;
        drop(file);
        replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TimelineGuideAssetKey {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Interlude,
    Bridge,
    Breakdown,
    Outro,
    Looping,
    Break,
    Trans,
    Complete,
}

impl TimelineGuideAssetKey {
    pub const ALL: [Self; TIMELINE_GUIDE_ASSET_COUNT] = [
        Self::Intro,
        Self::Verse,
        Self::PreChorus,
        Self::Chorus,
        Self::Interlude,
        Self::Bridge,
        Self::Breakdown,
        Self::Outro,
        Self::Looping,
        Self::Break,
        Self::Trans,
        Self::Complete,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedTimelineGuideAsset {
    pub key: TimelineGuideAssetKey,
    pub file_name: &'static str,
    pub sha256: &'static str,
    pub source_sample_rate: u32,
    pub source_frames: u64,
    pub output_48k_frames: u64,
    pub wav: &'static [u8],
}

pub const TIMELINE_GUIDE_EMBEDDED_ASSETS: [EmbeddedTimelineGuideAsset; TIMELINE_GUIDE_ASSET_COUNT] = [
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Intro,
        file_name: "intro.wav",
        sha256: "D0E2F402E231DA5DDAD2427E306DCAF9B2A243A6EBCC2E33EA1A7347DFD4FA15",
        source_sample_rate: 22_050,
        source_frames: 23_250,
        output_48k_frames: 50_612,
        wav: include_bytes!("../assets/timeline-guide/en/intro.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Verse,
        file_name: "verse.wav",
        sha256: "D19CD381BB82C5086990E43B68537976001956112504C497AF1C7089163E8F84",
        source_sample_rate: 22_050,
        source_frames: 22_808,
        output_48k_frames: 49_650,
        wav: include_bytes!("../assets/timeline-guide/en/verse.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::PreChorus,
        file_name: "pre_chorus.wav",
        sha256: "97E0A27F0467EEF4CC809C93DE7AD9DA28F8D7FE6EF2A75F066A82273E8908EF",
        source_sample_rate: 22_050,
        source_frames: 28_871,
        output_48k_frames: 62_848,
        wav: include_bytes!("../assets/timeline-guide/en/pre_chorus.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Chorus,
        file_name: "chorus.wav",
        sha256: "3CF0F90CA4F06A7BB54ACDA1D153CD21CC929631AE033E5B19B883BF820EAA9C",
        source_sample_rate: 22_050,
        source_frames: 26_448,
        output_48k_frames: 57_574,
        wav: include_bytes!("../assets/timeline-guide/en/chorus.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Interlude,
        file_name: "interlude.wav",
        sha256: "695E2ADDF83D1F3C236E14B57BF20124269B587A098293293A4EE9CAB99EAE21",
        source_sample_rate: 22_050,
        source_frames: 25_785,
        output_48k_frames: 56_131,
        wav: include_bytes!("../assets/timeline-guide/en/interlude.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Bridge,
        file_name: "bridge.wav",
        sha256: "3F30A92AE768E7054C1CE88C6A1B870C8373958F7C10473D609DF10CA3B902C2",
        source_sample_rate: 22_050,
        source_frames: 24_573,
        output_48k_frames: 53_492,
        wav: include_bytes!("../assets/timeline-guide/en/bridge.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Breakdown,
        file_name: "breakdown.wav",
        sha256: "F8618ED41272E0802083DBF70308C4FC4DE21F8C8E3155D0C6627391930A5981",
        source_sample_rate: 22_050,
        source_frames: 28_099,
        output_48k_frames: 61_168,
        wav: include_bytes!("../assets/timeline-guide/en/breakdown.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Outro,
        file_name: "outro.wav",
        sha256: "443FB54C20E2BD9E8F283CA522BC3584192000BB9BB5105A1AF9131D48F31A2B",
        source_sample_rate: 22_050,
        source_frames: 23_691,
        output_48k_frames: 51_572,
        wav: include_bytes!("../assets/timeline-guide/en/outro.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Looping,
        file_name: "looping.wav",
        sha256: "C1A9CDF97339AD658B299329AC3A326FDB78D3EDBE76281441036800C9455E39",
        source_sample_rate: 22_050,
        source_frames: 25_013,
        output_48k_frames: 54_450,
        wav: include_bytes!("../assets/timeline-guide/en/looping.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Break,
        file_name: "break_word.wav",
        sha256: "064051A6E3C1E077AC2D97073A7F927D8D004CF5A45195D13EF699A14BFF49B0",
        source_sample_rate: 22_050,
        source_frames: 23_360,
        output_48k_frames: 50_852,
        wav: include_bytes!("../assets/timeline-guide/en/break_word.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Trans,
        file_name: "trans.wav",
        sha256: "AFA787E27BC34625C914E6B7E4FED9D027334DB83980A4D6F5A8A4DCEE444B69",
        source_sample_rate: 22_050,
        source_frames: 27_219,
        output_48k_frames: 59_252,
        wav: include_bytes!("../assets/timeline-guide/en/trans.wav"),
    },
    EmbeddedTimelineGuideAsset {
        key: TimelineGuideAssetKey::Complete,
        file_name: "complete.wav",
        sha256: "3298C2CA19FB8BAA2C25C731D67702249C1E71EE00D599D6FFB3D27F55E1CC5C",
        source_sample_rate: 22_050,
        source_frames: 26_556,
        output_48k_frames: 57_809,
        wav: include_bytes!("../assets/timeline-guide/en/complete.wav"),
    },
];

#[derive(Debug)]
pub struct DecodedGuideAsset {
    key: TimelineGuideAssetKey,
    sample_rate: u32,
    channels: u16,
    samples: Box<[f32]>,
}

impl DecodedGuideAsset {
    pub fn decode_wav(key: TimelineGuideAssetKey, bytes: &'static [u8]) -> Result<Self, String> {
        let decoder = rodio::Decoder::try_from(Cursor::new(bytes)).map_err(|error| {
            format!("Timeline Guide asset {key:?} could not be decoded: {error}")
        })?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let max_samples = u64::from(sample_rate)
            .checked_mul(u64::from(channels))
            .and_then(|value| value.checked_mul(MAX_TIMELINE_GUIDE_DURATION_SECONDS))
            .ok_or_else(|| format!("Timeline Guide asset {key:?} format is too large"))?;
        let take = usize::try_from(max_samples.saturating_add(1))
            .map_err(|_| format!("Timeline Guide asset {key:?} format is too large"))?;
        let samples = decoder.take(take).collect::<Vec<_>>().into_boxed_slice();
        if samples.len() as u64 > max_samples {
            return Err(format!(
                "Timeline Guide asset {key:?} exceeds {} seconds",
                MAX_TIMELINE_GUIDE_DURATION_SECONDS
            ));
        }
        Self::new(key, sample_rate, channels, samples)
    }

    pub fn from_mono_pcm(
        key: TimelineGuideAssetKey,
        sample_rate: u32,
        samples: Vec<f32>,
    ) -> Result<Self, String> {
        Self::new(key, sample_rate, 1, samples.into_boxed_slice())
    }

    fn new(
        key: TimelineGuideAssetKey,
        sample_rate: u32,
        channels: u16,
        samples: Box<[f32]>,
    ) -> Result<Self, String> {
        if sample_rate == 0 || channels == 0 {
            return Err(format!(
                "Timeline Guide asset {key:?} has an invalid audio format"
            ));
        }
        if samples.is_empty() || !samples.len().is_multiple_of(usize::from(channels)) {
            return Err(format!(
                "Timeline Guide asset {key:?} has invalid interleaved PCM"
            ));
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(format!(
                "Timeline Guide asset {key:?} contains a non-finite sample"
            ));
        }
        Ok(Self {
            key,
            sample_rate,
            channels,
            samples,
        })
    }
}

#[derive(Debug)]
struct GuidePcmAsset {
    samples: Box<[f32]>,
}

#[derive(Debug)]
pub struct GuideAssetBank {
    output_sample_rate: u32,
    assets: Box<[GuidePcmAsset]>,
    peak_abs: f32,
}

impl GuideAssetBank {
    pub fn prepare_embedded(output_sample_rate: u32) -> Result<Self, String> {
        let decoded = TIMELINE_GUIDE_EMBEDDED_ASSETS
            .iter()
            .map(|asset| DecodedGuideAsset::decode_wav(asset.key, asset.wav))
            .collect::<Result<Vec<_>, _>>()?;
        Self::prepare_complete(output_sample_rate, decoded)
    }

    pub fn prepare_complete(
        output_sample_rate: u32,
        decoded: Vec<DecodedGuideAsset>,
    ) -> Result<Self, String> {
        validate_output_format(output_sample_rate, 1)?;
        if decoded.len() != TIMELINE_GUIDE_ASSET_COUNT {
            return Err(format!(
                "Timeline Guide asset catalog must contain exactly {} assets",
                TIMELINE_GUIDE_ASSET_COUNT
            ));
        }
        let mut slots = (0..TIMELINE_GUIDE_ASSET_COUNT)
            .map(|_| None)
            .collect::<Vec<Option<GuidePcmAsset>>>();
        for asset in decoded {
            let index = asset.key.index();
            if slots[index].is_some() {
                return Err(format!(
                    "Timeline Guide asset catalog contains duplicate {:?}",
                    asset.key
                ));
            }
            let mono = downmix_and_resample_guide_asset(&asset, output_sample_rate)?;
            slots[index] = Some(GuidePcmAsset {
                samples: mono.into_boxed_slice(),
            });
        }
        let missing = TimelineGuideAssetKey::ALL
            .into_iter()
            .find(|key| slots[key.index()].is_none());
        if let Some(key) = missing {
            return Err(format!("Timeline Guide asset catalog is missing {key:?}"));
        }
        let peak_abs = slots
            .iter()
            .flatten()
            .flat_map(|asset| asset.samples.iter())
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        if !peak_abs.is_finite() {
            return Err("Timeline Guide asset bank peak is non-finite".to_string());
        }
        if peak_abs > MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS {
            return Err(format!(
                "Timeline Guide asset bank peak {peak_abs:.6} exceeds the exporter contract {:.6}",
                MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS
            ));
        }
        Ok(Self {
            output_sample_rate,
            assets: slots
                .into_iter()
                .map(|asset| asset.expect("all Guide asset slots were validated"))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            peak_abs,
        })
    }

    pub fn output_sample_rate(&self) -> u32 {
        self.output_sample_rate
    }

    pub fn peak_abs(&self) -> f32 {
        self.peak_abs
    }

    fn samples(&self, key: TimelineGuideAssetKey) -> &[f32] {
        &self.assets[key.index()].samples
    }
}

fn downmix_and_resample_guide_asset(
    asset: &DecodedGuideAsset,
    output_sample_rate: u32,
) -> Result<Vec<f32>, String> {
    let channels = usize::from(asset.channels);
    let input_frames = asset.samples.len() / channels;
    let mut mono = Vec::with_capacity(input_frames);
    for frame in asset.samples.chunks_exact(channels) {
        let sample =
            frame.iter().map(|sample| f64::from(*sample)).sum::<f64>() / f64::from(asset.channels);
        let sample = sample as f32;
        if !sample.is_finite() {
            return Err(format!(
                "Timeline Guide asset {:?} overflows while downmixing",
                asset.key
            ));
        }
        mono.push(sample);
    }
    resample_mono_linear_f64(&mono, asset.sample_rate, output_sample_rate)
        .map_err(|error| format!("Timeline Guide asset {:?} {error}", asset.key))
}

/// Resample using the exporter contract shared by the offline guide builder:
/// outputLen = round(inputFrames * destinationRate / sourceRate), with a
/// half-up integer round for positive frame counts and f64 interpolation.
fn resample_mono_linear_f64(
    input: &[f32],
    source_sample_rate: u32,
    destination_sample_rate: u32,
) -> Result<Vec<f32>, String> {
    if source_sample_rate == 0 || destination_sample_rate == 0 {
        return Err("resampler sample rates must be non-zero".to_string());
    }
    if input.iter().any(|sample| !sample.is_finite()) {
        return Err("resampler input contains a non-finite sample".to_string());
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if source_sample_rate == destination_sample_rate {
        return Ok(input.to_vec());
    }
    let numerator = (input.len() as u128)
        .checked_mul(destination_sample_rate as u128)
        .ok_or_else(|| "resampler frame count overflowed".to_string())?;
    let rounded = numerator
        .checked_add((source_sample_rate / 2) as u128)
        .ok_or_else(|| "resampler frame count overflowed".to_string())?
        / source_sample_rate as u128;
    let output_len = usize::try_from(rounded)
        .map_err(|_| "resampler output frame count is too large".to_string())?;
    let mut output = Vec::with_capacity(output_len);
    let source_rate = f64::from(source_sample_rate);
    let destination_rate = f64::from(destination_sample_rate);
    for output_index in 0..output_len {
        let position = output_index as f64 * source_rate / destination_rate;
        let left = position.floor() as usize;
        let fraction = position - left as f64;
        let left_sample = f64::from(input[left.min(input.len() - 1)]);
        let right_sample = f64::from(input[(left + 1).min(input.len() - 1)]);
        let sample = left_sample + (right_sample - left_sample) * fraction;
        let sample = sample as f32;
        if !sample.is_finite() {
            return Err("resampler output contains a non-finite sample".to_string());
        }
        output.push(sample);
    }
    Ok(output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimelineCueFence {
    pub output_clock_epoch: u64,
    pub schedule_generation: u64,
    pub source_fence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineCueClockMap {
    pub canonical_anchor_frame: u64,
    pub output_anchor_frame: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineCueAuthority {
    pub fence: TimelineCueFence,
    pub clock: TimelineCueClockMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineCueEventKind {
    Click { accented: bool },
    Guide { asset: TimelineGuideAssetKey },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineCueEvent {
    pub canonical_frame: u64,
    pub sequence: u64,
    pub kind: TimelineCueEventKind,
    /// Fixed-point guide playback rate: 1000 = 1.00x, 1040 = 1.04x.
    /// Click events carry the same field for one uniform producer contract.
    pub playback_rate_milli: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledTimelineCueEvent {
    fence: TimelineCueFence,
    output_frame: u64,
    sequence: u64,
    kind: TimelineCueEventKind,
    playback_rate_milli: u16,
}

const SPSC_SLOT_EMPTY: u8 = 0;
const SPSC_SLOT_WRITING: u8 = 1;
const SPSC_SLOT_READY: u8 = 2;
const SPSC_SLOT_READING: u8 = 3;

/// Fixed-capacity, single-producer/single-consumer event ring.
///
/// The producer reserves and fills every slot in a batch before one Release
/// store advances `published_tail`; the callback therefore observes all or
/// none of a batch.  [`Self::pop_once`] performs exactly one bounded slot CAS
/// and contains no retry/backoff path.
#[derive(Debug)]
struct TimelineCueSpscQueue {
    slots: Box<[TimelineCueSpscSlot]>,
    capacity: u64,
    head: AtomicU64,
    published_tail: AtomicU64,
}

#[derive(Debug)]
struct TimelineCueSpscSlot {
    state: AtomicU8,
    event: UnsafeCell<MaybeUninit<ScheduledTimelineCueEvent>>,
}

#[derive(Debug, Clone, Copy)]
struct StagedTimelineCueBatch {
    tail: u64,
    count: u64,
}

// A producer is serialized by TimelineCueAudioControl::producer and there is
// exactly one callback Source per output-clock epoch.  Slot state transitions
// prevent either side from touching `event` while the other owns it.
unsafe impl Send for TimelineCueSpscSlot {}
unsafe impl Sync for TimelineCueSpscSlot {}

impl TimelineCueSpscQueue {
    fn new(capacity: usize) -> Self {
        Self {
            slots: (0..capacity)
                .map(|_| TimelineCueSpscSlot {
                    state: AtomicU8::new(SPSC_SLOT_EMPTY),
                    event: UnsafeCell::new(MaybeUninit::uninit()),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            capacity: capacity as u64,
            head: AtomicU64::new(0),
            published_tail: AtomicU64::new(0),
        }
    }

    fn capacity(&self) -> usize {
        self.capacity as usize
    }

    fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.published_tail.load(Ordering::Acquire);
        usize::try_from(tail.saturating_sub(head).min(self.capacity)).unwrap_or(self.capacity())
    }

    fn stage_batch(
        &self,
        events: &[ScheduledTimelineCueEvent],
    ) -> Result<StagedTimelineCueBatch, ()> {
        if events.is_empty() {
            return Ok(StagedTimelineCueBatch {
                tail: self.published_tail.load(Ordering::Acquire),
                count: 0,
            });
        }
        let head = self.head.load(Ordering::Acquire);
        let tail = self.published_tail.load(Ordering::Relaxed);
        let occupied = tail.checked_sub(head).ok_or(())?;
        let event_count = events.len() as u64;
        if occupied > self.capacity || event_count > self.capacity.saturating_sub(occupied) {
            return Err(());
        }
        let committed_tail = tail.checked_add(event_count).ok_or(())?;

        let mut reserved = 0_u64;
        while reserved < event_count {
            let slot = &self.slots[((tail + reserved) % self.capacity) as usize];
            if slot
                .state
                .compare_exchange(
                    SPSC_SLOT_EMPTY,
                    SPSC_SLOT_WRITING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
            {
                for rollback in 0..reserved {
                    let slot = &self.slots[((tail + rollback) % self.capacity) as usize];
                    slot.state.store(SPSC_SLOT_EMPTY, Ordering::Release);
                }
                return Err(());
            }
            reserved += 1;
        }

        for (offset, event) in events.iter().copied().enumerate() {
            let slot = &self.slots[((tail + offset as u64) % self.capacity) as usize];
            // SAFETY: this producer changed EMPTY -> WRITING and the consumer
            // cannot observe the slot before the single tail publication.
            unsafe { (*slot.event.get()).write(event) };
            slot.state.store(SPSC_SLOT_READY, Ordering::Release);
        }
        Ok(StagedTimelineCueBatch {
            tail,
            count: committed_tail - tail,
        })
    }

    fn publish_staged(&self, staged: StagedTimelineCueBatch) {
        self.published_tail
            .store(staged.tail + staged.count, Ordering::Release);
    }

    fn rollback_staged(&self, staged: StagedTimelineCueBatch) {
        self.published_tail.store(staged.tail, Ordering::Release);
        for offset in 0..staged.count {
            let slot = &self.slots[((staged.tail + offset) % self.capacity) as usize];
            let _ = slot.state.compare_exchange(
                SPSC_SLOT_READY,
                SPSC_SLOT_EMPTY,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    #[cfg(test)]
    fn push_batch(&self, events: &[ScheduledTimelineCueEvent]) -> Result<(), ()> {
        let staged = self.stage_batch(events)?;
        self.publish_staged(staged);
        Ok(())
    }

    fn pop_once(&self, event_publication_state: &AtomicU8) -> Option<ScheduledTimelineCueEvent> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.published_tail.load(Ordering::Acquire);
        if head >= tail {
            return None;
        }
        // This Acquire must follow the published-tail Acquire. Seeing a new
        // tail with stable state 0/2 therefore implies that the producer's
        // final commit won. A WRITING tail is invalidated before any slot CAS,
        // so an uncommitted descriptor can never escape into Source::pending.
        let publication_state = event_publication_state.load(Ordering::Acquire);
        match publication_state {
            0 | 2 => {}
            writing @ (1 | 3) => {
                match event_publication_state.compare_exchange(
                    writing,
                    4,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return None,
                    Err(0 | 2) => {}
                    Err(_) => return None,
                }
            }
            _ => return None,
        }
        let slot = &self.slots[(head % self.capacity) as usize];
        if slot
            .state
            .compare_exchange(
                SPSC_SLOT_READY,
                SPSC_SLOT_READING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return None;
        }
        // SAFETY: READING ownership excludes the producer until EMPTY is
        // published. ScheduledTimelineCueEvent is Copy, so the slot remains
        // initialized for the duration of this read.
        let event = unsafe { *(*slot.event.get()).assume_init_ref() };
        slot.state.store(SPSC_SLOT_EMPTY, Ordering::Release);
        if self
            .head
            .compare_exchange(head, head + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(event)
        } else {
            None
        }
    }

    /// Bounded producer-side drain used only while authority_sequence is odd.
    fn clear(&self) {
        let tail = self.published_tail.load(Ordering::Acquire);
        let head = self.head.swap(tail, Ordering::AcqRel);
        let count = tail.saturating_sub(head).min(self.capacity);
        for offset in 0..count {
            let slot = &self.slots[((head + offset) % self.capacity) as usize];
            let _ = slot.state.compare_exchange(
                SPSC_SLOT_READY,
                SPSC_SLOT_EMPTY,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimelineCueFaultCode {
    None,
    InvalidFormat,
    AuthorityUnstable,
    AuthorityExhausted,
    FrameMappingOverflow,
    QueueFull,
    OutOfOrder,
    PastDue,
    StaleFence,
    ActiveVoiceOverflow,
    OutputFrameExhausted,
    OutputOverUnity,
    NonFiniteOutput,
    GuideRateInvalid,
}

impl TimelineCueFaultCode {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::InvalidFormat,
            2 => Self::AuthorityUnstable,
            3 => Self::AuthorityExhausted,
            4 => Self::FrameMappingOverflow,
            5 => Self::QueueFull,
            6 => Self::OutOfOrder,
            7 => Self::PastDue,
            8 => Self::StaleFence,
            9 => Self::ActiveVoiceOverflow,
            10 => Self::OutputFrameExhausted,
            11 => Self::OutputOverUnity,
            12 => Self::NonFiniteOutput,
            13 => Self::GuideRateInvalid,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineCueFaultStatus {
    pub code: TimelineCueFaultCode,
    pub count: u64,
    pub sequence: u64,
}

#[derive(Debug)]
struct TimelineCueShared {
    retired: AtomicU8,
    authority_sequence: AtomicU64,
    // 0 = not started, 1 = pre-start publication, 2 = started, 3 = running publication.
    event_publication_state: AtomicU8,
    output_clock_epoch: AtomicU64,
    schedule_generation: AtomicU64,
    source_fence: AtomicU64,
    canonical_anchor_frame: AtomicU64,
    output_anchor_frame: AtomicU64,
    next_output_frame: AtomicU64,
    gain_sequence: AtomicU64,
    click_gain_bits: AtomicU32,
    guide_gain_bits: AtomicU32,
    fault_code: AtomicU8,
    fault_count: AtomicU64,
    fault_sequence: AtomicU64,
    fault_snapshot_sequence: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct TimelineCuePublicationToken {
    started: bool,
    writing_state: u8,
}

impl TimelineCueShared {
    fn new(authority: TimelineCueAuthority, click_gain: f32, guide_gain: f32) -> Self {
        Self {
            retired: AtomicU8::new(0),
            authority_sequence: AtomicU64::new(0),
            event_publication_state: AtomicU8::new(0),
            output_clock_epoch: AtomicU64::new(authority.fence.output_clock_epoch),
            schedule_generation: AtomicU64::new(authority.fence.schedule_generation),
            source_fence: AtomicU64::new(authority.fence.source_fence),
            canonical_anchor_frame: AtomicU64::new(authority.clock.canonical_anchor_frame),
            output_anchor_frame: AtomicU64::new(authority.clock.output_anchor_frame),
            next_output_frame: AtomicU64::new(0),
            gain_sequence: AtomicU64::new(0),
            click_gain_bits: AtomicU32::new(click_gain.to_bits()),
            guide_gain_bits: AtomicU32::new(guide_gain.to_bits()),
            fault_code: AtomicU8::new(TimelineCueFaultCode::None as u8),
            fault_count: AtomicU64::new(0),
            fault_sequence: AtomicU64::new(0),
            fault_snapshot_sequence: AtomicU64::new(0),
        }
    }

    fn read_authority(&self) -> Option<TimelineCueAuthority> {
        self.read_authority_snapshot()
            .map(|(authority, _)| authority)
    }

    fn begin_event_publication(&self) -> Result<TimelineCuePublicationToken, String> {
        loop {
            let state = self.event_publication_state.load(Ordering::Acquire);
            let (publishing, started) = match state {
                0 => (1, false),
                2 => (3, true),
                _ => {
                    return Err("Timeline cue audio event publication is already active".to_string())
                }
            };
            if self
                .event_publication_state
                .compare_exchange(state, publishing, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(TimelineCuePublicationToken {
                    started,
                    writing_state: publishing,
                });
            }
        }
    }

    fn commit_event_publication(&self, token: TimelineCuePublicationToken) -> bool {
        self.event_publication_state
            .compare_exchange(
                token.writing_state,
                if token.started { 2 } else { 0 },
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn reject_event_publication(&self) {
        // Any callback that invalidated a producer has started the output
        // clock, including the frame-zero race.
        self.event_publication_state.store(2, Ordering::Release);
    }

    fn cancel_event_publication(&self, token: TimelineCuePublicationToken) {
        if self
            .event_publication_state
            .compare_exchange(
                token.writing_state,
                if token.started { 2 } else { 0 },
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            self.reject_event_publication();
        }
    }

    /// One wait-free callback attempt: an active producer is invalidated with
    /// at most one additional CAS. The caller skips queue activation but keeps
    /// rendering existing voices and advances the device clock normally.
    fn callback_begin_frame(&self) -> bool {
        let state = self.event_publication_state.load(Ordering::Acquire);
        match state {
            0 => match self.event_publication_state.compare_exchange(
                0,
                2,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => true,
                Err(2) => true,
                Err(writing @ (1 | 3)) => {
                    let _ = self.event_publication_state.compare_exchange(
                        writing,
                        4,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                    false
                }
                Err(_) => false,
            },
            2 => true,
            writing @ (1 | 3) => match self.event_publication_state.compare_exchange(
                writing,
                4,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => false,
                Err(0) => self
                    .event_publication_state
                    .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok(),
                Err(2) => true,
                Err(_) => false,
            },
            _ => false,
        }
    }

    fn read_gain_snapshot(&self) -> Option<((f32, f32), u64)> {
        for _ in 0..4 {
            let before = self.gain_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            let gains = (
                f32::from_bits(self.click_gain_bits.load(Ordering::Relaxed)),
                f32::from_bits(self.guide_gain_bits.load(Ordering::Relaxed)),
            );
            if before == self.gain_sequence.load(Ordering::Acquire) {
                return Some((gains, before));
            }
        }
        None
    }

    fn read_authority_snapshot(&self) -> Option<(TimelineCueAuthority, u64)> {
        for _ in 0..4 {
            let before = self.authority_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            let authority = TimelineCueAuthority {
                fence: TimelineCueFence {
                    output_clock_epoch: self.output_clock_epoch.load(Ordering::Relaxed),
                    schedule_generation: self.schedule_generation.load(Ordering::Relaxed),
                    source_fence: self.source_fence.load(Ordering::Relaxed),
                },
                clock: TimelineCueClockMap {
                    canonical_anchor_frame: self.canonical_anchor_frame.load(Ordering::Relaxed),
                    output_anchor_frame: self.output_anchor_frame.load(Ordering::Relaxed),
                },
            };
            let after = self.authority_sequence.load(Ordering::Acquire);
            if before == after {
                return Some((authority, before));
            }
        }
        None
    }

    fn record_fault(&self, code: TimelineCueFaultCode, sequence: u64) {
        // This is a bounded seqlock.  The callback never waits on a mutex; a
        // producer/callback collision simply retries a small fixed number of
        // times and then records the fault with the same checked counters.
        for _ in 0..8 {
            let before = self.fault_snapshot_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            let Some(writing) = before.checked_add(1) else {
                return;
            };
            let Some(committed) = writing.checked_add(1) else {
                // Do not leave the snapshot sequence permanently odd or
                // publish a torn terminal snapshot when the seqlock is
                // exhausted.  The last coherent status remains readable.
                return;
            };
            if self
                .fault_snapshot_sequence
                .compare_exchange(before, writing, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            self.fault_sequence.store(sequence, Ordering::Relaxed);
            self.fault_code.store(code as u8, Ordering::Relaxed);
            let count = self.fault_count.load(Ordering::Relaxed);
            if let Some(next) = count.checked_add(1) {
                self.fault_count.store(next, Ordering::Relaxed);
            }
            self.fault_snapshot_sequence
                .store(committed, Ordering::Release);
            return;
        }
        // A contended writer must not bypass the seqlock and expose a mixed
        // code/count/sequence tuple.  Fault publication is best-effort under
        // this bounded contention; the previous coherent snapshot is kept.
    }

    fn fault_status(&self) -> TimelineCueFaultStatus {
        for _ in 0..8 {
            let before = self.fault_snapshot_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            let status = TimelineCueFaultStatus {
                code: TimelineCueFaultCode::from_u8(self.fault_code.load(Ordering::Relaxed)),
                count: self.fault_count.load(Ordering::Relaxed),
                sequence: self.fault_sequence.load(Ordering::Relaxed),
            };
            if before == self.fault_snapshot_sequence.load(Ordering::Acquire) {
                return status;
            }
        }
        // Never synthesize a tuple from fields that may belong to different
        // writes.  This bounded sentinel is coherent and makes contention
        // explicit to the control plane.
        TimelineCueFaultStatus {
            code: TimelineCueFaultCode::AuthorityUnstable,
            count: 0,
            sequence: 0,
        }
    }
}

#[derive(Debug, Default)]
struct TimelineCueProducerState {
    last_output_frame: Option<u64>,
    last_sequence: Option<u64>,
    source_epoch: Option<u64>,
}

#[derive(Clone)]
pub struct TimelineCueAudioControl {
    shared: Arc<TimelineCueShared>,
    queue: Arc<TimelineCueSpscQueue>,
    producer: Arc<Mutex<TimelineCueProducerState>>,
    assets: Arc<GuideAssetBank>,
    output_sample_rate: u32,
    channels: u16,
}

impl TimelineCueAudioControl {
    /// Retire the one callback Source owned by this control. The callback
    /// observes this atomic fence and returns `None`, allowing Rodio's mixer to
    /// remove it without retaining an infinite silent source after a device or
    /// project rotation.
    pub fn retire(&self) {
        self.shared.retired.store(1, Ordering::Release);
    }

    pub fn new_source(&self) -> Result<TimelineCueAudioSource, String> {
        let mut producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        let authority = self
            .shared
            .read_authority()
            .ok_or_else(|| "Timeline cue audio authority is unstable".to_string())?;
        if producer.source_epoch == Some(authority.fence.output_clock_epoch) {
            return Err(format!(
                "Timeline cue audio output clock epoch {} already owns a Source",
                authority.fence.output_clock_epoch
            ));
        }
        producer.source_epoch = Some(authority.fence.output_clock_epoch);
        self.shared.next_output_frame.store(0, Ordering::Release);
        Ok(TimelineCueAudioSource::new(
            Arc::clone(&self.shared),
            Arc::clone(&self.queue),
            Arc::clone(&self.assets),
            authority,
            self.output_sample_rate,
            self.channels,
        ))
    }

    pub fn publish_authority(&self, authority: TimelineCueAuthority) -> Result<(), String> {
        let mut producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        let previous = self.shared.read_authority().ok_or_else(|| {
            self.shared
                .record_fault(TimelineCueFaultCode::AuthorityUnstable, 0);
            "Timeline cue audio authority is unstable".to_string()
        })?;
        if previous == authority {
            return Ok(());
        }
        if previous.fence == authority.fence {
            self.shared
                .record_fault(TimelineCueFaultCode::InvalidFormat, 0);
            return Err(
                "Timeline cue audio clock map changed without a new authority fence".to_string(),
            );
        }
        if authority.fence < previous.fence {
            self.shared
                .record_fault(TimelineCueFaultCode::StaleFence, 0);
            return Err("Timeline cue audio authority fence regressed".to_string());
        }
        let current = self.shared.authority_sequence.load(Ordering::Acquire);
        if current & 1 != 0 {
            self.shared
                .record_fault(TimelineCueFaultCode::AuthorityUnstable, 0);
            return Err("Timeline cue audio authority is being updated".to_string());
        }
        let writing = current.checked_add(1).ok_or_else(|| {
            self.shared
                .record_fault(TimelineCueFaultCode::AuthorityExhausted, 0);
            "Timeline cue audio authority generation is exhausted".to_string()
        })?;
        let committed = writing.checked_add(1).ok_or_else(|| {
            self.shared
                .record_fault(TimelineCueFaultCode::AuthorityExhausted, 0);
            "Timeline cue audio authority generation is exhausted".to_string()
        })?;
        self.shared
            .authority_sequence
            .store(writing, Ordering::Release);
        // Keep the sequence odd while every old event is made unreachable.
        // The callback therefore clears local voices and emits silence during
        // the whole transition instead of observing a new fence with old PCM.
        self.queue.clear();
        self.shared
            .output_clock_epoch
            .store(authority.fence.output_clock_epoch, Ordering::Relaxed);
        self.shared
            .schedule_generation
            .store(authority.fence.schedule_generation, Ordering::Relaxed);
        self.shared
            .source_fence
            .store(authority.fence.source_fence, Ordering::Relaxed);
        self.shared
            .canonical_anchor_frame
            .store(authority.clock.canonical_anchor_frame, Ordering::Relaxed);
        self.shared
            .output_anchor_frame
            .store(authority.clock.output_anchor_frame, Ordering::Relaxed);
        if previous.fence.output_clock_epoch != authority.fence.output_clock_epoch {
            self.shared.next_output_frame.store(0, Ordering::Relaxed);
            self.shared
                .event_publication_state
                .store(0, Ordering::Relaxed);
            producer.source_epoch = None;
        }
        producer.last_output_frame = None;
        producer.last_sequence = None;
        self.shared
            .authority_sequence
            .store(committed, Ordering::Release);
        Ok(())
    }

    pub fn enqueue_batch(
        &self,
        fence: TimelineCueFence,
        events: &[TimelineCueEvent],
    ) -> Result<(), String> {
        let mut producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        let authority = self.shared.read_authority().ok_or_else(|| {
            self.shared
                .record_fault(TimelineCueFaultCode::AuthorityUnstable, 0);
            "Timeline cue audio authority is unstable".to_string()
        })?;
        if authority.fence != fence {
            self.shared
                .record_fault(TimelineCueFaultCode::StaleFence, 0);
            return Err("Timeline cue audio event fence is stale".to_string());
        }
        if events.len() > self.queue.capacity().saturating_sub(self.queue.len()) {
            self.shared.record_fault(
                TimelineCueFaultCode::QueueFull,
                events.first().map(|event| event.sequence).unwrap_or(0),
            );
            return Err("Timeline cue audio event queue is full".to_string());
        }
        let mut prepared = Vec::with_capacity(events.len());
        let mut last_output_frame = producer.last_output_frame;
        let mut last_sequence = producer.last_sequence;
        for event in events {
            let output_frame = map_canonical_frame_to_output(
                event.canonical_frame,
                authority.clock,
                self.output_sample_rate,
            )
            .inspect_err(|_error| {
                self.shared
                    .record_fault(TimelineCueFaultCode::FrameMappingOverflow, event.sequence);
            })?;
            if last_output_frame.is_some_and(|last| output_frame < last)
                || last_sequence.is_some_and(|last| event.sequence <= last)
            {
                self.shared
                    .record_fault(TimelineCueFaultCode::OutOfOrder, event.sequence);
                return Err(format!(
                    "Timeline cue audio event {} is out of order",
                    event.sequence
                ));
            }
            if !(TIMELINE_GUIDE_PLAYBACK_RATE_MIN_MILLI..=TIMELINE_GUIDE_PLAYBACK_RATE_MAX_MILLI)
                .contains(&event.playback_rate_milli)
            {
                self.shared
                    .record_fault(TimelineCueFaultCode::GuideRateInvalid, event.sequence);
                return Err(format!(
                    "Timeline cue event {} has playback rate {} outside {}..{} milli",
                    event.sequence,
                    event.playback_rate_milli,
                    TIMELINE_GUIDE_PLAYBACK_RATE_MIN_MILLI,
                    TIMELINE_GUIDE_PLAYBACK_RATE_MAX_MILLI
                ));
            }
            prepared.push(ScheduledTimelineCueEvent {
                fence,
                output_frame,
                sequence: event.sequence,
                kind: event.kind,
                playback_rate_milli: event.playback_rate_milli,
            });
            last_output_frame = Some(output_frame);
            last_sequence = Some(event.sequence);
        }
        // The callback never waits for this producer.  It invalidates a
        // WRITING state in one CAS and advances its clock normally.  The batch
        // remains invisible until the tail is staged and the producer wins the
        // final WRITING -> stable CAS; otherwise every staged slot is rolled
        // back and the whole batch is rejected.
        let token = self.shared.begin_event_publication()?;
        let next_output_frame = self.shared.next_output_frame.load(Ordering::Acquire);
        let minimum_output_frame = if token.started {
            match next_output_frame.checked_add(MIN_EVENT_LOOKAHEAD_FRAMES) {
                Some(frame) => frame,
                None => {
                    self.shared.cancel_event_publication(token);
                    self.shared
                        .record_fault(TimelineCueFaultCode::OutputFrameExhausted, 0);
                    return Err("Timeline cue audio output frame is exhausted".to_string());
                }
            }
        } else {
            0
        };
        if let Some(event) = prepared
            .iter()
            .find(|event| event.output_frame < minimum_output_frame)
        {
            self.shared.cancel_event_publication(token);
            self.shared
                .record_fault(TimelineCueFaultCode::PastDue, event.sequence);
            return Err(format!(
                "Timeline cue audio event {} does not satisfy the {} frame publication lookahead",
                event.sequence, MIN_EVENT_LOOKAHEAD_FRAMES
            ));
        }
        let staged = match self.queue.stage_batch(&prepared) {
            Ok(staged) => staged,
            Err(()) => {
                self.shared.cancel_event_publication(token);
                self.shared.record_fault(
                    TimelineCueFaultCode::QueueFull,
                    prepared.first().map(|event| event.sequence).unwrap_or(0),
                );
                return Err(
                    "Timeline cue audio event queue changed while staging a batch".to_string(),
                );
            }
        };
        self.queue.publish_staged(staged);
        if !self.shared.commit_event_publication(token) {
            self.queue.rollback_staged(staged);
            self.shared.reject_event_publication();
            self.shared.record_fault(
                TimelineCueFaultCode::PastDue,
                prepared.first().map(|event| event.sequence).unwrap_or(0),
            );
            return Err("Timeline cue audio event batch lost its publication race".to_string());
        }
        producer.last_output_frame = last_output_frame;
        producer.last_sequence = last_sequence;
        Ok(())
    }

    pub fn set_click_gain(&self, gain: f32) -> Result<(), String> {
        let _producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        let ((_, guide_gain), _) = self
            .shared
            .read_gain_snapshot()
            .ok_or_else(|| "Timeline cue audio gain authority is unstable".to_string())?;
        self.publish_gain_pair(gain, guide_gain)
    }

    pub fn set_guide_gain(&self, gain: f32) -> Result<(), String> {
        let _producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        let ((click_gain, _), _) = self
            .shared
            .read_gain_snapshot()
            .ok_or_else(|| "Timeline cue audio gain authority is unstable".to_string())?;
        self.publish_gain_pair(click_gain, gain)
    }

    pub fn set_gains(&self, click_gain: f32, guide_gain: f32) -> Result<(), String> {
        let _producer = self
            .producer
            .lock()
            .map_err(|_| "Timeline cue audio producer lock was poisoned".to_string())?;
        self.publish_gain_pair(click_gain, guide_gain)
    }

    fn publish_gain_pair(&self, click_gain: f32, guide_gain: f32) -> Result<(), String> {
        validate_mixer_headroom(click_gain, guide_gain, self.assets.peak_abs())?;
        let sequence = self.shared.gain_sequence.load(Ordering::Acquire);
        if sequence & 1 != 0 {
            return Err("Timeline cue audio gain authority is being updated".to_string());
        }
        let writing = sequence
            .checked_add(1)
            .ok_or_else(|| "Timeline cue audio gain authority is exhausted".to_string())?;
        let committed = writing
            .checked_add(1)
            .ok_or_else(|| "Timeline cue audio gain authority is exhausted".to_string())?;
        self.shared.gain_sequence.store(writing, Ordering::Release);
        self.shared
            .click_gain_bits
            .store(click_gain.to_bits(), Ordering::Relaxed);
        self.shared
            .guide_gain_bits
            .store(guide_gain.to_bits(), Ordering::Relaxed);
        self.shared
            .gain_sequence
            .store(committed, Ordering::Release);
        Ok(())
    }

    pub fn fault_status(&self) -> TimelineCueFaultStatus {
        self.shared.fault_status()
    }

    pub fn next_output_frame(&self) -> u64 {
        self.shared.next_output_frame.load(Ordering::Acquire)
    }
}

pub fn create_timeline_cue_audio_source(
    output_sample_rate: u32,
    channels: u16,
    queue_capacity: usize,
    assets: GuideAssetBank,
    authority: TimelineCueAuthority,
    click_gain: f32,
    guide_gain: f32,
) -> Result<(TimelineCueAudioControl, TimelineCueAudioSource), String> {
    validate_output_format(output_sample_rate, channels)?;
    if assets.output_sample_rate() != output_sample_rate {
        return Err("Timeline Guide assets do not match the output sample rate".to_string());
    }
    if !(1..=MAX_TIMELINE_CUE_QUEUE_CAPACITY).contains(&queue_capacity) {
        return Err(format!(
            "Timeline cue audio queue capacity must be between 1 and {}",
            MAX_TIMELINE_CUE_QUEUE_CAPACITY
        ));
    }
    validate_mixer_headroom(click_gain, guide_gain, assets.peak_abs())?;
    let shared = Arc::new(TimelineCueShared::new(authority, click_gain, guide_gain));
    let queue = Arc::new(TimelineCueSpscQueue::new(queue_capacity));
    let assets = Arc::new(assets);
    let producer = Arc::new(Mutex::new(TimelineCueProducerState {
        source_epoch: Some(authority.fence.output_clock_epoch),
        ..TimelineCueProducerState::default()
    }));
    let control = TimelineCueAudioControl {
        shared: Arc::clone(&shared),
        queue: Arc::clone(&queue),
        producer,
        assets: Arc::clone(&assets),
        output_sample_rate,
        channels,
    };
    let source = TimelineCueAudioSource::new(
        shared,
        queue,
        assets,
        authority,
        output_sample_rate,
        channels,
    );
    Ok((control, source))
}

fn validate_output_format(output_sample_rate: u32, channels: u16) -> Result<(), String> {
    if !(MIN_TIMELINE_CUE_OUTPUT_SAMPLE_RATE..=MAX_TIMELINE_CUE_OUTPUT_SAMPLE_RATE)
        .contains(&output_sample_rate)
    {
        return Err(format!(
            "Timeline cue audio output sample rate must be between {} and {} Hz",
            MIN_TIMELINE_CUE_OUTPUT_SAMPLE_RATE, MAX_TIMELINE_CUE_OUTPUT_SAMPLE_RATE
        ));
    }
    if !(1..=MAX_TIMELINE_CUE_OUTPUT_CHANNELS).contains(&channels) {
        return Err(format!(
            "Timeline cue audio output channels must be between 1 and {}",
            MAX_TIMELINE_CUE_OUTPUT_CHANNELS
        ));
    }
    Ok(())
}

pub fn map_canonical_frame_to_output(
    canonical_frame: u64,
    clock: TimelineCueClockMap,
    output_sample_rate: u32,
) -> Result<u64, String> {
    validate_output_format(output_sample_rate, 1)?;
    let delta = canonical_frame
        .checked_sub(clock.canonical_anchor_frame)
        .ok_or_else(|| "Timeline cue audio event precedes its clock anchor".to_string())?;
    let scaled = (delta as u128)
        .checked_mul(output_sample_rate as u128)
        .map(|value| value / TIMELINE_CUE_CANONICAL_SAMPLE_RATE as u128)
        .ok_or_else(|| "Timeline cue audio frame mapping overflowed".to_string())?;
    let scaled = u64::try_from(scaled)
        .map_err(|_| "Timeline cue audio frame mapping overflowed".to_string())?;
    clock
        .output_anchor_frame
        .checked_add(scaled)
        .ok_or_else(|| "Timeline cue audio frame mapping overflowed".to_string())
}

#[derive(Debug, Clone, Copy)]
struct ClickVoice {
    active: bool,
    phase: f64,
    phase_increment: f64,
    frame: u64,
    duration_frames: u64,
    decay_frames: u64,
    envelope: f32,
    decay_multiplier: f32,
}

impl Default for ClickVoice {
    fn default() -> Self {
        Self {
            active: false,
            phase: 0.0,
            phase_increment: 0.0,
            frame: 0,
            duration_frames: 0,
            decay_frames: 0,
            envelope: 0.0,
            decay_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct GuideVoice {
    active: bool,
    asset: Option<TimelineGuideAssetKey>,
    position: f64,
    playback_rate_milli: u16,
}

#[derive(Debug, Clone, Copy)]
struct GainSlew {
    current: f32,
    target: f32,
    step: f32,
    remaining_frames: u64,
    slew_frames: u64,
}

impl GainSlew {
    fn new(gain: f32, sample_rate: u32) -> Self {
        Self {
            current: gain,
            target: gain,
            step: 0.0,
            remaining_frames: 0,
            slew_frames: frames_for_ms(sample_rate, GAIN_SLEW_MS).max(1),
        }
    }

    fn advance(&mut self, target: f32) -> f32 {
        if target.to_bits() != self.target.to_bits() {
            self.target = target;
            self.remaining_frames = self.slew_frames;
            self.step = (self.target - self.current) / self.slew_frames as f32;
        }
        if self.remaining_frames > 0 {
            self.current += self.step;
            self.remaining_frames -= 1;
            if self.remaining_frames == 0 {
                self.current = self.target;
            }
        }
        self.current
    }
}

pub struct TimelineCueAudioSource {
    shared: Arc<TimelineCueShared>,
    queue: Arc<TimelineCueSpscQueue>,
    assets: Arc<GuideAssetBank>,
    bound_output_clock_epoch: u64,
    local_fence: TimelineCueFence,
    output_sample_rate: u32,
    channels: u16,
    channel_index: u16,
    output_frame: u64,
    frame_sample: f32,
    pending: Option<ScheduledTimelineCueEvent>,
    click_voices: [ClickVoice; MAX_CLICK_VOICES],
    guide_voices: [GuideVoice; MAX_GUIDE_VOICES],
    guide_pending: [Option<ScheduledTimelineCueEvent>; MAX_PENDING_GUIDE_EVENTS],
    guide_pending_len: usize,
    click_gain: GainSlew,
    guide_gain: GainSlew,
    click_decay_multiplier: f32,
    exhausted: bool,
    clock_detached: bool,
}

impl TimelineCueAudioSource {
    fn new(
        shared: Arc<TimelineCueShared>,
        queue: Arc<TimelineCueSpscQueue>,
        assets: Arc<GuideAssetBank>,
        authority: TimelineCueAuthority,
        output_sample_rate: u32,
        channels: u16,
    ) -> Self {
        let ((click_gain, guide_gain), _) = shared
            .read_gain_snapshot()
            .expect("Timeline cue audio gains are stable during Source construction");
        Self {
            shared,
            queue,
            assets,
            bound_output_clock_epoch: authority.fence.output_clock_epoch,
            local_fence: authority.fence,
            output_sample_rate,
            channels,
            channel_index: 0,
            output_frame: 0,
            frame_sample: 0.0,
            pending: None,
            click_voices: [ClickVoice::default(); MAX_CLICK_VOICES],
            guide_voices: [GuideVoice::default(); MAX_GUIDE_VOICES],
            guide_pending: [None; MAX_PENDING_GUIDE_EVENTS],
            guide_pending_len: 0,
            click_gain: GainSlew::new(click_gain, output_sample_rate),
            guide_gain: GainSlew::new(guide_gain, output_sample_rate),
            click_decay_multiplier: precompute_click_decay_multiplier(output_sample_rate),
            exhausted: false,
            clock_detached: false,
        }
    }

    fn clear_voices(&mut self) {
        self.pending = None;
        self.click_voices = [ClickVoice::default(); MAX_CLICK_VOICES];
        self.guide_voices = [GuideVoice::default(); MAX_GUIDE_VOICES];
        self.guide_pending = [None; MAX_PENDING_GUIDE_EVENTS];
        self.guide_pending_len = 0;
    }

    fn render_frame(&mut self) -> f32 {
        if self.exhausted || self.output_frame == u64::MAX {
            if !self.exhausted {
                self.shared
                    .record_fault(TimelineCueFaultCode::OutputFrameExhausted, 0);
                self.exhausted = true;
                self.clear_voices();
            }
            return 0.0;
        }
        let Some((authority, authority_sequence)) = self.shared.read_authority_snapshot() else {
            // An odd authority sequence is the expected, bounded reset/drain
            // window.  It is not a runtime fault: silence this complete frame
            // and discard every local voice until the even commit is visible.
            self.clear_voices();
            return 0.0;
        };
        if authority.fence.output_clock_epoch != self.bound_output_clock_epoch {
            self.clear_voices();
            self.clock_detached = true;
            return 0.0;
        }
        let may_activate_events = self.shared.callback_begin_frame();
        if authority.fence != self.local_fence {
            self.clear_voices();
            self.local_fence = authority.fence;
        }
        if may_activate_events && !self.activate_due_events(authority.fence) {
            self.clear_voices();
            return 0.0;
        }
        let click = self.render_click_voices();
        let guide = self.render_guide_voices();
        let (click_target, guide_target) = self
            .shared
            .read_gain_snapshot()
            .map(|(gains, _)| gains)
            .unwrap_or((self.click_gain.target, self.guide_gain.target));
        if !click_target.is_finite()
            || !guide_target.is_finite()
            || click_target < 0.0
            || guide_target < 0.0
            || click_target > 2.0
            || guide_target > 2.0
        {
            self.shared
                .record_fault(TimelineCueFaultCode::NonFiniteOutput, 0);
            self.clear_voices();
            return 0.0;
        }
        let click_gain = self.click_gain.advance(click_target);
        let guide_gain = self.guide_gain.advance(guide_target);
        let mixed =
            f64::from(click) * f64::from(click_gain) + f64::from(guide) * f64::from(guide_gain);
        if self.shared.authority_sequence.load(Ordering::Acquire) != authority_sequence {
            self.clear_voices();
            return 0.0;
        }
        if !mixed.is_finite() {
            self.shared
                .record_fault(TimelineCueFaultCode::NonFiniteOutput, 0);
            self.clear_voices();
            return 0.0;
        }
        if mixed.abs() > 1.0 {
            self.shared
                .record_fault(TimelineCueFaultCode::OutputOverUnity, 0);
            self.clear_voices();
            return 0.0;
        }
        let mixed = mixed as f32;
        if !mixed.is_finite() {
            self.shared
                .record_fault(TimelineCueFaultCode::NonFiniteOutput, 0);
            self.clear_voices();
            return 0.0;
        }
        mixed
    }

    fn activate_due_events(&mut self, fence: TimelineCueFence) -> bool {
        let mut due = [None; MAX_DUE_EVENTS_PER_FRAME];
        let mut due_len = 0usize;
        for _ in 0..MAX_DUE_EVENTS_PER_FRAME {
            let event = match self
                .pending
                .take()
                .or_else(|| self.queue.pop_once(&self.shared.event_publication_state))
            {
                Some(event) => event,
                None => break,
            };
            if event.fence != fence {
                self.shared
                    .record_fault(TimelineCueFaultCode::StaleFence, event.sequence);
                continue;
            }
            if event.output_frame < self.output_frame {
                self.shared
                    .record_fault(TimelineCueFaultCode::PastDue, event.sequence);
                continue;
            }
            if event.output_frame > self.output_frame {
                self.pending = Some(event);
                break;
            }
            due[due_len] = Some(event);
            due_len += 1;
        }

        let required_clicks = due[..due_len]
            .iter()
            .flatten()
            .filter(|event| matches!(event.kind, TimelineCueEventKind::Click { .. }))
            .count();
        let required_guides = due[..due_len]
            .iter()
            .flatten()
            .filter(|event| matches!(event.kind, TimelineCueEventKind::Guide { .. }))
            .count();
        let free_clicks = self
            .click_voices
            .iter()
            .filter(|voice| !voice.active)
            .count();
        let free_guides = self
            .guide_voices
            .iter()
            .filter(|voice| !voice.active)
            .count();
        let pending_capacity = MAX_PENDING_GUIDE_EVENTS.saturating_sub(self.guide_pending_len);
        if required_clicks > free_clicks {
            self.shared.record_fault(
                TimelineCueFaultCode::ActiveVoiceOverflow,
                due[..due_len]
                    .iter()
                    .flatten()
                    .next()
                    .map(|event| event.sequence)
                    .unwrap_or(0),
            );
            return false;
        }
        if required_guides > free_guides.saturating_add(pending_capacity) {
            self.shared.record_fault(
                TimelineCueFaultCode::QueueFull,
                due[..due_len]
                    .iter()
                    .flatten()
                    .next()
                    .map(|event| event.sequence)
                    .unwrap_or(0),
            );
            return false;
        }

        for event in due[..due_len].iter().flatten().copied() {
            match event.kind {
                TimelineCueEventKind::Click { accented } => {
                    let _ = self.start_click(accented);
                }
                TimelineCueEventKind::Guide { asset } => {
                    let scheduled = event;
                    if self.guide_voices.iter().any(|voice| voice.active) {
                        let _ = self.enqueue_pending_guide(scheduled);
                    } else {
                        let _ = self.start_guide(asset, scheduled.playback_rate_milli);
                    }
                }
            }
        }
        true
    }

    fn start_click(&mut self, accented: bool) -> bool {
        let Some(slot) = self.click_voices.iter_mut().find(|voice| !voice.active) else {
            return false;
        };
        let duration_frames = frames_for_ms(self.output_sample_rate, CLICK_DURATION_MS).max(1);
        let decay_frames = frames_for_ms(self.output_sample_rate, CLICK_DECAY_MS).max(2);
        *slot = ClickVoice {
            active: true,
            phase: 0.0,
            phase_increment: if accented {
                CLICK_ACCENT_FREQUENCY_HZ
            } else {
                CLICK_REGULAR_FREQUENCY_HZ
            } / self.output_sample_rate as f64,
            frame: 0,
            duration_frames,
            decay_frames,
            envelope: CLICK_ENVELOPE_START,
            decay_multiplier: self.click_decay_multiplier,
        };
        true
    }

    fn enqueue_pending_guide(&mut self, event: ScheduledTimelineCueEvent) -> bool {
        if self.guide_pending_len >= MAX_PENDING_GUIDE_EVENTS {
            self.shared
                .record_fault(TimelineCueFaultCode::QueueFull, event.sequence);
            return false;
        }
        self.guide_pending[self.guide_pending_len] = Some(event);
        self.guide_pending_len += 1;
        true
    }

    fn take_pending_guide(&mut self) -> Option<ScheduledTimelineCueEvent> {
        if self.guide_pending_len == 0 {
            return None;
        }
        let event = self.guide_pending[0].take();
        for index in 1..self.guide_pending_len {
            self.guide_pending[index - 1] = self.guide_pending[index];
        }
        self.guide_pending[self.guide_pending_len - 1] = None;
        self.guide_pending_len -= 1;
        event
    }

    fn start_guide(&mut self, asset: TimelineGuideAssetKey, playback_rate_milli: u16) -> bool {
        if !(TIMELINE_GUIDE_PLAYBACK_RATE_MIN_MILLI..=TIMELINE_GUIDE_PLAYBACK_RATE_MAX_MILLI)
            .contains(&playback_rate_milli)
        {
            self.shared
                .record_fault(TimelineCueFaultCode::GuideRateInvalid, 0);
            return false;
        }
        let Some(slot) = self.guide_voices.iter_mut().find(|voice| !voice.active) else {
            return false;
        };
        *slot = GuideVoice {
            active: true,
            asset: Some(asset),
            position: 0.0,
            playback_rate_milli,
        };
        true
    }

    fn render_click_voices(&mut self) -> f32 {
        let mut mixed = 0.0;
        let mut active_voice_count = 0_u32;
        for voice in &mut self.click_voices {
            if !voice.active {
                continue;
            }
            active_voice_count += 1;
            let polarity = if voice.phase < 0.5 { 1.0 } else { -1.0 };
            mixed += polarity * voice.envelope;
            voice.phase += voice.phase_increment;
            voice.phase -= voice.phase.floor();
            voice.frame += 1;
            if voice.frame < voice.decay_frames {
                voice.envelope *= voice.decay_multiplier;
            } else {
                voice.envelope = CLICK_ENVELOPE_END;
            }
            if voice.frame >= voice.duration_frames {
                voice.active = false;
            }
        }
        // Multiple descriptors may coincide during a bounded producer fault,
        // but the authored click bus peak contract remains 0.1.  Averaging
        // active click voices preserves that bound without a hidden limiter or
        // a callback-time clamp.
        if active_voice_count == 0 {
            0.0
        } else {
            mixed / active_voice_count as f32
        }
    }

    fn render_guide_voices(&mut self) -> f32 {
        let mut mixed = 0.0;
        for voice in &mut self.guide_voices {
            if !voice.active {
                continue;
            }
            let Some(asset) = voice.asset else {
                voice.active = false;
                continue;
            };
            let samples = self.assets.samples(asset);
            if voice.position.is_finite() && voice.position >= 0.0 {
                let frame_position = voice.position;
                if frame_position < samples.len() as f64 {
                    let left = frame_position.floor() as usize;
                    let fraction = frame_position - left as f64;
                    let left_sample = f64::from(samples[left.min(samples.len() - 1)]);
                    let right_sample = f64::from(samples[(left + 1).min(samples.len() - 1)]);
                    let sample = left_sample + (right_sample - left_sample) * fraction;
                    if !sample.is_finite() {
                        self.shared
                            .record_fault(TimelineCueFaultCode::NonFiniteOutput, 0);
                        voice.active = false;
                        voice.asset = None;
                        continue;
                    }
                    mixed += sample as f32;
                    voice.position += f64::from(voice.playback_rate_milli) / 1000.0;
                    if voice.position >= samples.len() as f64 {
                        voice.active = false;
                        voice.asset = None;
                    }
                } else {
                    voice.active = false;
                    voice.asset = None;
                }
            } else {
                self.shared
                    .record_fault(TimelineCueFaultCode::NonFiniteOutput, 0);
                voice.active = false;
                voice.asset = None;
            }
        }
        if !self.guide_voices.iter().any(|voice| voice.active) {
            if let Some(event) = self.take_pending_guide() {
                if let TimelineCueEventKind::Guide { asset } = event.kind {
                    let _ = self.start_guide(asset, event.playback_rate_milli);
                }
            }
        }
        mixed
    }
}

impl Iterator for TimelineCueAudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.shared.retired.load(Ordering::Acquire) != 0 || self.clock_detached {
            self.clear_voices();
            return None;
        }
        if self.channel_index == 0 {
            self.frame_sample = self.render_frame();
            if self.shared.authority_sequence.load(Ordering::Acquire) & 1 != 0 {
                self.clear_voices();
                self.frame_sample = 0.0;
            }
        }
        let sample = self.frame_sample;
        self.channel_index += 1;
        if self.channel_index == self.channels {
            self.channel_index = 0;
            if !self.exhausted
                && !self.clock_detached
                && self.shared.output_clock_epoch.load(Ordering::Acquire)
                    == self.bound_output_clock_epoch
            {
                self.output_frame += 1;
                self.shared
                    .next_output_frame
                    .store(self.output_frame, Ordering::Release);
            }
        }
        Some(sample)
    }
}

impl Source for TimelineCueAudioSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.output_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn frames_for_ms(sample_rate: u32, milliseconds: u64) -> u64 {
    (sample_rate as u64)
        .saturating_mul(milliseconds)
        .saturating_add(999)
        / 1_000
}

fn precompute_click_decay_multiplier(sample_rate: u32) -> f32 {
    let decay_frames = frames_for_ms(sample_rate, CLICK_DECAY_MS).max(2);
    (CLICK_ENVELOPE_END / CLICK_ENVELOPE_START).powf(1.0 / (decay_frames - 1) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
        sync::Barrier,
        thread,
    };

    struct CallbackTrackingAllocator;

    thread_local! {
        static TRACK_CALLBACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
        static CALLBACK_ALLOCATION_ACTIVITY: Cell<u64> = const { Cell::new(0) };
    }

    fn record_callback_allocation_activity() {
        if TRACK_CALLBACK_ALLOCATIONS
            .try_with(Cell::get)
            .unwrap_or(false)
        {
            let _ = CALLBACK_ALLOCATION_ACTIVITY.try_with(|count| {
                count.set(count.get().saturating_add(1));
            });
        }
    }

    unsafe impl GlobalAlloc for CallbackTrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record_callback_allocation_activity();
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            record_callback_allocation_activity();
            unsafe { System.dealloc(pointer, layout) };
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record_callback_allocation_activity();
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            record_callback_allocation_activity();
            unsafe { System.realloc(pointer, layout, size) }
        }
    }

    #[global_allocator]
    static CALLBACK_TRACKING_ALLOCATOR: CallbackTrackingAllocator = CallbackTrackingAllocator;

    fn authority(epoch: u64, generation: u64) -> TimelineCueAuthority {
        TimelineCueAuthority {
            fence: TimelineCueFence {
                output_clock_epoch: epoch,
                schedule_generation: generation,
                source_fence: 17,
            },
            clock: TimelineCueClockMap {
                canonical_anchor_frame: 0,
                output_anchor_frame: 0,
            },
        }
    }

    fn asset_bank(sample_rate: u32) -> GuideAssetBank {
        let decoded = TimelineGuideAssetKey::ALL
            .into_iter()
            .map(|key| {
                DecodedGuideAsset::from_mono_pcm(key, sample_rate, vec![0.25, -0.25, 0.125])
                    .unwrap()
            })
            .collect();
        GuideAssetBank::prepare_complete(sample_rate, decoded).unwrap()
    }

    fn asset_bank_with(
        sample_rate: u32,
        sample_for: impl Fn(TimelineGuideAssetKey) -> Vec<f32>,
    ) -> GuideAssetBank {
        let decoded = TimelineGuideAssetKey::ALL
            .into_iter()
            .map(|key| DecodedGuideAsset::from_mono_pcm(key, sample_rate, sample_for(key)).unwrap())
            .collect();
        GuideAssetBank::prepare_complete(sample_rate, decoded).unwrap()
    }

    fn source_at(
        sample_rate: u32,
        channels: u16,
    ) -> (TimelineCueAudioControl, TimelineCueAudioSource) {
        create_timeline_cue_audio_source(
            sample_rate,
            channels,
            16,
            asset_bank(sample_rate),
            authority(1, 1),
            1.0,
            1.0,
        )
        .unwrap()
    }

    fn collect_frames(source: &mut TimelineCueAudioSource, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|_| {
                let first = source.next().unwrap();
                for _ in 1..source.channels() {
                    assert_eq!(source.next(), Some(first));
                }
                first
            })
            .collect()
    }

    #[test]
    fn exact_onset_is_silent_before_due_frame_and_channels_share_one_clock() {
        let (control, mut source) = source_at(48_000, 2);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 48,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        let frames = collect_frames(&mut source, 50);
        assert!(frames[..48].iter().all(|sample| *sample == 0.0));
        assert!((frames[48] - CLICK_ENVELOPE_START).abs() < 1e-6);
    }

    #[test]
    fn retired_control_ends_the_infinite_source_without_rendering_an_old_frame() {
        let (control, mut source) = source_at(48_000, 2);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        control.retire();
        assert_eq!(source.next(), None);
        assert_eq!(source.next(), None);
        assert_eq!(control.next_output_frame(), 0);
    }

    #[test]
    fn click_square_tone_and_exponential_decay_match_contract() {
        let (control, mut source) = source_at(48_000, 1);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: false },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        let frames = collect_frames(&mut source, frames_for_ms(48_000, 50) as usize + 1);
        assert!((frames[0].abs() - CLICK_ENVELOPE_START).abs() < 1e-6);
        let decay_end = frames_for_ms(48_000, 45) as usize - 1;
        assert!((frames[decay_end].abs() - CLICK_ENVELOPE_END).abs() < 2e-6);
        assert_eq!(frames[frames_for_ms(48_000, 50) as usize], 0.0);
        let polarity_changes = frames[..2_000]
            .windows(2)
            .filter(|pair| pair[0].signum() != pair[1].signum())
            .count();
        assert!((70..=80).contains(&polarity_changes));
    }

    #[test]
    fn logical_bus_gains_are_isolated_and_slew_for_ten_milliseconds() {
        let (control, mut source) = create_timeline_cue_audio_source(
            48_000,
            1,
            16,
            asset_bank(48_000),
            authority(1, 1),
            0.0,
            1.0,
        )
        .unwrap();
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 1,
                        kind: TimelineCueEventKind::Click { accented: true },
                        playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 2,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Complete,
                        },
                        playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                ],
            )
            .unwrap();
        assert_eq!(source.next(), Some(0.25));

        let (control, mut source) = source_at(48_000, 1);
        control.set_click_gain(0.0).unwrap();
        let slew_frames = frames_for_ms(48_000, 10) as usize;
        let silence = collect_frames(&mut source, slew_frames);
        assert!(silence.iter().all(|sample| *sample == 0.0));
        assert_eq!(source.click_gain.current, 0.0);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: slew_frames as u64 + MIN_EVENT_LOOKAHEAD_FRAMES,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        assert_eq!(source.next(), Some(0.0));
        assert!(control.set_guide_gain(f32::INFINITY).is_err());
    }

    #[test]
    fn concurrent_gain_updates_publish_only_a_coherent_safe_pair() {
        let bank = asset_bank_with(48_000, |_| vec![MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS]);
        let (control, mut source) =
            create_timeline_cue_audio_source(48_000, 1, 16, bank, authority(1, 1), 0.0, 0.0)
                .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let click_control = control.clone();
        let click_barrier = Arc::clone(&barrier);
        let click = thread::spawn(move || {
            click_barrier.wait();
            click_control.set_click_gain(2.0)
        });
        let guide_control = control.clone();
        let guide_barrier = Arc::clone(&barrier);
        let guide = thread::spawn(move || {
            guide_barrier.wait();
            guide_control.set_guide_gain(1.4)
        });
        barrier.wait();
        let results = [click.join().unwrap(), guide.join().unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);

        let ((click_gain, guide_gain), sequence) = control.shared.read_gain_snapshot().unwrap();
        assert_eq!(sequence & 1, 0);
        assert!(validate_mixer_headroom(click_gain, guide_gain, control.assets.peak_abs()).is_ok());
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
        source.click_gain = GainSlew::new(click_gain, 48_000);
        source.guide_gain = GainSlew::new(guide_gain, 48_000);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 1,
                        kind: TimelineCueEventKind::Click { accented: true },
                        playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 2,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Complete,
                        },
                        playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                ],
            )
            .unwrap();
        assert!(source.next().unwrap().abs() <= 1.0);
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn unsafe_machine_gain_pair_rejects_without_runtime_delta() {
        let (control, _) = create_timeline_cue_audio_source(
            48_000,
            1,
            16,
            asset_bank_with(48_000, |_| vec![MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS]),
            authority(1, 1),
            0.0,
            0.0,
        )
        .unwrap();
        let before = control.shared.read_gain_snapshot().unwrap();
        assert!(control.set_gains(2.0, 2.0).is_err());
        assert_eq!(control.shared.read_gain_snapshot().unwrap(), before);
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn canonical_48k_frames_map_to_44k1_with_checked_floor_rounding() {
        let clock = TimelineCueClockMap {
            canonical_anchor_frame: 48_000,
            output_anchor_frame: 7,
        };
        assert_eq!(
            map_canonical_frame_to_output(96_000, clock, 44_100).unwrap(),
            44_107
        );
        assert_eq!(
            map_canonical_frame_to_output(48_001, clock, 44_100).unwrap(),
            7
        );
        assert!(map_canonical_frame_to_output(47_999, clock, 44_100).is_err());

        let (control, mut source) = source_at(44_100, 2);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 480,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Complete,
                    },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        let frames = collect_frames(&mut source, 442);
        assert!(frames[..441].iter().all(|sample| *sample == 0.0));
        assert_ne!(frames[441], 0.0);
    }

    #[test]
    fn stale_fence_and_authority_rotation_are_silent_without_catch_up() {
        let (control, mut source) = source_at(48_000, 1);
        let event = TimelineCueEvent {
            canonical_frame: 5,
            sequence: 1,
            kind: TimelineCueEventKind::Click { accented: true },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        control
            .enqueue_batch(authority(1, 1).fence, &[event])
            .unwrap();
        control.publish_authority(authority(1, 2)).unwrap();
        assert!(collect_frames(&mut source, 10)
            .iter()
            .all(|sample| *sample == 0.0));
        assert!(control
            .enqueue_batch(authority(1, 1).fence, &[event])
            .is_err());

        let (control, mut source) = source_at(48_000, 1);
        assert!(collect_frames(&mut source, 8)
            .iter()
            .all(|sample| *sample == 0.0));
        assert!(control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 3,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .is_err());
        assert!(collect_frames(&mut source, 4)
            .iter()
            .all(|sample| *sample == 0.0));
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::PastDue);
    }

    #[test]
    fn stop_and_output_epoch_restart_clear_old_audio() {
        let (control, mut old_source) = source_at(48_000, 1);
        assert!(control.new_source().is_err());
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Intro,
                    },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        assert_ne!(old_source.next(), Some(0.0));
        control.publish_authority(authority(2, 1)).unwrap();
        assert_eq!(old_source.next(), Some(0.0));
        let mut replacement = control.new_source().unwrap();
        control
            .enqueue_batch(
                authority(2, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Intro,
                    },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        assert_ne!(replacement.next(), Some(0.0));
    }

    #[test]
    fn active_voice_overflow_drops_the_whole_due_frame() {
        let (control, mut source) = source_at(48_000, 1);
        let events = (0..=MAX_CLICK_VOICES)
            .map(|index| TimelineCueEvent {
                canonical_frame: 0,
                sequence: index as u64 + 1,
                kind: TimelineCueEventKind::Click {
                    accented: index == 0,
                },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            })
            .collect::<Vec<_>>();
        control
            .enqueue_batch(authority(1, 1).fence, &events)
            .unwrap();
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::ActiveVoiceOverflow
        );
        assert!(source.click_voices.iter().all(|voice| !voice.active));
    }

    #[test]
    fn coincident_clicks_preserve_the_authored_bus_peak_contract() {
        let (control, mut source) = source_at(48_000, 1);
        let events = (0..MAX_CLICK_VOICES)
            .map(|index| TimelineCueEvent {
                canonical_frame: 0,
                sequence: index as u64 + 1,
                kind: TimelineCueEventKind::Click { accented: true },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            })
            .collect::<Vec<_>>();
        control
            .enqueue_batch(authority(1, 1).fence, &events)
            .unwrap();
        assert!((source.next().unwrap() - THEORETICAL_CLICK_PEAK as f32).abs() < 1e-6);
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn guide_voice_is_serial_ordered_and_captures_each_fixed_rate() {
        let bank = asset_bank_with(48_000, |key| match key {
            TimelineGuideAssetKey::Intro => vec![0.1, 0.1],
            TimelineGuideAssetKey::Complete => vec![0.2, 0.2],
            _ => vec![0.01],
        });
        let (control, mut source) =
            create_timeline_cue_audio_source(48_000, 1, 16, bank, authority(1, 1), 0.0, 1.0)
                .unwrap();
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 1,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Intro,
                        },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_MIN_MILLI,
                    },
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 2,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Complete,
                        },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_MAX_MILLI,
                    },
                ],
            )
            .unwrap();

        assert!((source.next().unwrap() - 0.1).abs() < 1e-6);
        assert_eq!(source.guide_voices[0].playback_rate_milli, 920);
        assert_eq!(source.guide_pending_len, 1);
        assert_eq!(
            source.guide_pending[0].unwrap().playback_rate_milli,
            GUIDE_PLAYBACK_RATE_MAX_MILLI
        );
        let remaining = collect_frames(&mut source, 4);
        assert!(remaining[..2]
            .iter()
            .all(|sample| (*sample - 0.1).abs() < 1e-6));
        assert!(remaining[2..]
            .iter()
            .all(|sample| (*sample - 0.2).abs() < 1e-6));
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn guide_pending_overflow_is_visible_and_silences_the_fault_frame() {
        let (control, mut source) = source_at(48_000, 1);
        let pending = ScheduledTimelineCueEvent {
            fence: authority(1, 1).fence,
            output_frame: 0,
            sequence: 100,
            kind: TimelineCueEventKind::Guide {
                asset: TimelineGuideAssetKey::Intro,
            },
            playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        source.guide_voices[0] = GuideVoice {
            active: true,
            asset: Some(TimelineGuideAssetKey::Intro),
            position: 0.0,
            playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        source.guide_pending.fill(Some(pending));
        source.guide_pending_len = MAX_PENDING_GUIDE_EVENTS;
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Complete,
                    },
                    playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::QueueFull);
        assert_eq!(source.guide_pending_len, 0);
        assert!(source.guide_voices.iter().all(|voice| !voice.active));
    }

    #[test]
    fn authority_reset_callback_interleave_is_silent_without_a_false_fault() {
        let (control, mut source) = source_at(48_000, 1);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Intro,
                    },
                    playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();

        // Force the callback into the exact odd-seqlock reset/drain window.
        control
            .shared
            .authority_sequence
            .store(1, Ordering::Release);
        assert_eq!(source.next(), Some(0.0));
        control.queue.clear();
        control
            .shared
            .schedule_generation
            .store(2, Ordering::Relaxed);
        control
            .shared
            .authority_sequence
            .store(2, Ordering::Release);

        assert_eq!(source.next(), Some(0.0));
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
        assert_eq!(control.fault_status().count, 0);
        assert_eq!(control.queue.len(), 0);
    }

    #[test]
    fn enqueue_near_due_decision_keeps_one_frame_after_forced_progress() {
        let (control, mut source) = source_at(48_000, 1);
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(control.next_output_frame(), 1);
        assert!(control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 2,
                    sequence: 1,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .is_err());
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 3,
                    sequence: 2,
                    kind: TimelineCueEventKind::Click { accented: true },
                    playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();

        // A callback frame can complete immediately after publication and the
        // accepted event still remains in the future instead of becoming late.
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.0));
        assert!((source.next().unwrap() - CLICK_ENVELOPE_START).abs() < 1e-6);
    }

    #[test]
    fn batch_sequence_regression_fails_before_queue_publication() {
        let (control, mut source) = source_at(48_000, 1);
        let events = [
            TimelineCueEvent {
                canonical_frame: 1,
                sequence: 2,
                kind: TimelineCueEventKind::Click { accented: true },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
            TimelineCueEvent {
                canonical_frame: 2,
                sequence: 1,
                kind: TimelineCueEventKind::Click { accented: false },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
        ];
        assert!(control
            .enqueue_batch(authority(1, 1).fence, &events)
            .is_err());
        assert_eq!(control.queue.len(), 0);
        assert!(collect_frames(&mut source, 4)
            .iter()
            .all(|sample| *sample == 0.0));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::OutOfOrder
        );
    }

    #[test]
    fn spsc_consumer_is_one_attempt_and_batch_publication_is_atomic() {
        let queue = TimelineCueSpscQueue::new(2);
        let publication_state = AtomicU8::new(2);
        let first = ScheduledTimelineCueEvent {
            fence: authority(1, 1).fence,
            output_frame: 4,
            sequence: 1,
            kind: TimelineCueEventKind::Click { accented: true },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        let second = ScheduledTimelineCueEvent {
            sequence: 2,
            output_frame: 5,
            ..first
        };

        // An unpublished producer slot causes one failed CAS and immediate
        // return. There is no loop, Backoff, yield, or retry in pop_once.
        queue.slots[0]
            .state
            .store(SPSC_SLOT_WRITING, Ordering::Release);
        queue.published_tail.store(1, Ordering::Release);
        assert_eq!(queue.pop_once(&publication_state), None);
        assert_eq!(queue.head.load(Ordering::Acquire), 0);
        assert_eq!(
            queue.slots[0].state.load(Ordering::Acquire),
            SPSC_SLOT_WRITING
        );
        queue.slots[0]
            .state
            .store(SPSC_SLOT_EMPTY, Ordering::Release);
        queue.published_tail.store(0, Ordering::Release);

        queue.push_batch(&[first]).unwrap();
        assert!(queue.push_batch(&[second, second]).is_err());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop_once(&publication_state), Some(first));
        assert_eq!(queue.pop_once(&publication_state), None);
    }

    #[test]
    fn tail_then_publication_state_check_blocks_uncommitted_pending_escape() {
        let (control, mut source) = source_at(48_000, 1);
        // Callback passed its frame-start check while the state was stable.
        assert!(source.shared.callback_begin_frame());
        let token = control.shared.begin_event_publication().unwrap();
        assert!(token.started);
        let event = ScheduledTimelineCueEvent {
            fence: authority(1, 1).fence,
            output_frame: 0,
            sequence: 1,
            kind: TimelineCueEventKind::Guide {
                asset: TimelineGuideAssetKey::Complete,
            },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        let staged = control.queue.stage_batch(&[event]).unwrap();
        control.queue.publish_staged(staged);

        // The consumer sees the new tail, then invalidates WRITING before the
        // slot CAS. Nothing can reach pending or a voice before rollback.
        assert!(source.activate_due_events(authority(1, 1).fence));
        assert!(source.pending.is_none());
        assert!(source.guide_voices.iter().all(|voice| !voice.active));
        assert!(!control.shared.commit_event_publication(token));
        control.queue.rollback_staged(staged);
        control.shared.reject_event_publication();
        assert_eq!(control.queue.len(), 0);
        assert!(collect_frames(&mut source, 8)
            .iter()
            .all(|sample| *sample == 0.0));

        // A normally committed tail remains observable exactly once.
        let (committed_control, mut committed_source) = source_at(48_000, 1);
        committed_control
            .enqueue_batch(
                authority(1, 1).fence,
                &[TimelineCueEvent {
                    canonical_frame: 0,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Complete,
                    },
                    playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        assert_eq!(committed_control.queue.len(), 1);
        assert_ne!(committed_source.next(), Some(0.0));
        assert_eq!(committed_control.queue.len(), 0);
        assert!(committed_source.pending.is_none());
        assert!(committed_source.guide_voices[0].active);
    }

    #[test]
    fn frame_zero_publication_interleave_rejects_batch_and_advances_clock() {
        let (control, mut source) = source_at(48_000, 2);
        let token = control.shared.begin_event_publication().unwrap();
        assert!(!token.started);

        // First channel enters while the producer owns the publication seam.
        assert_eq!(source.next(), Some(0.0));
        let event = ScheduledTimelineCueEvent {
            fence: authority(1, 1).fence,
            output_frame: 0,
            sequence: 1,
            kind: TimelineCueEventKind::Click { accented: true },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        let staged = control.queue.stage_batch(&[event]).unwrap();
        control.queue.publish_staged(staged);
        assert!(!control.shared.commit_event_publication(token));
        control.queue.rollback_staged(staged);
        control.shared.reject_event_publication();

        // Complete the same silent stereo frame. The device and logical clock
        // advance normally; the losing frame-zero batch remains invisible.
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.output_frame, 1);
        assert_eq!(control.next_output_frame(), 1);
        assert_eq!(control.queue.len(), 0);
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn long_running_producer_preemption_never_stops_the_callback_clock() {
        let (control, mut source) = source_at(48_000, 1);
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.output_frame, 1);
        let token = control.shared.begin_event_publication().unwrap();
        assert!(token.started);
        let event = ScheduledTimelineCueEvent {
            fence: authority(1, 1).fence,
            output_frame: 100,
            sequence: 1,
            kind: TimelineCueEventKind::Click { accented: true },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        let staged = control.queue.stage_batch(&[event]).unwrap();

        // Simulate a producer being descheduled for many device frames. The
        // first callback invalidates it; every callback remains wait-free and
        // the logical/device frame advances on every sample.
        assert!(collect_frames(&mut source, 16)
            .iter()
            .all(|sample| *sample == 0.0));
        assert_eq!(source.output_frame, 17);
        assert_eq!(control.next_output_frame(), 17);

        control.queue.publish_staged(staged);
        assert!(!control.shared.commit_event_publication(token));
        control.queue.rollback_staged(staged);
        control.shared.reject_event_publication();
        assert_eq!(control.queue.len(), 0);
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::None);
    }

    #[test]
    fn publication_race_never_interrupts_an_active_one_second_guide() {
        let make = || {
            create_timeline_cue_audio_source(
                48_000,
                1,
                16,
                asset_bank_with(48_000, |_| vec![0.2; 48_000]),
                authority(1, 1),
                0.0,
                1.0,
            )
            .unwrap()
        };
        let (baseline_control, mut baseline) = make();
        let (raced_control, mut raced) = make();
        let event = TimelineCueEvent {
            canonical_frame: 0,
            sequence: 1,
            kind: TimelineCueEventKind::Guide {
                asset: TimelineGuideAssetKey::Intro,
            },
            playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
        };
        baseline_control
            .enqueue_batch(authority(1, 1).fence, &[event])
            .unwrap();
        raced_control
            .enqueue_batch(authority(1, 1).fence, &[event])
            .unwrap();
        assert_eq!(
            collect_frames(&mut baseline, 16),
            collect_frames(&mut raced, 16)
        );

        let token = raced_control.shared.begin_event_publication().unwrap();
        let staged = raced_control
            .queue
            .stage_batch(&[ScheduledTimelineCueEvent {
                fence: authority(1, 1).fence,
                output_frame: 1_000,
                sequence: 2,
                kind: TimelineCueEventKind::Guide {
                    asset: TimelineGuideAssetKey::Complete,
                },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            }])
            .unwrap();
        let expected = collect_frames(&mut baseline, 64);
        let actual = collect_frames(&mut raced, 64);
        assert_eq!(actual, expected);
        assert!(actual.iter().all(|sample| *sample != 0.0));
        assert_eq!(baseline.output_frame, raced.output_frame);
        assert_eq!(
            baseline.guide_voices[0].position,
            raced.guide_voices[0].position
        );

        raced_control.queue.publish_staged(staged);
        assert!(!raced_control.shared.commit_event_publication(token));
        raced_control.queue.rollback_staged(staged);
        raced_control.shared.reject_event_publication();
        assert_eq!(raced_control.queue.len(), 0);
        assert_eq!(
            collect_frames(&mut baseline, 32),
            collect_frames(&mut raced, 32)
        );
        assert_eq!(
            raced_control.fault_status().code,
            TimelineCueFaultCode::None
        );
    }

    #[test]
    fn overflow_and_queue_full_fail_before_partial_publication() {
        let (control, mut source) = create_timeline_cue_audio_source(
            48_000,
            1,
            1,
            asset_bank(48_000),
            TimelineCueAuthority {
                fence: authority(1, 1).fence,
                clock: TimelineCueClockMap {
                    canonical_anchor_frame: 0,
                    output_anchor_frame: u64::MAX,
                },
            },
            1.0,
            1.0,
        )
        .unwrap();
        let events = [
            TimelineCueEvent {
                canonical_frame: 1,
                sequence: 1,
                kind: TimelineCueEventKind::Click { accented: true },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
            TimelineCueEvent {
                canonical_frame: 2,
                sequence: 2,
                kind: TimelineCueEventKind::Click { accented: false },
                playback_rate_milli: TIMELINE_GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
        ];
        assert!(control
            .enqueue_batch(authority(1, 1).fence, &events)
            .is_err());
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(control.fault_status().code, TimelineCueFaultCode::QueueFull);

        let (control, mut source) = source_at(48_000, 1);
        let overflow = TimelineCueAuthority {
            fence: authority(1, 2).fence,
            clock: TimelineCueClockMap {
                canonical_anchor_frame: 0,
                output_anchor_frame: u64::MAX,
            },
        };
        control.publish_authority(overflow).unwrap();
        let mixed_mapping = [
            TimelineCueEvent {
                canonical_frame: 0,
                sequence: 1,
                kind: TimelineCueEventKind::Click { accented: true },
                playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
            TimelineCueEvent {
                canonical_frame: 1,
                sequence: 2,
                kind: TimelineCueEventKind::Click { accented: false },
                playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
            },
        ];
        assert!(control
            .enqueue_batch(overflow.fence, &mixed_mapping)
            .is_err());
        assert_eq!(control.queue.len(), 0);
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::FrameMappingOverflow
        );
    }

    #[test]
    fn exact_authority_retry_preserves_queue_but_same_fence_shape_change_fails() {
        let (control, mut source) = source_at(48_000, 1);
        let initial = authority(1, 1);
        control
            .enqueue_batch(
                initial.fence,
                &[TimelineCueEvent {
                    canonical_frame: 2,
                    sequence: 1,
                    kind: TimelineCueEventKind::Guide {
                        asset: TimelineGuideAssetKey::Complete,
                    },
                    playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                }],
            )
            .unwrap();
        control.publish_authority(initial).unwrap();
        let frames = collect_frames(&mut source, 3);
        assert_eq!(&frames[..2], &[0.0, 0.0]);
        assert_ne!(frames[2], 0.0);

        let changed_clock = TimelineCueAuthority {
            clock: TimelineCueClockMap {
                canonical_anchor_frame: 0,
                output_anchor_frame: 9,
            },
            ..initial
        };
        assert!(control.publish_authority(changed_clock).is_err());
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::InvalidFormat
        );

        control.publish_authority(authority(1, 2)).unwrap();
        assert!(control.publish_authority(initial).is_err());
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::StaleFence
        );
    }

    #[test]
    fn authority_and_output_frame_overflow_are_silent_without_partial_frame() {
        let (control, mut source) = source_at(48_000, 2);
        let previous = control.shared.read_authority().unwrap();
        control
            .shared
            .authority_sequence
            .store(u64::MAX - 1, Ordering::Release);
        assert!(control.publish_authority(authority(1, 2)).is_err());
        assert_eq!(control.shared.read_authority(), Some(previous));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::AuthorityExhausted
        );

        source.output_frame = u64::MAX;
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::OutputFrameExhausted
        );
    }

    #[test]
    fn callback_descriptors_are_copy_only_and_source_format_is_infinite() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<ScheduledTimelineCueEvent>();
        assert!(!std::mem::needs_drop::<ScheduledTimelineCueEvent>());
        let (_, source) = source_at(44_100, 6);
        assert_eq!(source.channels(), 6);
        assert_eq!(source.sample_rate(), 44_100);
        assert_eq!(source.current_span_len(), None);
        assert_eq!(source.total_duration(), None);
        assert!(create_timeline_cue_audio_source(
            0,
            1,
            1,
            asset_bank(44_100),
            authority(1, 1),
            1.0,
            1.0,
        )
        .is_err());
        assert!(create_timeline_cue_audio_source(
            44_100,
            MAX_TIMELINE_CUE_OUTPUT_CHANNELS + 1,
            1,
            asset_bank(44_100),
            authority(1, 1),
            1.0,
            1.0,
        )
        .is_err());
    }

    #[test]
    fn callback_render_path_allocates_and_deallocates_nothing() {
        let (control, mut source) = source_at(48_000, 2);
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 1,
                        kind: TimelineCueEventKind::Click { accented: true },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 2,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Complete,
                        },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                ],
            )
            .unwrap();
        CALLBACK_ALLOCATION_ACTIVITY.with(|count| count.set(0));
        TRACK_CALLBACK_ALLOCATIONS.with(|tracking| tracking.set(true));
        for _ in 0..2_000 {
            std::hint::black_box(source.next());
        }
        TRACK_CALLBACK_ALLOCATIONS.with(|tracking| tracking.set(false));
        assert_eq!(CALLBACK_ALLOCATION_ACTIVITY.with(Cell::get), 0);
    }

    #[test]
    fn unexpected_over_unity_is_not_limited_and_silences_the_whole_frame() {
        let bank = asset_bank_with(48_000, |_| vec![MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS]);
        let (control, mut source) =
            create_timeline_cue_audio_source(48_000, 2, 16, bank, authority(1, 1), 0.0, 0.0)
                .unwrap();
        control
            .enqueue_batch(
                authority(1, 1).fence,
                &[
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 1,
                        kind: TimelineCueEventKind::Click { accented: true },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                    TimelineCueEvent {
                        canonical_frame: 0,
                        sequence: 2,
                        kind: TimelineCueEventKind::Guide {
                            asset: TimelineGuideAssetKey::Complete,
                        },
                        playback_rate_milli: GUIDE_PLAYBACK_RATE_DEFAULT_MILLI,
                    },
                ],
            )
            .unwrap();

        // Simulate an impossible control-plane corruption after preflight.
        // The callback must report and silence, never clamp to +/-1.0.
        control.shared.gain_sequence.store(1, Ordering::Release);
        control
            .shared
            .click_gain_bits
            .store(2.0_f32.to_bits(), Ordering::Relaxed);
        control
            .shared
            .guide_gain_bits
            .store(2.0_f32.to_bits(), Ordering::Relaxed);
        control.shared.gain_sequence.store(2, Ordering::Release);
        source.click_gain = GainSlew::new(2.0, 48_000);
        source.guide_gain = GainSlew::new(2.0, 48_000);
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(
            control.fault_status().code,
            TimelineCueFaultCode::OutputOverUnity
        );
        assert!(source.click_voices.iter().all(|voice| !voice.active));
        assert!(source.guide_voices.iter().all(|voice| !voice.active));
    }

    #[test]
    fn fault_snapshot_is_coherent_and_count_never_wraps() {
        let (control, _) = source_at(48_000, 1);
        control
            .shared
            .record_fault(TimelineCueFaultCode::QueueFull, 7);
        assert_eq!(
            control.fault_status(),
            TimelineCueFaultStatus {
                code: TimelineCueFaultCode::QueueFull,
                count: 1,
                sequence: 7,
            }
        );

        control
            .shared
            .fault_count
            .store(u64::MAX, Ordering::Relaxed);
        control
            .shared
            .record_fault(TimelineCueFaultCode::NonFiniteOutput, 9);
        let terminal = control.fault_status();
        assert_eq!(terminal.code, TimelineCueFaultCode::NonFiniteOutput);
        assert_eq!(terminal.count, u64::MAX);
        assert_eq!(terminal.sequence, 9);

        control
            .shared
            .fault_snapshot_sequence
            .store(u64::MAX - 1, Ordering::Release);
        control
            .shared
            .record_fault(TimelineCueFaultCode::PastDue, 10);
        assert_eq!(control.fault_status(), terminal);
    }

    fn unique_test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "syndocal-timeline-cue-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn machine_settings_missing_default_and_roundtrip_are_strict() {
        let directory = unique_test_directory("settings-roundtrip");
        let path = timeline_cue_audio_settings_path(&directory);
        assert_eq!(
            load_timeline_cue_audio_settings_from_path(&path).unwrap(),
            MachineTimelineCueAudioSettingsV1::default()
        );
        let settings = MachineTimelineCueAudioSettingsV1 {
            version: 1,
            route: TimelineCueAudioRoute::ExplicitDevice,
            device_name: Some("Cue Monitor".to_string()),
            topology_fingerprint: Some("A1B2C3".to_string()),
            click_gain: 0.7,
            guide_gain: 1.25,
        };
        persist_timeline_cue_audio_settings_to_path_with(&path, &settings, |temporary, target| {
            fs::rename(temporary, target).map_err(|error| error.to_string())
        })
        .unwrap();
        assert_eq!(
            load_timeline_cue_audio_settings_from_path(&path).unwrap(),
            settings
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn machine_settings_corrupt_future_invalid_and_oversize_fail_closed() {
        let directory = unique_test_directory("settings-invalid");
        fs::create_dir_all(&directory).unwrap();
        let path = timeline_cue_audio_settings_path(&directory);
        fs::write(&path, b"{").unwrap();
        assert!(load_timeline_cue_audio_settings_from_path(&path).is_err());
        fs::write(
            &path,
            br#"{"version":1,"route":"follow_program","device_name":null,"click_gain":1.0,"guide_gain":0.8,"unknown":true}"#,
        )
        .unwrap();
        assert!(load_timeline_cue_audio_settings_from_path(&path).is_err());
        fs::write(
            &path,
            br#"{"version":2,"route":"follow_program","device_name":null,"click_gain":1.0,"guide_gain":0.8}"#,
        )
        .unwrap();
        assert!(load_timeline_cue_audio_settings_from_path(&path).is_err());
        let unsafe_headroom = br#"{"version":1,"route":"follow_program","device_name":null,"click_gain":2.0,"guide_gain":2.0}"#;
        fs::write(&path, unsafe_headroom).unwrap();
        assert!(load_timeline_cue_audio_settings_from_path(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), unsafe_headroom);
        let replace_called = Cell::new(false);
        assert!(persist_timeline_cue_audio_settings_to_path_with(
            &path,
            &MachineTimelineCueAudioSettingsV1 {
                click_gain: 2.0,
                guide_gain: 2.0,
                ..MachineTimelineCueAudioSettingsV1::default()
            },
            |_, _| {
                replace_called.set(true);
                Ok(())
            },
        )
        .is_err());
        assert!(!replace_called.get());
        assert_eq!(fs::read(&path).unwrap(), unsafe_headroom);
        fs::write(
            &path,
            vec![b'x'; MAX_TIMELINE_CUE_SETTINGS_BYTES as usize + 1],
        )
        .unwrap();
        assert!(load_timeline_cue_audio_settings_from_path(&path).is_err());
        assert!(MachineTimelineCueAudioSettingsV1 {
            click_gain: f32::NAN,
            ..MachineTimelineCueAudioSettingsV1::default()
        }
        .validated()
        .is_err());
        assert!(MachineTimelineCueAudioSettingsV1::default()
            .validated()
            .is_ok());
        assert!(MachineTimelineCueAudioSettingsV1 {
            route: TimelineCueAudioRoute::ExplicitDevice,
            ..MachineTimelineCueAudioSettingsV1::default()
        }
        .validated()
        .is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn machine_settings_replace_failure_preserves_previous_and_cleans_temp() {
        let directory = unique_test_directory("settings-write-failure");
        fs::create_dir_all(&directory).unwrap();
        let path = timeline_cue_audio_settings_path(&directory);
        fs::write(&path, b"previous").unwrap();
        let error = persist_timeline_cue_audio_settings_to_path_with(
            &path,
            &MachineTimelineCueAudioSettingsV1::default(),
            |_, _| Err("injected replace failure".to_string()),
        )
        .unwrap_err();
        assert!(error.contains("injected replace failure"));
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn guide_asset_catalog_requires_complete_and_resamples_before_callback() {
        let decoded = TimelineGuideAssetKey::ALL
            .into_iter()
            .map(|key| DecodedGuideAsset::from_mono_pcm(key, 48_000, vec![0.0; 480]).unwrap())
            .collect::<Vec<_>>();
        assert!(GuideAssetBank::prepare_complete(44_100, decoded).is_ok());
        let incomplete = TimelineGuideAssetKey::ALL[..TIMELINE_GUIDE_ASSET_COUNT - 1]
            .iter()
            .copied()
            .map(|key| DecodedGuideAsset::from_mono_pcm(key, 48_000, vec![0.0]).unwrap())
            .collect();
        assert!(GuideAssetBank::prepare_complete(48_000, incomplete)
            .unwrap_err()
            .contains("exactly"));
        assert_eq!(
            TimelineGuideAssetKey::ALL.last(),
            Some(&TimelineGuideAssetKey::Complete)
        );
    }

    #[test]
    fn guide_manifest_is_exactly_twelve_assets_in_exporter_order() {
        assert_eq!(TIMELINE_GUIDE_ASSET_COUNT, 12);
        assert_eq!(
            TimelineGuideAssetKey::ALL,
            [
                TimelineGuideAssetKey::Intro,
                TimelineGuideAssetKey::Verse,
                TimelineGuideAssetKey::PreChorus,
                TimelineGuideAssetKey::Chorus,
                TimelineGuideAssetKey::Interlude,
                TimelineGuideAssetKey::Bridge,
                TimelineGuideAssetKey::Breakdown,
                TimelineGuideAssetKey::Outro,
                TimelineGuideAssetKey::Looping,
                TimelineGuideAssetKey::Break,
                TimelineGuideAssetKey::Trans,
                TimelineGuideAssetKey::Complete,
            ]
        );
        assert_eq!(TimelineGuideAssetKey::Interlude.index(), 4);
        assert_eq!(TimelineGuideAssetKey::Complete.index(), 11);
    }

    #[test]
    fn embedded_wav_manifest_hashes_and_frame_counts_are_exact() {
        let expected_files = [
            "intro.wav",
            "verse.wav",
            "pre_chorus.wav",
            "chorus.wav",
            "interlude.wav",
            "bridge.wav",
            "breakdown.wav",
            "outro.wav",
            "looping.wav",
            "break_word.wav",
            "trans.wav",
            "complete.wav",
        ];
        assert_eq!(TIMELINE_GUIDE_EMBEDDED_ASSETS.len(), 12);
        let mut decoded = Vec::with_capacity(TIMELINE_GUIDE_ASSET_COUNT);
        for ((asset, key), file_name) in TIMELINE_GUIDE_EMBEDDED_ASSETS
            .iter()
            .zip(TimelineGuideAssetKey::ALL)
            .zip(expected_files)
        {
            assert_eq!(asset.key, key);
            assert_eq!(asset.file_name, file_name);
            assert_eq!(format!("{:X}", Sha256::digest(asset.wav)), asset.sha256);
            let wav = DecodedGuideAsset::decode_wav(asset.key, asset.wav).unwrap();
            assert_eq!(wav.sample_rate, asset.source_sample_rate);
            assert_eq!(wav.channels, 1);
            assert_eq!(
                wav.samples.len() as u64 / u64::from(wav.channels),
                asset.source_frames
            );
            decoded.push(wav);
        }
        let bank = GuideAssetBank::prepare_complete(48_000, decoded).unwrap();
        for asset in TIMELINE_GUIDE_EMBEDDED_ASSETS {
            assert_eq!(
                bank.samples(asset.key).len() as u64,
                asset.output_48k_frames
            );
        }
        assert!(bank.peak_abs() <= MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS);
    }

    #[test]
    fn exporter_resampler_rounds_frame_count_and_interpolates_in_f64() {
        assert_eq!(
            resample_mono_linear_f64(&vec![0.0; 441], 44_100, 48_000)
                .unwrap()
                .len(),
            480
        );
        assert_eq!(
            resample_mono_linear_f64(&vec![0.0; 480], 48_000, 44_100)
                .unwrap()
                .len(),
            441
        );
        assert_eq!(resample_mono_linear_f64(&[0.25], 2, 1).unwrap(), vec![0.25]);
        assert_eq!(
            resample_mono_linear_f64(&[0.0, 1.0], 2, 4).unwrap(),
            vec![0.0, 0.5, 1.0, 1.0]
        );
        assert_eq!(
            resample_mono_linear_f64(&[0.0, 1.0, 0.0], 3, 2).unwrap(),
            vec![0.0, 0.5]
        );
    }

    #[test]
    fn exporter_peak_and_mixer_headroom_fail_closed_before_apply() {
        assert!(validate_mixer_headroom(1.0, 0.85, MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS).is_ok());
        assert!(validate_mixer_headroom(2.0, 1.34, MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS).is_err());
        let over_peak = TimelineGuideAssetKey::ALL
            .into_iter()
            .map(|key| {
                DecodedGuideAsset::from_mono_pcm(
                    key,
                    48_000,
                    vec![MAX_TIMELINE_GUIDE_ASSET_PEAK_ABS + 0.001],
                )
                .unwrap()
            })
            .collect();
        assert!(GuideAssetBank::prepare_complete(48_000, over_peak)
            .unwrap_err()
            .contains("exporter contract"));
    }
}
