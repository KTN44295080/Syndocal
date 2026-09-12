//! Tauri-owned ShowClock process runtime.
//!
//! The protocol crate owns authentication, estimation, scheduling, and
//! fencing policy.  This module owns only the process lifecycle and the
//! manually configured LAN worker.  It deliberately has no path that arms
//! lighting/video/audio output; that remains a separate local ownership
//! operation.

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use io::show_clock_lan::{ShowClockLanError, ShowClockLanMessage, ShowClockLanTransport};
use protocol::{
    show_clock::{
        AuthenticatedShowClockSample, ShowClockHash, ShowClockNodeId, ShowClockNonce,
        ShowClockPeerValidator, ShowClockSample, ShowClockSessionId, ShowClockSource,
        ShowTransportState, SHOW_CLOCK_PROTOCOL_VERSION, SHOW_CLOCK_SCHEMA_VERSION,
    },
    show_clock_runtime::{ShowClockEstimatorState, ShowClockPeerEstimator},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const RECEIVE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const SAMPLE_EXPIRY_US: u64 = 250_000;
const MAX_USED_SESSIONS_PER_PROCESS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowClockIpcRole {
    Primary,
    Standby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ShowClockIpcPhase {
    Stopped,
    Acquiring,
    Locked,
    Hold,
    Stale,
    Fault,
}

impl ShowClockIpcPhase {
    fn from_estimator(state: ShowClockEstimatorState) -> Self {
        match state {
            ShowClockEstimatorState::Acquiring => Self::Acquiring,
            ShowClockEstimatorState::Locked => Self::Locked,
            ShowClockEstimatorState::Hold => Self::Hold,
            ShowClockEstimatorState::Stale => Self::Stale,
            ShowClockEstimatorState::Fault => Self::Fault,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ShowClockIpcStatus {
    pub running: bool,
    pub role: Option<ShowClockIpcRole>,
    pub state: ShowClockIpcPhase,
    pub local_address: Option<String>,
    pub peer_address: Option<String>,
    pub session_id: Option<String>,
    pub node_id: Option<String>,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub accepted_samples: u32,
    pub last_sequence: u64,
    pub offset_us: i64,
    pub sample_age_us: Option<u64>,
    pub output_armed: bool,
    pub last_error: Option<String>,
}

impl Default for ShowClockIpcStatus {
    fn default() -> Self {
        Self {
            running: false,
            role: None,
            state: ShowClockIpcPhase::Stopped,
            local_address: None,
            peer_address: None,
            session_id: None,
            node_id: None,
            clock_generation: 0,
            fencing_generation: 0,
            accepted_samples: 0,
            last_sequence: 0,
            offset_us: 0,
            sample_age_us: None,
            output_armed: false,
            last_error: None,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockStartRequest {
    pub role: ShowClockIpcRole,
    pub bind_address: String,
    pub peer_address: String,
    pub session_id: String,
    pub node_id: String,
    /// Standby validates that the authenticated sender is exactly this node.
    /// Primary does not use this value, but it is still validated so the two
    /// machine configurations have the same shape.
    pub peer_node_id: String,
    pub project_hash_hex: String,
    pub media_hash_hex: String,
    /// The pairing key is accepted for this process only and is never echoed
    /// in status or persisted by this runtime.
    pub key_hex: String,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub bpm_milli: u32,
    pub initial_show_time_us: u64,
}

#[derive(Clone)]
struct ShowClockConfig {
    role: ShowClockIpcRole,
    bind: SocketAddr,
    peer: SocketAddr,
    session_id: ShowClockSessionId,
    node_id: ShowClockNodeId,
    peer_node_id: ShowClockNodeId,
    project_hash: ShowClockHash,
    media_hash: ShowClockHash,
    key: [u8; 32],
    clock_generation: u64,
    fencing_generation: u64,
    bpm_milli: u32,
    initial_show_time_us: u64,
}

struct ShowClockWorker {
    stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

pub struct ShowClockIpcState {
    status: Arc<Mutex<ShowClockIpcStatus>>,
    operation: Mutex<()>,
    lifecycle: Mutex<Option<ShowClockWorker>>,
    used_sessions: Mutex<HashSet<String>>,
}

impl Default for ShowClockIpcState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(ShowClockIpcStatus::default())),
            operation: Mutex::new(()),
            lifecycle: Mutex::new(None),
            used_sessions: Mutex::new(HashSet::new()),
        }
    }
}

impl ShowClockIpcState {
    pub fn status(&self) -> Result<ShowClockIpcStatus, String> {
        self.status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "ShowClock status lock was poisoned; restart Syndocal".to_string())
    }

    pub fn start(&self, request: ShowClockStartRequest) -> Result<ShowClockIpcStatus, String> {
        let config = ShowClockConfig::try_from(request)?;
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        {
            let used_sessions = self.used_sessions.lock().map_err(|_| {
                "ShowClock session registry lock was poisoned; restart Syndocal".to_string()
            })?;
            if used_sessions.contains(config.session_id.as_str()) {
                return Err(
                    "ShowClock session id was already used by this process; choose a fresh paired session"
                        .to_string(),
                );
            }
            if used_sessions.len() >= MAX_USED_SESSIONS_PER_PROCESS {
                return Err(
                    "ShowClock process session registry is exhausted; restart Syndocal".to_string(),
                );
            }
        }
        // Release the previous socket before binding the replacement. An
        // explicit restart is also the replay/estimator incarnation boundary.
        self.stop_worker()?;
        {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "ShowClock status lock was poisoned; restart Syndocal".to_string())?;
            status.running = false;
            status.state = ShowClockIpcPhase::Stopped;
            status.output_armed = false;
            status.last_error = None;
        }
        let transport = ShowClockLanTransport::bind(config.bind, config.peer)
            .map_err(|error| error.to_string())?;
        let local_address = transport.local_addr().map_err(|error| error.to_string())?;
        self.used_sessions
            .lock()
            .map_err(|_| {
                "ShowClock session registry lock was poisoned; restart Syndocal".to_string()
            })?
            .insert(config.session_id.as_str().to_string());
        {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "ShowClock status lock was poisoned; restart Syndocal".to_string())?;
            *status = ShowClockIpcStatus {
                running: true,
                role: Some(config.role),
                state: match config.role {
                    ShowClockIpcRole::Primary => ShowClockIpcPhase::Locked,
                    ShowClockIpcRole::Standby => ShowClockIpcPhase::Acquiring,
                },
                local_address: Some(local_address.to_string()),
                peer_address: Some(config.peer.to_string()),
                session_id: Some(config.session_id.as_str().to_string()),
                node_id: Some(config.node_id.as_str().to_string()),
                clock_generation: config.clock_generation,
                fencing_generation: config.fencing_generation,
                accepted_samples: 0,
                last_sequence: 0,
                offset_us: 0,
                sample_age_us: None,
                // This worker has no physical-output authority by design.
                output_armed: false,
                last_error: None,
            };
        }

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_status = Arc::clone(&self.status);
        let join = thread::Builder::new()
            .name(format!("syndocal-show-clock-{:?}", config.role))
            .spawn(move || run_worker(config, transport, worker_stop, worker_status))
            .map_err(|error| format!("ShowClock worker could not start: {error}"))?;

        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| "ShowClock lifecycle lock was poisoned; restart Syndocal".to_string())?;
        *lifecycle = Some(ShowClockWorker { stop, join });
        drop(lifecycle);
        self.status()
    }

    pub fn stop(&self) -> Result<ShowClockIpcStatus, String> {
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        self.stop_worker()?;
        let mut status = self
            .status
            .lock()
            .map_err(|_| "ShowClock status lock was poisoned; restart Syndocal".to_string())?;
        status.running = false;
        status.state = ShowClockIpcPhase::Stopped;
        status.output_armed = false;
        Ok(status.clone())
    }

    fn stop_worker(&self) -> Result<(), String> {
        let worker = {
            let mut lifecycle = self.lifecycle.lock().map_err(|_| {
                "ShowClock lifecycle lock was poisoned; restart Syndocal".to_string()
            })?;
            lifecycle.take()
        };
        if let Some(worker) = worker {
            worker.stop.store(true, Ordering::Release);
            worker
                .join
                .join()
                .map_err(|_| "ShowClock worker did not terminate cleanly".to_string())?;
        }
        Ok(())
    }
}

impl Drop for ShowClockIpcState {
    fn drop(&mut self) {
        let worker = self.lifecycle.get_mut().ok().and_then(Option::take);
        if let Some(worker) = worker {
            worker.stop.store(true, Ordering::Release);
            let _ = worker.join.join();
        }
    }
}

impl TryFrom<ShowClockStartRequest> for ShowClockConfig {
    type Error = String;

    fn try_from(request: ShowClockStartRequest) -> Result<Self, Self::Error> {
        let bind = request
            .bind_address
            .parse::<SocketAddr>()
            .map_err(|error| format!("Invalid ShowClock bind address: {error}"))?;
        let peer = request
            .peer_address
            .parse::<SocketAddr>()
            .map_err(|error| format!("Invalid ShowClock peer address: {error}"))?;
        let session_id = ShowClockSessionId::new(request.session_id)
            .map_err(|error| format!("Invalid ShowClock session id: {error}"))?;
        let node_id = ShowClockNodeId::new(request.node_id)
            .map_err(|error| format!("Invalid ShowClock node id: {error}"))?;
        let peer_node_id = ShowClockNodeId::new(request.peer_node_id)
            .map_err(|error| format!("Invalid ShowClock peer node id: {error}"))?;
        if request.clock_generation == 0 || request.fencing_generation == 0 {
            return Err("ShowClock generations must be non-zero".to_string());
        }
        if !(30_000..=300_000).contains(&request.bpm_milli) {
            return Err("ShowClock BPM must be between 30 and 300 BPM".to_string());
        }
        if request.initial_show_time_us == 0 {
            return Err("ShowClock initial show time must be non-zero".to_string());
        }
        Ok(Self {
            role: request.role,
            bind,
            peer,
            session_id,
            node_id,
            peer_node_id,
            project_hash: ShowClockHash(parse_fixed_hex::<32>(
                &request.project_hash_hex,
                "project hash",
            )?),
            media_hash: ShowClockHash(parse_fixed_hex::<32>(
                &request.media_hash_hex,
                "media hash",
            )?),
            key: parse_fixed_hex::<32>(&request.key_hex, "pairing key")?,
            clock_generation: request.clock_generation,
            fencing_generation: request.fencing_generation,
            bpm_milli: request.bpm_milli,
            initial_show_time_us: request.initial_show_time_us,
        })
    }
}

fn parse_fixed_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N], String> {
    let value = value.trim();
    if value.len() != N * 2 {
        return Err(format!(
            "ShowClock {label} must contain exactly {} hex characters",
            N * 2
        ));
    }
    let bytes = value.as_bytes();
    let mut output = [0_u8; N];
    for (index, slot) in output.iter_mut().enumerate() {
        let high = hex_nibble(bytes[index * 2])
            .ok_or_else(|| format!("ShowClock {label} contains non-hex data"))?;
        let low = hex_nibble(bytes[index * 2 + 1])
            .ok_or_else(|| format!("ShowClock {label} contains non-hex data"))?;
        *slot = (high << 4) | low;
    }
    if output.iter().all(|byte| *byte == 0) {
        return Err(format!("ShowClock {label} must not be all zero"));
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn run_worker(
    config: ShowClockConfig,
    transport: ShowClockLanTransport,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<ShowClockIpcStatus>>,
) {
    let result = match config.role {
        ShowClockIpcRole::Primary => run_primary(&config, &transport, &stop, &status),
        ShowClockIpcRole::Standby => run_standby(&config, &transport, &stop, &status),
    };
    if let Err(error) = result {
        update_status(&status, |current| {
            current.running = false;
            current.state = ShowClockIpcPhase::Fault;
            current.output_armed = false;
            current.last_error = Some(error);
        });
    } else if !stop.load(Ordering::Acquire) {
        update_status(&status, |current| {
            current.running = false;
            current.state = ShowClockIpcPhase::Stopped;
            current.output_armed = false;
        });
    }
}

fn run_primary(
    config: &ShowClockConfig,
    transport: &ShowClockLanTransport,
    stop: &AtomicBool,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
) -> Result<(), String> {
    let origin = Instant::now();
    let mut sequence = 1_u64;
    let mut next_send = origin;
    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        if now >= next_send {
            let elapsed_us = monotonic_elapsed_us(origin, now);
            let show_time_us = config
                .initial_show_time_us
                .saturating_add(elapsed_us)
                .max(1);
            let sample = AuthenticatedShowClockSample::sign(
                ShowClockSample {
                    schema_version: SHOW_CLOCK_SCHEMA_VERSION,
                    protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
                    session_id: config.session_id.clone(),
                    sender: config.node_id.clone(),
                    clock_generation: config.clock_generation,
                    fencing_generation: config.fencing_generation,
                    sequence,
                    sender_monotonic_us: elapsed_us,
                    show_time_us,
                    bpm_milli: config.bpm_milli,
                    beat_phase_ppm: beat_phase_ppm(show_time_us, config.bpm_milli),
                    transport: ShowTransportState::Playing,
                    source: ShowClockSource::ShowClock,
                    project_hash: config.project_hash,
                    media_hash: config.media_hash,
                    expires_after_us: SAMPLE_EXPIRY_US,
                    nonce: ShowClockNonce(sample_nonce(config, sequence)),
                },
                &config.key,
            )
            .map_err(|error| format!("ShowClock primary sample signing failed: {error}"))?;
            transport
                .send_sample(&sample)
                .map_err(|error| format!("ShowClock primary send failed: {error}"))?;
            update_status(status, |current| {
                current.last_sequence = sequence;
                current.sample_age_us = Some(0);
            });
            sequence = sequence.saturating_add(1);
            next_send = now + SAMPLE_INTERVAL;
        } else {
            thread::sleep((next_send - now).min(Duration::from_millis(10)));
        }
    }
    Ok(())
}

fn run_standby(
    config: &ShowClockConfig,
    transport: &ShowClockLanTransport,
    stop: &AtomicBool,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
) -> Result<(), String> {
    let mut validator = ShowClockPeerValidator::new(
        config.session_id.clone(),
        config.peer_node_id.clone(),
        config.project_hash,
        config.media_hash,
        config.clock_generation,
        config.fencing_generation,
        config.key,
    )
    .map_err(|error| format!("ShowClock standby validator initialization failed: {error}"))?;
    let mut estimator = ShowClockPeerEstimator::new(
        Default::default(),
        config.clock_generation,
        config.fencing_generation,
    )
    .map_err(|error| format!("ShowClock standby estimator initialization failed: {error}"))?;
    let origin = Instant::now();
    while !stop.load(Ordering::Acquire) {
        let received = match transport.receive(RECEIVE_POLL_INTERVAL) {
            Ok(message) => message,
            Err(ShowClockLanError::Timeout) => {
                let estimate = estimator
                    .estimator_mut()
                    .advance(monotonic_elapsed_us(origin, Instant::now()));
                publish_estimate(status, &estimate, validator.last_sequence());
                continue;
            }
            Err(error) => return Err(format!("ShowClock standby receive failed: {error}")),
        };
        match received {
            ShowClockLanMessage::Sample(sample) => {
                let received_at_us = monotonic_elapsed_us(origin, Instant::now());
                estimator
                    .accept_authenticated_sample(&mut validator, &sample, received_at_us)
                    .map_err(|error| format!("ShowClock standby sample rejected: {error}"))?;
                let estimate = estimator.estimator_mut().advance(received_at_us);
                publish_estimate(status, &estimate, validator.last_sequence());
            }
            ShowClockLanMessage::Action(_) => {
                return Err(
                    "ShowClock standby received an action before action runtime wiring".to_string(),
                );
            }
        }
    }
    Ok(())
}

fn publish_estimate(
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    estimate: &protocol::show_clock_runtime::ShowClockEstimate,
    last_sequence: u64,
) {
    update_status(status, |current| {
        current.state = ShowClockIpcPhase::from_estimator(estimate.state);
        current.accepted_samples = estimate.accepted_samples;
        current.last_sequence = last_sequence;
        current.offset_us = estimate.offset_us;
        current.sample_age_us = estimate.sample_age_us;
    });
}

fn update_status(
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    update: impl FnOnce(&mut ShowClockIpcStatus),
) {
    if let Ok(mut current) = status.lock() {
        update(&mut current);
    }
}

fn monotonic_elapsed_us(origin: Instant, now: Instant) -> u64 {
    now.saturating_duration_since(origin)
        .as_micros()
        .min(u64::MAX as u128) as u64
}

fn beat_phase_ppm(show_time_us: u64, bpm_milli: u32) -> u32 {
    let beat_period_us = 60_000_000_000_u64 / u64::from(bpm_milli);
    if beat_period_us == 0 {
        return 0;
    }
    ((show_time_us % beat_period_us).saturating_mul(1_000_000) / beat_period_us) as u32
}

fn sample_nonce(config: &ShowClockConfig, sequence: u64) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(config.session_id.as_str().as_bytes());
    hasher.update(config.node_id.as_str().as_bytes());
    hasher.update(sequence.to_le_bytes());
    let digest = hasher.finalize();
    let mut nonce = [0_u8; 16];
    nonce.copy_from_slice(&digest[..16]);
    if nonce == [0; 16] {
        nonce[0] = 1;
    }
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    const HASH: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const MEDIA: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const KEY: &str = "4242424242424242424242424242424242424242424242424242424242424242";

    fn request(
        role: ShowClockIpcRole,
        bind_address: String,
        peer_address: String,
        node_id: &str,
        peer_node_id: &str,
    ) -> ShowClockStartRequest {
        ShowClockStartRequest {
            role,
            bind_address,
            peer_address,
            session_id: "process-loopback-session".to_string(),
            node_id: node_id.to_string(),
            peer_node_id: peer_node_id.to_string(),
            project_hash_hex: HASH.to_string(),
            media_hash_hex: MEDIA.to_string(),
            key_hex: KEY.to_string(),
            clock_generation: 1,
            fencing_generation: 1,
            bpm_milli: 120_000,
            initial_show_time_us: 1_000_000,
        }
    }

    #[test]
    fn primary_and_standby_workers_lock_over_paired_loopback_and_stop_cleanly() {
        let primary_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let standby_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let primary_address = primary_probe.local_addr().unwrap();
        let standby_address = standby_probe.local_addr().unwrap();
        drop(primary_probe);
        drop(standby_probe);

        let primary = ShowClockIpcState::default();
        let standby = ShowClockIpcState::default();
        primary
            .start(request(
                ShowClockIpcRole::Primary,
                primary_address.to_string(),
                standby_address.to_string(),
                "node-primary",
                "node-standby",
            ))
            .unwrap();
        standby
            .start(request(
                ShowClockIpcRole::Standby,
                standby_address.to_string(),
                primary_address.to_string(),
                "node-standby",
                "node-primary",
            ))
            .unwrap();

        for _ in 0..30 {
            if standby.status().unwrap().state == ShowClockIpcPhase::Locked {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let status = standby.status().unwrap();
        assert_eq!(status.state, ShowClockIpcPhase::Locked);
        assert!(status.accepted_samples >= 3);
        assert!(!status.output_armed);

        assert_eq!(primary.stop().unwrap().state, ShowClockIpcPhase::Stopped);
        for _ in 0..20 {
            if standby.status().unwrap().state == ShowClockIpcPhase::Stale {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(standby.status().unwrap().state, ShowClockIpcPhase::Stale);
        assert_eq!(standby.stop().unwrap().state, ShowClockIpcPhase::Stopped);
        assert!(!standby.status().unwrap().running);
        assert!(!primary.status().unwrap().running);

        let reused = primary.start(request(
            ShowClockIpcRole::Primary,
            primary_address.to_string(),
            standby_address.to_string(),
            "node-primary",
            "node-standby",
        ));
        assert!(reused
            .unwrap_err()
            .contains("choose a fresh paired session"));
        let mut fresh_request = request(
            ShowClockIpcRole::Primary,
            primary_address.to_string(),
            standby_address.to_string(),
            "node-primary",
            "node-standby",
        );
        fresh_request.session_id = "process-loopback-session-2".to_string();
        primary.start(fresh_request).unwrap();
        primary.stop().unwrap();
    }

    #[test]
    fn failed_replacement_bind_leaves_the_process_fenced_and_stopped() {
        let active_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let active_address = active_probe.local_addr().unwrap();
        drop(active_probe);
        let state = ShowClockIpcState::default();
        state
            .start(request(
                ShowClockIpcRole::Primary,
                active_address.to_string(),
                "127.0.0.1:9".to_string(),
                "node-primary",
                "node-standby",
            ))
            .unwrap();

        let blocker = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut replacement = request(
            ShowClockIpcRole::Primary,
            blocker.local_addr().unwrap().to_string(),
            "127.0.0.1:9".to_string(),
            "node-primary",
            "node-standby",
        );
        replacement.session_id = "replacement-bind-failure-session".to_string();
        let error = state.start(replacement).unwrap_err();
        assert!(!error.is_empty());
        let status = state.status().unwrap();
        assert!(!status.running);
        assert_eq!(status.state, ShowClockIpcPhase::Stopped);
        assert!(!status.output_armed);
    }
}
