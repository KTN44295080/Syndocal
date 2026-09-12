//! Tauri-owned ShowClock process runtime.
//!
//! The protocol crate owns authentication, estimation, scheduling, and
//! fencing policy.  This module owns only the process lifecycle and the
//! manually configured LAN worker. Lighting output is armable only through
//! the existing local ownership permit and explicit ShowClock fence; video and
//! audio output remain outside this module.

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use engine::{EngineCommand, EngineHandle, OutputOwnershipPermit};
use io::show_clock_lan::{ShowClockLanError, ShowClockLanMessage, ShowClockLanTransport};
use protocol::{
    show_clock::{
        AuthenticatedShowClockAction, AuthenticatedShowClockSample, ShowClockAction,
        ShowClockActionAdmission, ShowClockActionKind, ShowClockActionPayload,
        ShowClockActionReceiver, ShowClockFenceState, ShowClockHash, ShowClockLatePolicy,
        ShowClockManualFence, ShowClockNodeId, ShowClockNonce, ShowClockPeerValidator,
        ShowClockReArm, ShowClockSample, ShowClockSessionId, ShowClockSource, ShowTransportState,
        SHOW_CLOCK_PROTOCOL_VERSION, SHOW_CLOCK_SCHEMA_VERSION,
    },
    show_clock_runtime::{
        ShowClockActionDispatch, ShowClockActionGeneration, ShowClockActionScheduler,
        ShowClockEstimatorState, ShowClockOutputContext, ShowClockOutputGate,
        ShowClockPeerEstimator,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ShowClockIpcFenceState {
    Disarmed,
    Hold,
    Armed,
}

impl From<ShowClockFenceState> for ShowClockIpcFenceState {
    fn from(state: ShowClockFenceState) -> Self {
        match state {
            ShowClockFenceState::Disarmed => Self::Disarmed,
            ShowClockFenceState::Hold => Self::Hold,
            ShowClockFenceState::Armed => Self::Armed,
        }
    }
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
    pub show_clock_gate_armed: bool,
    pub fence_state: ShowClockIpcFenceState,
    pub accepted_actions: u32,
    pub scheduled_actions: u32,
    pub last_action_sequence: u64,
    pub last_action_id: Option<String>,
    pub last_action_status: Option<String>,
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
            show_clock_gate_armed: false,
            fence_state: ShowClockIpcFenceState::Disarmed,
            accepted_actions: 0,
            scheduled_actions: 0,
            last_action_sequence: 0,
            last_action_id: None,
            last_action_status: None,
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
    #[serde(default = "default_generation")]
    pub project_generation: u64,
    #[serde(default = "default_generation")]
    pub lease_generation: u64,
    #[serde(default = "default_generation")]
    pub audio_generation: u64,
    #[serde(default = "default_generation")]
    pub recording_generation: u64,
    pub bpm_milli: u32,
    pub initial_show_time_us: u64,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockActionRequest {
    pub action_id_hex: String,
    pub sequence: u64,
    pub target_show_time_us: u64,
    pub action: ShowClockActionKind,
    pub late_policy: ShowClockLatePolicy,
    #[serde(default)]
    pub payload: Option<ShowClockActionPayload>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockArmOutputRequest {
    pub operator_confirmed: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockReArmRequest {
    pub operator_confirmed_primary_stopped: bool,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub project_generation: u64,
    pub lease_generation: u64,
    pub audio_generation: u64,
    pub recording_generation: u64,
    pub project_hash_hex: String,
    pub media_hash_hex: String,
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
    project_generation: u64,
    lease_generation: u64,
    audio_generation: u64,
    recording_generation: u64,
    bpm_milli: u32,
    initial_show_time_us: u64,
}

enum ShowClockWorkerCommand {
    ScheduleAction(ShowClockActionRequest, SyncSender<Result<(), String>>),
    EnterHold(SyncSender<Result<(), String>>),
    ReArm(ShowClockReArmRequest, SyncSender<Result<(), String>>),
    ArmOutput(
        ShowClockArmOutputRequest,
        OutputOwnershipPermit,
        SyncSender<Result<(), String>>,
    ),
}

struct ShowClockWorker {
    stop: Arc<AtomicBool>,
    commands: Sender<ShowClockWorkerCommand>,
    join: JoinHandle<()>,
}

pub struct ShowClockIpcState {
    engine: Option<EngineHandle>,
    status: Arc<Mutex<ShowClockIpcStatus>>,
    operation: Mutex<()>,
    lifecycle: Mutex<Option<ShowClockWorker>>,
    used_sessions: Mutex<HashSet<String>>,
}

impl Default for ShowClockIpcState {
    fn default() -> Self {
        Self {
            engine: None,
            status: Arc::new(Mutex::new(ShowClockIpcStatus::default())),
            operation: Mutex::new(()),
            lifecycle: Mutex::new(None),
            used_sessions: Mutex::new(HashSet::new()),
        }
    }
}

impl ShowClockIpcState {
    pub fn with_engine(engine: EngineHandle) -> Self {
        let mut state = Self::default();
        state.engine = Some(engine);
        state
    }

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
            status.show_clock_gate_armed = false;
            status.fence_state = ShowClockIpcFenceState::Disarmed;
            status.scheduled_actions = 0;
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
                show_clock_gate_armed: false,
                fence_state: ShowClockIpcFenceState::Disarmed,
                accepted_actions: 0,
                scheduled_actions: 0,
                last_action_sequence: 0,
                last_action_id: None,
                last_action_status: None,
                last_error: None,
            };
        }

        let stop = Arc::new(AtomicBool::new(false));
        let (commands_tx, commands_rx) = mpsc::channel();
        let worker_stop = Arc::clone(&stop);
        let worker_status = Arc::clone(&self.status);
        let worker_engine = self.engine.clone();
        let join = thread::Builder::new()
            .name(format!("syndocal-show-clock-{:?}", config.role))
            .spawn(move || {
                run_worker(
                    config,
                    transport,
                    worker_stop,
                    worker_status,
                    commands_rx,
                    worker_engine,
                )
            })
            .map_err(|error| format!("ShowClock worker could not start: {error}"))?;

        let mut lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| "ShowClock lifecycle lock was poisoned; restart Syndocal".to_string())?;
        *lifecycle = Some(ShowClockWorker {
            stop,
            commands: commands_tx,
            join,
        });
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
        status.show_clock_gate_armed = false;
        status.fence_state = ShowClockIpcFenceState::Disarmed;
        status.scheduled_actions = 0;
        Ok(status.clone())
    }

    pub fn schedule_action(
        &self,
        request: ShowClockActionRequest,
    ) -> Result<ShowClockIpcStatus, String> {
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        self.send_command(|reply| ShowClockWorkerCommand::ScheduleAction(request, reply))?;
        self.status()
    }

    pub fn enter_hold(&self) -> Result<ShowClockIpcStatus, String> {
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        self.send_command(|reply| ShowClockWorkerCommand::EnterHold(reply))?;
        self.status()
    }

    pub fn rearm(&self, request: ShowClockReArmRequest) -> Result<ShowClockIpcStatus, String> {
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        self.send_command(|reply| ShowClockWorkerCommand::ReArm(request, reply))?;
        self.status()
    }

    pub fn arm_output(
        &self,
        request: ShowClockArmOutputRequest,
    ) -> Result<ShowClockIpcStatus, String> {
        let _operation = self.operation.lock().map_err(|_| {
            "ShowClock lifecycle operation lock was poisoned; restart Syndocal".to_string()
        })?;
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| "ShowClock local output engine is unavailable".to_string())?;
        let permit = engine.acquire_lighting_output()?;
        self.send_command(|reply| ShowClockWorkerCommand::ArmOutput(request, permit, reply))?;
        self.status()
    }

    fn send_command(
        &self,
        command: impl FnOnce(SyncSender<Result<(), String>>) -> ShowClockWorkerCommand,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| "ShowClock lifecycle lock was poisoned; restart Syndocal".to_string())?;
        let worker = lifecycle
            .as_ref()
            .ok_or_else(|| "ShowClock is not running".to_string())?;
        worker
            .commands
            .send(command(reply_tx))
            .map_err(|_| "ShowClock worker is no longer accepting commands".to_string())?;
        drop(lifecycle);
        reply_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "ShowClock worker did not acknowledge the command".to_string())?
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
            project_generation: validate_nonzero_generation(request.project_generation, "project")?,
            lease_generation: validate_nonzero_generation(request.lease_generation, "lease")?,
            audio_generation: validate_nonzero_generation(request.audio_generation, "audio")?,
            recording_generation: validate_nonzero_generation(
                request.recording_generation,
                "recording",
            )?,
            bpm_milli: request.bpm_milli,
            initial_show_time_us: request.initial_show_time_us,
        })
    }
}

fn default_generation() -> u64 {
    1
}

fn validate_nonzero_generation(value: u64, label: &str) -> Result<u64, String> {
    if value == 0 {
        return Err(format!("ShowClock {label} generation must be non-zero"));
    }
    Ok(value)
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

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn run_worker(
    config: ShowClockConfig,
    transport: ShowClockLanTransport,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<ShowClockIpcStatus>>,
    commands: Receiver<ShowClockWorkerCommand>,
    engine: Option<EngineHandle>,
) {
    let result =
        ShowClockWorkerRuntime::new(&config, engine).and_then(|mut runtime| match config.role {
            ShowClockIpcRole::Primary => {
                run_primary(&config, &transport, &stop, &status, &commands, &mut runtime)
            }
            ShowClockIpcRole::Standby => {
                run_standby(&config, &transport, &stop, &status, &commands, &mut runtime)
            }
        });
    if let Err(error) = result {
        update_status(&status, |current| {
            current.running = false;
            current.state = ShowClockIpcPhase::Fault;
            current.output_armed = false;
            current.show_clock_gate_armed = false;
            current.fence_state = ShowClockIpcFenceState::Disarmed;
            current.last_error = Some(error);
        });
    } else if !stop.load(Ordering::Acquire) {
        update_status(&status, |current| {
            current.running = false;
            current.state = ShowClockIpcPhase::Stopped;
            current.output_armed = false;
            current.show_clock_gate_armed = false;
        });
    }
}

struct ShowClockWorkerRuntime {
    role: ShowClockIpcRole,
    owner: ShowClockNodeId,
    session_id: ShowClockSessionId,
    peer_node_id: ShowClockNodeId,
    key: [u8; 32],
    project_hash: ShowClockHash,
    media_hash: ShowClockHash,
    fence: ShowClockManualFence,
    scheduler: ShowClockActionScheduler,
    output_gate: ShowClockOutputGate,
    estimator: Option<ShowClockPeerEstimator>,
    validator: Option<ShowClockPeerValidator>,
    action_receiver: Option<ShowClockActionReceiver>,
    engine: Option<EngineHandle>,
    output_permit: Option<OutputOwnershipPermit>,
    held: bool,
}

impl ShowClockWorkerRuntime {
    fn new(config: &ShowClockConfig, engine: Option<EngineHandle>) -> Result<Self, String> {
        let context = ShowClockOutputContext::new(
            config.project_generation,
            config.lease_generation,
            config.audio_generation,
            config.recording_generation,
            config.clock_generation,
            config.fencing_generation,
            config.project_hash,
            config.media_hash,
        )
        .map_err(|error| format!("ShowClock output context initialization failed: {error}"))?;
        let fence = ShowClockManualFence::new(config.clock_generation, config.fencing_generation)
            .map_err(|error| {
            format!("ShowClock manual fence initialization failed: {error}")
        })?;
        let generation =
            ShowClockActionGeneration::new(config.clock_generation, config.fencing_generation)
                .map_err(|error| {
                    format!("ShowClock action generation initialization failed: {error}")
                })?;
        let scheduler = ShowClockActionScheduler::new(
            generation,
            protocol::show_clock_runtime::SHOW_CLOCK_DEFAULT_ACTION_HORIZON_US,
        )
        .map_err(|error| format!("ShowClock action scheduler initialization failed: {error}"))?;
        let output_gate = ShowClockOutputGate::new(config.node_id.clone(), context)
            .map_err(|error| format!("ShowClock output gate initialization failed: {error}"))?;
        let (estimator, validator, action_receiver) = match config.role {
            ShowClockIpcRole::Primary => (None, None, None),
            ShowClockIpcRole::Standby => {
                let validator = ShowClockPeerValidator::new(
                    config.session_id.clone(),
                    config.peer_node_id.clone(),
                    config.project_hash,
                    config.media_hash,
                    config.clock_generation,
                    config.fencing_generation,
                    config.key,
                )
                .map_err(|error| {
                    format!("ShowClock standby validator initialization failed: {error}")
                })?;
                let estimator = ShowClockPeerEstimator::new(
                    Default::default(),
                    config.clock_generation,
                    config.fencing_generation,
                )
                .map_err(|error| {
                    format!("ShowClock standby estimator initialization failed: {error}")
                })?;
                let action_receiver = ShowClockActionReceiver::new(
                    config.session_id.clone(),
                    config.peer_node_id.clone(),
                    config.project_hash,
                    config.media_hash,
                    config.clock_generation,
                    config.fencing_generation,
                    config.key,
                )
                .map_err(|error| {
                    format!("ShowClock action receiver initialization failed: {error}")
                })?;
                (Some(estimator), Some(validator), Some(action_receiver))
            }
        };
        Ok(Self {
            role: config.role,
            owner: config.node_id.clone(),
            session_id: config.session_id.clone(),
            peer_node_id: config.peer_node_id.clone(),
            key: config.key,
            project_hash: config.project_hash,
            media_hash: config.media_hash,
            fence,
            scheduler,
            output_gate,
            estimator,
            validator,
            action_receiver,
            engine,
            output_permit: None,
            held: false,
        })
    }

    fn enter_hold(&mut self, status: &Arc<Mutex<ShowClockIpcStatus>>) {
        self.fence.enter_hold();
        self.held = true;
        self.output_gate.disarm();
        self.output_permit.take();
        if let Some(estimator) = self.estimator.as_mut() {
            estimator.estimator_mut().enter_hold();
        }
        update_status(status, |current| {
            current.state = ShowClockIpcPhase::Hold;
            current.fence_state = self.fence.state().into();
            current.show_clock_gate_armed = false;
            current.output_armed = false;
            current.last_action_status = Some("manual_hold".to_string());
        });
    }

    fn rearm(
        &mut self,
        request: ShowClockReArmRequest,
        status: &Arc<Mutex<ShowClockIpcStatus>>,
    ) -> Result<(), String> {
        let mut fence = self.fence;
        fence
            .rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: request.operator_confirmed_primary_stopped,
                clock_generation: request.clock_generation,
                fencing_generation: request.fencing_generation,
            })
            .map_err(|error| format!("ShowClock Manual Re-arm rejected: {error}"))?;
        let project_hash = ShowClockHash(parse_fixed_hex::<32>(
            &request.project_hash_hex,
            "project hash",
        )?);
        let media_hash = ShowClockHash(parse_fixed_hex::<32>(
            &request.media_hash_hex,
            "media hash",
        )?);
        let context = ShowClockOutputContext::new(
            request.project_generation,
            request.lease_generation,
            request.audio_generation,
            request.recording_generation,
            request.clock_generation,
            request.fencing_generation,
            project_hash,
            media_hash,
        )
        .map_err(|error| format!("ShowClock re-arm output context rejected: {error}"))?;
        let mut estimator = self
            .estimator
            .clone()
            .ok_or_else(|| "ShowClock Manual Re-arm is available on Standby only".to_string())?;
        estimator
            .estimator_mut()
            .rearm_from_fence(fence)
            .map_err(|error| format!("ShowClock estimator re-arm rejected: {error}"))?;
        let mut scheduler = self.scheduler.clone();
        scheduler
            .rebind_to_armed_fence(fence)
            .map_err(|error| format!("ShowClock action generation re-arm rejected: {error}"))?;
        let mut output_gate = self.output_gate.clone();
        output_gate
            .replace_context(context)
            .map_err(|error| format!("ShowClock output context replacement rejected: {error}"))?;
        // Re-arm proves the new generation and clears old actions. The gate
        // remains disarmed until explicit local output ownership is acquired
        // through the engine dispatcher.
        output_gate.disarm();
        let validator = ShowClockPeerValidator::new(
            self.session_id.clone(),
            self.peer_node_id.clone(),
            project_hash,
            media_hash,
            request.clock_generation,
            request.fencing_generation,
            self.key,
        )
        .map_err(|error| format!("ShowClock re-arm validator rejected: {error}"))?;
        let action_receiver = ShowClockActionReceiver::new(
            self.session_id.clone(),
            self.peer_node_id.clone(),
            project_hash,
            media_hash,
            request.clock_generation,
            request.fencing_generation,
            self.key,
        )
        .map_err(|error| format!("ShowClock re-arm action receiver rejected: {error}"))?;

        self.project_hash = project_hash;
        self.media_hash = media_hash;
        self.fence = fence;
        self.scheduler = scheduler;
        self.output_gate = output_gate;
        self.output_permit.take();
        self.estimator = Some(estimator);
        self.validator = Some(validator);
        self.action_receiver = Some(action_receiver);
        self.held = false;
        update_status(status, |current| {
            current.state = ShowClockIpcPhase::Acquiring;
            current.clock_generation = request.clock_generation;
            current.fencing_generation = request.fencing_generation;
            current.accepted_samples = 0;
            current.last_sequence = 0;
            current.offset_us = 0;
            current.sample_age_us = None;
            current.fence_state = self.fence.state().into();
            current.show_clock_gate_armed = false;
            current.output_armed = false;
            current.scheduled_actions = 0;
            current.last_action_status = Some("manual_rearm_waiting_for_lock".to_string());
            current.last_error = None;
        });
        Ok(())
    }

    fn arm_output(
        &mut self,
        request: ShowClockArmOutputRequest,
        permit: OutputOwnershipPermit,
        status: &Arc<Mutex<ShowClockIpcStatus>>,
    ) -> Result<(), String> {
        if !request.operator_confirmed {
            return Err("ShowClock output Arm requires explicit operator confirmation".to_string());
        }
        if self.output_gate.is_armed() {
            return Err("ShowClock output is already armed".to_string());
        }
        if self.role == ShowClockIpcRole::Standby
            && self.estimator.as_ref().is_none_or(|estimator| {
                estimator.estimator().state() != ShowClockEstimatorState::Locked
            })
        {
            return Err("ShowClock Standby output Arm requires LOCKED peer state".to_string());
        }

        let mut fence = self.fence;
        if self.role == ShowClockIpcRole::Primary {
            fence
                .arm_initial(true)
                .map_err(|error| format!("ShowClock Primary output Arm rejected: {error}"))?;
        } else if fence.state() != ShowClockFenceState::Armed {
            return Err("ShowClock Standby output Arm requires Manual Re-arm first".to_string());
        }
        let mut output_gate = self.output_gate.clone();
        output_gate
            .arm(fence, output_gate.context())
            .map_err(|error| format!("ShowClock output Arm rejected: {error}"))?;
        self.fence = fence;
        self.output_gate = output_gate;
        self.output_permit = Some(permit);
        update_status(status, |current| {
            current.fence_state = self.fence.state().into();
            current.show_clock_gate_armed = self.output_gate.is_armed();
            current.output_armed = self.output_permit.is_some();
            current.last_action_status = Some("local_output_armed".to_string());
            current.last_error = None;
        });
        Ok(())
    }
}

fn run_primary(
    config: &ShowClockConfig,
    transport: &ShowClockLanTransport,
    stop: &AtomicBool,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    commands: &Receiver<ShowClockWorkerCommand>,
    runtime: &mut ShowClockWorkerRuntime,
) -> Result<(), String> {
    let origin = Instant::now();
    let mut sequence = 1_u64;
    let mut next_send = origin;
    while !stop.load(Ordering::Acquire) {
        drain_commands(config, transport, status, commands, runtime, origin)?;
        if runtime.held {
            thread::sleep(Duration::from_millis(10));
            continue;
        }
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
            pump_actions(
                runtime,
                status,
                show_time_us,
                ShowClockEstimatorState::Locked,
            )?;
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
    commands: &Receiver<ShowClockWorkerCommand>,
    runtime: &mut ShowClockWorkerRuntime,
) -> Result<(), String> {
    let origin = Instant::now();
    while !stop.load(Ordering::Acquire) {
        drain_commands(config, transport, status, commands, runtime, origin)?;
        let received = match transport.receive(RECEIVE_POLL_INTERVAL) {
            Ok(message) => message,
            Err(ShowClockLanError::Timeout) => {
                let estimate = runtime
                    .estimator
                    .as_mut()
                    .expect("standby estimator exists")
                    .estimator_mut()
                    .advance(monotonic_elapsed_us(origin, Instant::now()));
                publish_estimate(
                    status,
                    &estimate,
                    runtime
                        .validator
                        .as_ref()
                        .expect("standby validator exists")
                        .last_sequence(),
                );
                pump_actions(runtime, status, estimate.show_time_us, estimate.state)?;
                continue;
            }
            Err(error) => return Err(format!("ShowClock standby receive failed: {error}")),
        };
        match received {
            ShowClockLanMessage::Sample(sample) => {
                let received_at_us = monotonic_elapsed_us(origin, Instant::now());
                runtime
                    .estimator
                    .as_mut()
                    .expect("standby estimator exists")
                    .accept_authenticated_sample(
                        runtime
                            .validator
                            .as_mut()
                            .expect("standby validator exists"),
                        &sample,
                        received_at_us,
                    )
                    .map_err(|error| format!("ShowClock standby sample rejected: {error}"))?;
                let estimate = runtime
                    .estimator
                    .as_mut()
                    .expect("standby estimator exists")
                    .estimator_mut()
                    .advance(received_at_us);
                publish_estimate(
                    status,
                    &estimate,
                    runtime
                        .validator
                        .as_ref()
                        .expect("standby validator exists")
                        .last_sequence(),
                );
                pump_actions(runtime, status, estimate.show_time_us, estimate.state)?;
            }
            ShowClockLanMessage::Action(action) => {
                let admission = runtime
                    .action_receiver
                    .as_mut()
                    .expect("standby action receiver exists")
                    .admit(&action)
                    .map_err(|error| format!("ShowClock standby action rejected: {error}"))?;
                let action_id = hex_string(&action.body.action_id);
                publish_action_admission(
                    status,
                    action.body.sequence,
                    action_id,
                    &admission,
                    runtime
                        .action_receiver
                        .as_ref()
                        .expect("standby action receiver exists")
                        .accepted_action_count() as u32,
                );
                if admission == ShowClockActionAdmission::Accepted {
                    let received_at_us = monotonic_elapsed_us(origin, Instant::now());
                    let estimate = runtime
                        .estimator
                        .as_mut()
                        .expect("standby estimator exists")
                        .estimator_mut()
                        .advance(received_at_us);
                    runtime
                        .scheduler
                        .schedule(action.body, estimate.show_time_us)
                        .map_err(|error| {
                            format!("ShowClock action scheduling rejected: {error}")
                        })?;
                    update_status(status, |current| {
                        current.scheduled_actions = runtime.scheduler.len() as u32;
                        current.last_action_status =
                            Some("authenticated_and_scheduled".to_string());
                    });
                    pump_actions(runtime, status, estimate.show_time_us, estimate.state)?;
                }
            }
        }
    }
    Ok(())
}

fn drain_commands(
    config: &ShowClockConfig,
    transport: &ShowClockLanTransport,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    commands: &Receiver<ShowClockWorkerCommand>,
    runtime: &mut ShowClockWorkerRuntime,
    origin: Instant,
) -> Result<(), String> {
    while let Ok(command) = commands.try_recv() {
        match command {
            ShowClockWorkerCommand::ScheduleAction(request, reply) => {
                let result = if config.role == ShowClockIpcRole::Primary {
                    schedule_primary_action(config, transport, runtime, status, origin, request)
                } else {
                    Err("ShowClock actions must be scheduled by Primary".to_string())
                };
                let _ = reply.send(result);
            }
            ShowClockWorkerCommand::EnterHold(reply) => {
                runtime.enter_hold(status);
                let _ = reply.send(Ok(()));
            }
            ShowClockWorkerCommand::ReArm(request, reply) => {
                let result = if config.role == ShowClockIpcRole::Standby {
                    runtime.rearm(request, status)
                } else {
                    Err("ShowClock Manual Re-arm is available on Standby only".to_string())
                };
                let _ = reply.send(result);
            }
            ShowClockWorkerCommand::ArmOutput(request, permit, reply) => {
                let result = runtime.arm_output(request, permit, status);
                let _ = reply.send(result);
            }
        }
    }
    Ok(())
}

fn schedule_primary_action(
    config: &ShowClockConfig,
    transport: &ShowClockLanTransport,
    runtime: &mut ShowClockWorkerRuntime,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    origin: Instant,
    request: ShowClockActionRequest,
) -> Result<(), String> {
    if runtime.held {
        return Err("ShowClock primary is in Manual Hold".to_string());
    }
    let action = ShowClockAction {
        schema_version: SHOW_CLOCK_SCHEMA_VERSION,
        protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
        session_id: config.session_id.clone(),
        sender: config.node_id.clone(),
        action_id: parse_fixed_hex::<16>(&request.action_id_hex, "action id")?,
        sequence: request.sequence,
        clock_generation: config.clock_generation,
        fencing_generation: config.fencing_generation,
        target_show_time_us: request.target_show_time_us,
        action: request.action,
        late_policy: request.late_policy,
        payload: request.payload,
        project_hash: config.project_hash,
        media_hash: config.media_hash,
    };
    let now = config
        .initial_show_time_us
        .saturating_add(monotonic_elapsed_us(origin, Instant::now()))
        .max(1);
    runtime
        .scheduler
        .schedule(action.clone(), now)
        .map_err(|error| format!("ShowClock action scheduling rejected: {error}"))?;
    let signed = AuthenticatedShowClockAction::sign(action.clone(), &config.key)
        .map_err(|error| format!("ShowClock action signing failed: {error}"))?;
    if let Err(error) = transport.send_action(&signed) {
        update_status(status, |current| {
            current.scheduled_actions = runtime.scheduler.len() as u32;
            current.last_action_sequence = action.sequence;
            current.last_action_id = Some(hex_string(&action.action_id));
            current.last_action_status = Some("send_failed_action_retained_locally".to_string());
            current.last_error = Some(error.to_string());
        });
        return Err(format!("ShowClock primary action send failed: {error}"));
    }
    update_status(status, |current| {
        current.scheduled_actions = runtime.scheduler.len() as u32;
        current.last_action_sequence = action.sequence;
        current.last_action_id = Some(hex_string(&action.action_id));
        current.last_action_status = Some("sent_and_scheduled".to_string());
        current.last_error = None;
    });
    Ok(())
}

fn pump_actions(
    runtime: &mut ShowClockWorkerRuntime,
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    current_show_time_us: u64,
    state: ShowClockEstimatorState,
) -> Result<(), String> {
    if matches!(state, ShowClockEstimatorState::Stale | ShowClockEstimatorState::Fault)
        && (runtime.output_gate.is_armed() || runtime.output_permit.is_some())
    {
        runtime.output_gate.disarm();
        runtime.output_permit.take();
        update_status(status, |current| {
            current.show_clock_gate_armed = false;
            current.output_armed = false;
            current.last_action_status = Some("auto_disarmed_unsafe_clock_state".to_string());
        });
    }
    if runtime.scheduler.is_empty() {
        return Ok(());
    }
    let dispatch = if state == ShowClockEstimatorState::Locked {
        match runtime.scheduler.poll_authorized(
            current_show_time_us,
            state,
            &runtime.output_gate,
            &runtime.owner,
            runtime.output_gate.context(),
        ) {
            Ok(dispatch) => dispatch,
            Err(protocol::show_clock::ShowClockValidationError::OutputNotArmed) => {
                update_status(status, |current| {
                    current.scheduled_actions = runtime.scheduler.len() as u32;
                    current.show_clock_gate_armed = runtime.output_gate.is_armed();
                    current.last_action_status = Some("blocked_output_gate_disarmed".to_string());
                });
                return Ok(());
            }
            Err(error) => return Err(format!("ShowClock output authorization failed: {error}")),
        }
    } else {
        runtime.scheduler.poll(current_show_time_us, state)
    };
    update_status(status, |current| {
        current.scheduled_actions = runtime.scheduler.len() as u32;
        if dispatch.is_some() {
            current.last_action_status = Some(
                match &dispatch {
                    Some(ShowClockActionDispatch::Held { .. }) => "held_until_locked",
                    Some(ShowClockActionDispatch::Dropped { .. }) => "dropped_late_action",
                    Some(ShowClockActionDispatch::Execute(_)) => "execute_authorized",
                    None => "no_dispatch",
                }
                .to_string(),
            );
        }
    });
    if let Some(ShowClockActionDispatch::Execute(action)) = dispatch {
        let engine = runtime
            .engine
            .as_ref()
            .ok_or_else(|| "ShowClock output dispatcher engine is unavailable".to_string())?;
        dispatch_show_clock_action(engine, &action)?;
        update_status(status, |current| {
            current.last_action_status = Some("executed_by_local_output_dispatcher".to_string());
        });
    }
    Ok(())
}

fn dispatch_show_clock_action(
    engine: &EngineHandle,
    action: &ShowClockAction,
) -> Result<(), String> {
    // Lighting is held for the lifetime of an armed ShowClock worker. Video
    // actions acquire the matching capability for the complete synchronous
    // EngineHandle operation, so a role transition cannot overlap a Take or
    // Clip Launch admission.
    let _video_output_permit = matches!(
        action.action,
        ShowClockActionKind::Take
            | ShowClockActionKind::ClipLaunch
            | ShowClockActionKind::Transition
    )
    .then(|| engine.acquire_video_output())
    .transpose()?;
    match (action.action, action.payload.as_ref()) {
        (ShowClockActionKind::Go, None) => engine
            .send(EngineCommand::SetTimelinePlaying(true))
            .map_err(|error| error.to_string()),
        (ShowClockActionKind::Stop, None) => engine
            .send(EngineCommand::SetTimelinePlaying(false))
            .map_err(|error| error.to_string()),
        (ShowClockActionKind::Back, None) => engine
            .send(EngineCommand::SeekTimeline(0))
            .map_err(|error| error.to_string()),
        (ShowClockActionKind::Blackout, None) => engine
            .send(EngineCommand::SetAllBlackout(true))
            .map_err(|error| error.to_string()),
        (ShowClockActionKind::Release, Some(ShowClockActionPayload::CueRelease { cue_id })) => {
            engine
                .send(EngineCommand::ReleaseCue(*cue_id))
                .map_err(|error| error.to_string())
        }
        (
            ShowClockActionKind::Take,
            Some(ShowClockActionPayload::VideoTake {
                target_layer_id,
                fade_ms,
                preview_position_ms,
                preview_speed_milli,
            }),
        ) => engine.exclusive_video_take(protocol::ExclusiveVideoTakeRequest {
            target_layer_id: *target_layer_id,
            fade_ms: *fade_ms,
            preview_position_ms: *preview_position_ms,
            preview_speed: preview_speed_milli.map(|speed| speed as f32 / 1000.0),
        }),
        (
            ShowClockActionKind::ClipLaunch | ShowClockActionKind::Transition,
            Some(ShowClockActionPayload::ClipLaunch {
                layer_id,
                slot_id,
                transition_kind,
                transition_duration_ms,
            }),
        ) => engine.launch_video_clip_slot_with_transition_published(
            *layer_id,
            *slot_id,
            *transition_kind,
            *transition_duration_ms,
        ),
        (
            ShowClockActionKind::TimelineJump,
            Some(ShowClockActionPayload::TimelineJump { position_ms }),
        ) => engine
            .send(EngineCommand::SeekTimeline(*position_ms))
            .map_err(|error| error.to_string()),
        _ => Err("ShowClock action payload did not match its dispatch kind".to_string()),
    }
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

fn publish_action_admission(
    status: &Arc<Mutex<ShowClockIpcStatus>>,
    sequence: u64,
    action_id: String,
    admission: &ShowClockActionAdmission,
    accepted_actions: u32,
) {
    update_status(status, |current| {
        // Duplicate packets can arrive after a later accepted action. Keep the
        // status sequence as a recovery floor for a remounted UI instead of
        // allowing an old retransmission to move it backwards.
        current.last_action_sequence = current.last_action_sequence.max(sequence);
        current.last_action_id = Some(action_id);
        current.last_action_status = Some(
            match admission {
                ShowClockActionAdmission::Accepted => "authenticated",
                ShowClockActionAdmission::Duplicate => "duplicate",
            }
            .to_string(),
        );
        current.accepted_actions = accepted_actions;
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
    use protocol::DmxOutputConfig;
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
            project_generation: 1,
            lease_generation: 1,
            audio_generation: 1,
            recording_generation: 1,
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

    #[test]
    fn authenticated_action_is_scheduled_and_manual_rearm_clears_old_generation() {
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
        primary
            .schedule_action(ShowClockActionRequest {
                action_id_hex: "01000000000000000000000000000000".to_string(),
                sequence: 1,
                target_show_time_us: 1_500_000,
                action: ShowClockActionKind::Go,
                late_policy: ShowClockLatePolicy::Hold,
                payload: None,
            })
            .unwrap();
        for _ in 0..20 {
            if standby.status().unwrap().accepted_actions >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let action_status = standby.status().unwrap();
        assert_eq!(action_status.accepted_actions, 1);
        assert_eq!(action_status.scheduled_actions, 1);
        assert_eq!(
            action_status.last_action_status.as_deref(),
            Some("blocked_output_gate_disarmed")
        );

        primary.stop().unwrap();
        standby.enter_hold().unwrap();
        let held = standby.status().unwrap();
        assert_eq!(held.state, ShowClockIpcPhase::Hold);
        assert_eq!(held.fence_state, ShowClockIpcFenceState::Hold);
        standby
            .rearm(ShowClockReArmRequest {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 2,
                project_generation: 1,
                lease_generation: 1,
                audio_generation: 1,
                recording_generation: 1,
                project_hash_hex: HASH.to_string(),
                media_hash_hex: MEDIA.to_string(),
            })
            .unwrap();
        let rearmed = standby.status().unwrap();
        assert_eq!(rearmed.state, ShowClockIpcPhase::Acquiring);
        assert_eq!(rearmed.fence_state, ShowClockIpcFenceState::Armed);
        assert_eq!(rearmed.fencing_generation, 2);
        assert_eq!(rearmed.scheduled_actions, 0);
        assert!(!rearmed.show_clock_gate_armed);
        standby.stop().unwrap();
    }

    #[test]
    fn primary_arm_requires_confirmation_and_dispatches_through_local_engine() {
        let primary_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let primary_address = primary_probe.local_addr().unwrap();
        drop(primary_probe);
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let state = ShowClockIpcState::with_engine(engine.clone());
        state
            .start(request(
                ShowClockIpcRole::Primary,
                primary_address.to_string(),
                "127.0.0.1:9".to_string(),
                "node-primary",
                "node-standby",
            ))
            .unwrap();

        let confirmation_error = state
            .arm_output(ShowClockArmOutputRequest {
                operator_confirmed: false,
            })
            .unwrap_err();
        assert!(confirmation_error.contains("explicit operator confirmation"));

        let armed = state
            .arm_output(ShowClockArmOutputRequest {
                operator_confirmed: true,
            })
            .unwrap();
        assert!(armed.output_armed);
        assert!(armed.show_clock_gate_armed);
        assert_eq!(armed.fence_state, ShowClockIpcFenceState::Armed);
        let initial_transport = engine.timeline_transport_authority();

        state
            .schedule_action(ShowClockActionRequest {
                action_id_hex: "03000000000000000000000000000000".to_string(),
                sequence: 1,
                target_show_time_us: 1_000_001,
                action: ShowClockActionKind::Go,
                late_policy: ShowClockLatePolicy::ExecuteImmediately,
                payload: None,
            })
            .unwrap();
        for _ in 0..30 {
            if state.status().unwrap().last_action_status.as_deref()
                == Some("executed_by_local_output_dispatcher")
            {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let dispatched = state.status().unwrap();
        assert_eq!(
            dispatched.last_action_status.as_deref(),
            Some("executed_by_local_output_dispatcher")
        );
        for _ in 0..20 {
            if engine.timeline_transport_authority() != initial_transport {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        assert_ne!(engine.timeline_transport_authority(), initial_transport);

        state.enter_hold().unwrap();
        let held = state.status().unwrap();
        assert_eq!(held.state, ShowClockIpcPhase::Hold);
        assert!(!held.output_armed);
        assert!(!held.show_clock_gate_armed);
        state.stop().unwrap();
    }

    #[test]
    fn stale_clock_state_revokes_local_output_permit_before_queue_poll() {
        let primary_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let primary_address = primary_probe.local_addr().unwrap();
        drop(primary_probe);
        let config = ShowClockConfig::try_from(request(
            ShowClockIpcRole::Primary,
            primary_address.to_string(),
            "127.0.0.1:9".to_string(),
            "node-primary",
            "node-standby",
        ))
        .unwrap();
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let mut runtime = ShowClockWorkerRuntime::new(&config, Some(engine.clone())).unwrap();
        let permit = engine.acquire_lighting_output().unwrap();
        let status = Arc::new(Mutex::new(ShowClockIpcStatus::default()));
        runtime
            .arm_output(
                ShowClockArmOutputRequest {
                    operator_confirmed: true,
                },
                permit,
                &status,
            )
            .unwrap();
        assert!(runtime.output_gate.is_armed());
        assert!(runtime.output_permit.is_some());

        pump_actions(
            &mut runtime,
            &status,
            1_000_000,
            ShowClockEstimatorState::Stale,
        )
        .unwrap();
        assert!(!runtime.output_gate.is_armed());
        assert!(runtime.output_permit.is_none());
        let status = status.lock().unwrap().clone();
        assert!(!status.output_armed);
        assert_eq!(
            status.last_action_status.as_deref(),
            Some("auto_disarmed_unsafe_clock_state")
        );
    }

    #[test]
    fn duplicate_action_status_cannot_lower_sequence_recovery_floor() {
        let status = Arc::new(Mutex::new(ShowClockIpcStatus {
            last_action_sequence: 5,
            ..ShowClockIpcStatus::default()
        }));
        publish_action_admission(
            &status,
            2,
            "02000000000000000000000000000000".to_string(),
            &ShowClockActionAdmission::Duplicate,
            5,
        );
        let status = status.lock().unwrap().clone();
        assert_eq!(status.last_action_sequence, 5);
        assert_eq!(status.last_action_status.as_deref(), Some("duplicate"));
        assert_eq!(status.accepted_actions, 5);
    }

    #[test]
    fn video_show_clock_actions_recheck_local_video_ownership() {
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine
            .set_output_ownership_role(protocol::MachineOutputRole::Standby)
            .unwrap();
        let action = ShowClockAction {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: ShowClockSessionId::new("video-ownership-test").unwrap(),
            sender: ShowClockNodeId::new("node-primary").unwrap(),
            action_id: [4; 16],
            sequence: 1,
            clock_generation: 1,
            fencing_generation: 1,
            target_show_time_us: 1,
            action: ShowClockActionKind::Take,
            late_policy: ShowClockLatePolicy::ExecuteImmediately,
            payload: Some(ShowClockActionPayload::VideoTake {
                target_layer_id: 1,
                fade_ms: 0,
                preview_position_ms: None,
                preview_speed_milli: None,
            }),
            project_hash: ShowClockHash([1; 32]),
            media_hash: ShowClockHash([2; 32]),
        };
        let error = dispatch_show_clock_action(&engine, &action).unwrap_err();
        assert!(error.contains("Video output is blocked"));
    }
}
