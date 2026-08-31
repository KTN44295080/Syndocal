use std::{
    hint::spin_loop,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc, Arc, Mutex, TryLockError,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossbeam_queue::ArrayQueue;
use protocol::{DmxOutputProtocol, SerialPortSummary};
use serialport::{DataBits, Parity, SerialPort, SerialPortType, StopBits};
use thiserror::Error;

pub const ENTTEC_OPEN_DMX_BAUD_RATE: u32 = 250_000;
pub const ENTTEC_OPEN_DMX_START_CODE: u8 = 0x00;
pub const ENTTEC_OPEN_DMX_PAYLOAD_LEN: usize = 513;
pub const ENTTEC_OPEN_DMX_BREAK_US: u64 = 176;
pub const ENTTEC_OPEN_DMX_MAB_US: u64 = 16;
// 1 start + 8 data + 2 stop bits per byte at 250 kbaud.
pub const ENTTEC_OPEN_DMX_BYTE_WIRE_US: u64 = 44;
pub const ENTTEC_OPEN_DMX_FRAME_WIRE_US: u64 = ENTTEC_OPEN_DMX_BREAK_US
    + ENTTEC_OPEN_DMX_MAB_US
    + ENTTEC_OPEN_DMX_PAYLOAD_LEN as u64 * ENTTEC_OPEN_DMX_BYTE_WIRE_US;
// FTDI VCP write/flush returns before the chip's internal TX buffer has drained
// onto the wire; asserting the next break early slices the in-flight frame.
//
// This 8ms guard is the accepted default for the exact FT232R/COM3 rig in
// qa/M4_IO_VALIDATION.md: its 2ms guard showed periodic dropouts, while the
// 22,764us wire time plus 8ms was stable. It yields about 32.5fps. This is not
// a DMX512 or Open DMX universal maximum; a faster setting needs fresh
// waveform and fixture evidence for the target adapter.
pub const ENTTEC_OPEN_DMX_FRAME_GUARD_US: u64 = 8_000;
pub const ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US: u64 =
    ENTTEC_OPEN_DMX_FRAME_WIRE_US + ENTTEC_OPEN_DMX_FRAME_GUARD_US;
pub const ENTTEC_PRO_START_DELIMITER: u8 = 0x7e;
pub const ENTTEC_PRO_END_DELIMITER: u8 = 0xe7;
pub const ENTTEC_PRO_SEND_DMX_LABEL: u8 = 0x06;
pub const ENTTEC_PRO_DMX_START_CODE: u8 = 0x00;
pub const ENTTEC_PRO_DMX_PAYLOAD_LEN: usize = 513;
pub const ENTTEC_PRO_SEND_PACKET_LEN: usize = 4 + ENTTEC_PRO_DMX_PAYLOAD_LEN + 1;

#[derive(Debug, Error)]
pub enum SerialDmxError {
    #[error("failed to list serial ports: {0}")]
    List(#[source] serialport::Error),
    #[error("serial port path is required")]
    MissingPort,
    #[error("failed to open serial port {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: serialport::Error,
    },
    #[error("serial port identity verification failed: {0}")]
    Identity(String),
    #[error("failed to write serial DMX packet: {0}")]
    Write(#[source] io::Error),
    #[error("failed to start Open DMX worker: {0}")]
    WorkerStart(#[source] io::Error),
    #[error("Open DMX worker stopped: {0}")]
    Worker(String),
}

/// Immutable USB identity captured from the serial enumeration immediately
/// before a show-critical serial route is opened. A COM name alone is not a
/// hardware identity: Windows can reassign it after an unplug/replug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedUsbSerialPortIdentity {
    pub port_name: String,
    pub port_type: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub serial_number: String,
    pub manufacturer: String,
    pub product: String,
    /// The Windows PnP devnode that owned the exact COM device-interface when
    /// this identity was captured. It is deliberately distinct from the COM
    /// alias: Windows may reuse COM3 for a different physical interface.
    pub windows_device_instance_id: Option<String>,
}

impl VerifiedUsbSerialPortIdentity {
    /// Fail closed if the enumerator cannot establish a nonempty hardware
    /// instance identity. Callers may additionally constrain VID/PID and
    /// product fields for a particular authored route.
    pub fn from_summary(summary: &SerialPortSummary) -> Result<Self, SerialDmxError> {
        let serial_number = summary
            .serial_number
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                SerialDmxError::Identity(
                    "the serial device did not expose a nonempty hardware serial number"
                        .to_string(),
                )
            })?;
        let usb_vid = summary.usb_vid.ok_or_else(|| {
            SerialDmxError::Identity("the serial device did not expose a USB VID".to_string())
        })?;
        let usb_pid = summary.usb_pid.ok_or_else(|| {
            SerialDmxError::Identity("the serial device did not expose a USB PID".to_string())
        })?;
        if usb_vid == 0 || usb_pid == 0 {
            return Err(SerialDmxError::Identity(
                "the serial device did not expose nonzero USB VID and PID".to_string(),
            ));
        }
        let manufacturer = summary
            .manufacturer
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                SerialDmxError::Identity(
                    "the serial device did not expose a manufacturer identity".to_string(),
                )
            })?;
        let product = summary
            .product
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                SerialDmxError::Identity(
                    "the serial device did not expose a product identity".to_string(),
                )
            })?;
        if summary.name.trim().is_empty() || summary.port_type.trim().is_empty() {
            return Err(SerialDmxError::Identity(
                "the serial device did not expose an exact COM-path identity".to_string(),
            ));
        }
        Ok(Self {
            port_name: summary.name.clone(),
            port_type: summary.port_type.clone(),
            usb_vid,
            usb_pid,
            serial_number: serial_number.to_string(),
            manufacturer: manufacturer.to_string(),
            product: product.to_string(),
            windows_device_instance_id: None,
        })
    }

    /// Capture the real Windows PnP instance behind the COM alias.  The
    /// resulting identity is required by `new_verified` on Windows so the
    /// opened handle is selected through its physical device-interface path,
    /// never through `SerialPort::name()`.
    #[cfg(target_os = "windows")]
    pub fn from_summary_with_windows_com_binding(
        summary: &SerialPortSummary,
    ) -> Result<Self, SerialDmxError> {
        let mut identity = Self::from_summary(summary)?;
        let binding = resolve_windows_com_port_binding(&identity)?;
        let enumerated_instance = summary
            .windows_device_instance_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                SerialDmxError::Identity(
                    "the serial enumerator did not expose the current Windows PnP device instance"
                        .to_string(),
                )
            })?;
        if binding.device_instance_id != enumerated_instance {
            return Err(SerialDmxError::Identity(format!(
                "the serial enumerator and Windows COM binding disagree on the current PnP instance (enumerated {enumerated_instance}, observed {})",
                binding.device_instance_id,
            )));
        }
        identity.windows_device_instance_id = Some(binding.device_instance_id);
        identity.validate_for_verified_open()?;
        Ok(identity)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn from_summary_with_windows_com_binding(
        _summary: &SerialPortSummary,
    ) -> Result<Self, SerialDmxError> {
        Err(SerialDmxError::Identity(
            "the approved show serial route requires Windows PnP COM identity binding".to_string(),
        ))
    }

    pub fn matches_summary(&self, summary: &SerialPortSummary) -> bool {
        self.validate_for_verified_open().is_ok()
            && summary.name == self.port_name
            && summary.port_type == self.port_type
            && summary.usb_vid == Some(self.usb_vid)
            && summary.usb_pid == Some(self.usb_pid)
            && summary.serial_number.as_deref() == Some(self.serial_number.as_str())
            && summary.manufacturer.as_deref() == Some(self.manufacturer.as_str())
            && summary.product.as_deref() == Some(self.product.as_str())
            && summary.windows_device_instance_id.as_deref()
                == self.windows_device_instance_id.as_deref()
    }

    fn validate_for_verified_open(&self) -> Result<(), SerialDmxError> {
        let required = [
            ("COM path", self.port_name.as_str()),
            ("serial port type", self.port_type.as_str()),
            ("hardware serial number", self.serial_number.as_str()),
            ("manufacturer", self.manufacturer.as_str()),
            ("product", self.product.as_str()),
        ];
        for (label, value) in required {
            if value.trim().is_empty() {
                return Err(SerialDmxError::Identity(format!(
                    "the verified show serial identity is missing its {label}"
                )));
            }
        }
        if self.usb_vid == 0 || self.usb_pid == 0 {
            return Err(SerialDmxError::Identity(
                "the verified show serial identity requires nonzero USB VID and PID".to_string(),
            ));
        }
        if self
            .windows_device_instance_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(SerialDmxError::Identity(
                "the verified show serial identity did not capture a Windows PnP device instance"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// The exact Windows device interface used to create an opened COM handle.
/// Its path is allocated by PnP for one present devnode; unlike `COM3`, it is
/// not a user-facing alias that can be reassigned after a USB swap.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsComPortBinding {
    port_name: String,
    device_instance_id: String,
    device_interface_path: String,
}

pub struct EnttecUsbProSender {
    port: Box<dyn SerialPort>,
}

pub struct EnttecOpenDmxSender {
    frames: Arc<ArrayQueue<[u8; 512]>>,
    running: Arc<AtomicBool>,
    worker_failed: Arc<AtomicBool>,
    worker_error: Arc<Mutex<Option<String>>>,
    zero_write_generation: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
    worker_done: Option<mpsc::Receiver<()>>,
}

/// Evidence that is strictly narrower than fixture verification: the caller
/// observed the physical Open-DMX worker complete a later all-zero
/// BREAK/MAB/write_all/flush transaction. It contains no device-level or
/// fixture-level acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenDmxZeroWriteReceipt {
    completed_generation_before_queue: u64,
}

/// The physical linearization boundary for Open DMX output.
///
/// The worker holds this short mutex from deciding whether a queued frame is
/// live or zero through BREAK, MAB, `write_all`, and `flush`. The atomic S0
/// latch is the single frame-selection truth: it is set before a caller ever
/// waits on this mutex, so a wedged driver cannot stall the engine's safer
/// direction. A worker that wins immediately before the latch may finish one
/// live wire write; every later selection is all-zero. This is runtime
/// authority only; callers must never persist it in an `.sdc` project.
#[derive(Clone, Debug)]
pub struct OpenDmxSafetyWriteGate {
    state: Arc<Mutex<()>>,
    /// A nonblocking S0 latch set before any potentially wedged physical
    /// transaction lock. Once true, every future worker selection is zero
    /// even if the mutex holder cannot be joined or interrupted.
    blackout_latched: Arc<AtomicBool>,
    /// A reservation must be releasable after a worker wedges while it owns
    /// the physical mutex. Keep the count independent of that mutex so Drop
    /// can always decrement it without making a later successful re-arm
    /// silently permanent-zero.
    zero_write_holds: Arc<AtomicU32>,
}

impl Default for OpenDmxSafetyWriteGate {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenDmxSafetyWriteGate {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(())),
            blackout_latched: Arc::new(AtomicBool::new(false)),
            zero_write_holds: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Atomically reserve the safer physical direction before the priority S0
    /// command becomes observable.  The caller may hold other engine gates;
    /// this method itself contains no serial I/O.
    pub fn engage_blackout(&self) -> Result<(), String> {
        self.latch_blackout();
        self.set_blackout_engaged(true)
    }

    /// Latch S0 before attempting a bounded physical-gate confirmation. A
    /// timeout means the holder may still be in a driver call: it is never a
    /// zero-write receipt, but future frame selection remains zero-only.
    pub fn engage_blackout_bounded(&self, timeout: Duration) -> Result<(), String> {
        self.latch_blackout();
        // This is a bounded observation that no physical transaction owns
        // the gate. It is deliberately not required for the S0 latch itself:
        // a timeout is not a zero receipt, but the latch remains effective.
        let _state = self.lock_bounded(timeout)?;
        Ok(())
    }

    /// Immediate fail-closed direction latch for a fault path. This makes no
    /// claim that an in-progress physical write was interrupted or that a
    /// zero frame reached the fixture.
    pub fn latch_blackout(&self) {
        self.blackout_latched.store(true, Ordering::Release);
    }

    /// Acquire the physical-write authority.  The returned guard is kept
    /// across the complete Open DMX wire transaction, not merely frame
    /// selection, so an S0 reservation and an actual `write_all` have one
    /// deterministic order.
    pub fn lock(&self) -> Result<OpenDmxSafetyWriteGateGuard<'_>, String> {
        self.state
            .lock()
            .map(|state| OpenDmxSafetyWriteGateGuard {
                _state: state,
                blackout_latched: &self.blackout_latched,
            })
            .map_err(|_| "Open DMX physical-write gate was poisoned".to_string())
    }

    /// Wait only until `timeout` for a physical writer to leave its
    /// BREAK/MAB/write_all/flush linearization section. It does not interrupt
    /// a wedged driver. Callers must keep the S0 latch set and report an
    /// unconfirmed physical zero on timeout.
    pub fn lock_bounded(
        &self,
        timeout: Duration,
    ) -> Result<OpenDmxSafetyWriteGateGuard<'_>, String> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.state.try_lock() {
                Ok(state) => {
                    return Ok(OpenDmxSafetyWriteGateGuard {
                        _state: state,
                        blackout_latched: &self.blackout_latched,
                    });
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err("Open DMX physical-write gate was poisoned".to_string());
                }
                Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                    return Err(format!(
                        "Open DMX physical-write gate did not become available within {}ms",
                        timeout.as_millis()
                    ));
                }
                Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    /// Synchronize a successful authority transition (or a failed enqueue
    /// rollback) with the physical writer.  The only less-safe transition is
    /// the engine's separately authorized release path.
    pub fn set_blackout_engaged(&self, blackout_engaged: bool) -> Result<(), String> {
        // S0 selection must never wait for a driver-held physical mutex.
        // Callers authorize a clear through the engine's exact release fence;
        // a USB fault re-latches before it can wait on any physical work.
        self.blackout_latched
            .store(blackout_engaged, Ordering::Release);
        Ok(())
    }

    pub fn blackout_engaged(&self) -> Result<bool, String> {
        Ok(self.blackout_latched.load(Ordering::Acquire))
    }

    /// Preserve the physical zero-only direction even if a concurrent normal
    /// release clears its authority while a stop/fault path waits for the
    /// worker to complete one real zero transaction. The reservation carries
    /// no serial I/O and is released automatically.
    pub fn reserve_zero_write(&self) -> Result<OpenDmxZeroWriteReservation, String> {
        let state = self.lock()?;
        self.reserve_zero_write_after_lock(state)
    }

    /// Bounded variant used for USB worker stop/fault. A timeout retains the
    /// nonblocking S0 latch but establishes no reservation and no physical
    /// zero completion claim.
    pub fn reserve_zero_write_bounded(
        &self,
        timeout: Duration,
    ) -> Result<OpenDmxZeroWriteReservation, String> {
        let state = self.lock_bounded(timeout)?;
        self.reserve_zero_write_after_lock(state)
    }

    fn reserve_zero_write_after_lock(
        &self,
        _state: OpenDmxSafetyWriteGateGuard<'_>,
    ) -> Result<OpenDmxZeroWriteReservation, String> {
        if !self.blackout_latched.load(Ordering::Acquire) {
            return Err("Open DMX zero-write reservation requires engaged S0".to_string());
        }
        self.zero_write_holds
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |holds| {
                holds.checked_add(1)
            })
            .map_err(|_| "Open DMX zero-write reservation overflowed".to_string())?;
        Ok(OpenDmxZeroWriteReservation {
            gate: self.clone(),
            active: true,
        })
    }

    fn write_selected_frame<T>(
        &self,
        requested: &[u8; 512],
        write: impl FnOnce(&[u8; 512]) -> io::Result<T>,
    ) -> io::Result<(T, bool)> {
        let _physical_gate = self.lock().map_err(io::Error::other)?;
        let blackout = [0u8; 512];
        let selected_zero = self.blackout_latched.load(Ordering::Acquire)
            || self.zero_write_holds.load(Ordering::Acquire) > 0;
        let selected = if selected_zero { &blackout } else { requested };
        // Keep `_physical_gate` alive until the complete physical transaction returns.
        write(selected).map(|result| (result, selected_zero))
    }
}

/// Guard shared by the engine's S0 reservation path and the Open DMX worker.
/// The state is intentionally private so callers cannot observe an unlocked
/// decision and reuse it for a later physical write.
pub struct OpenDmxSafetyWriteGateGuard<'a> {
    _state: std::sync::MutexGuard<'a, ()>,
    blackout_latched: &'a AtomicBool,
}

impl OpenDmxSafetyWriteGateGuard<'_> {
    pub fn blackout_engaged(&self) -> bool {
        self.blackout_latched.load(Ordering::Acquire)
    }

    pub fn set_blackout_engaged(&mut self, blackout_engaged: bool) {
        self.blackout_latched
            .store(blackout_engaged, Ordering::Release);
    }
}

pub struct OpenDmxZeroWriteReservation {
    gate: OpenDmxSafetyWriteGate,
    active: bool,
}

impl Drop for OpenDmxZeroWriteReservation {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if self
            .gate
            .zero_write_holds
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |holds| {
                holds.checked_sub(1)
            })
            .is_err()
        {
            // Count corruption must never restore live selection. The
            // authoritative S0 latch remains set until an explicit recovery.
            self.gate.blackout_latched.store(true, Ordering::Release);
        }
        self.active = false;
    }
}

impl EnttecUsbProSender {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, SerialDmxError> {
        if path.trim().is_empty() {
            return Err(SerialDmxError::MissingPort);
        }
        let port = serialport::new(path, baud_rate.max(1))
            .timeout(Duration::from_millis(2))
            .open()
            .map_err(|source| SerialDmxError::Open {
                path: path.to_string(),
                source,
            })?;
        Ok(Self { port })
    }

    pub fn send_dmx_frame(&mut self, frame: &[u8; 512]) -> Result<usize, SerialDmxError> {
        write_enttec_usb_pro_dmx_frame(&mut self.port, frame).map_err(SerialDmxError::Write)
    }
}

impl EnttecOpenDmxSender {
    pub fn new(path: &str) -> Result<Self, SerialDmxError> {
        Self::new_with_safety_write_gate(path, OpenDmxSafetyWriteGate::new())
    }

    /// Open an Open-DMX transport sharing the engine's physical S0 authority.
    /// Generic callers that do not participate in engine safety use `new` and
    /// receive an isolated gate instead.
    pub fn new_with_safety_write_gate(
        path: &str,
        safety_write_gate: OpenDmxSafetyWriteGate,
    ) -> Result<Self, SerialDmxError> {
        if path.trim().is_empty() {
            return Err(SerialDmxError::MissingPort);
        }
        let port = serialport::new(path, ENTTEC_OPEN_DMX_BAUD_RATE)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::Two)
            .timeout(Duration::from_millis(2))
            .open()
            .map_err(|source| SerialDmxError::Open {
                path: path.to_string(),
                source,
            })?;
        Self::from_open_port(port, safety_write_gate)
    }

    /// Open only the exact enumerated USB device through its Windows PnP
    /// device-interface path. A COM alias and `SerialPort::name()` are both
    /// mutable/self-reported strings and are therefore never evidence that an
    /// opened handle belongs to the approved physical USB instance.
    pub fn new_verified(identity: &VerifiedUsbSerialPortIdentity) -> Result<Self, SerialDmxError> {
        Self::new_verified_with_safety_write_gate(identity, OpenDmxSafetyWriteGate::new())
    }

    /// Verified physical-device open that shares the engine's runtime-only S0
    /// write authority.  The gate is never a project/show field.
    pub fn new_verified_with_safety_write_gate(
        identity: &VerifiedUsbSerialPortIdentity,
        safety_write_gate: OpenDmxSafetyWriteGate,
    ) -> Result<Self, SerialDmxError> {
        identity.validate_for_verified_open()?;
        require_exact_verified_usb_port(&list_serial_ports()?, identity)?;
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::io::AsRawHandle;

            let pre_open = resolve_windows_com_port_binding(identity)?;
            require_windows_binding_matches_identity(identity, &pre_open)?;
            let direct_interface_path = pre_open.device_interface_path.as_str();
            let port = serialport::new(direct_interface_path, ENTTEC_OPEN_DMX_BAUD_RATE)
                .data_bits(DataBits::Eight)
                .parity(Parity::None)
                .stop_bits(StopBits::Two)
                .timeout(Duration::from_millis(2))
                .open_native()
                .map_err(|source| SerialDmxError::Open {
                    path: identity.port_name.clone(),
                    source,
                })?;
            // `GetCommState` is the supported, handle-local proof that the
            // actual returned HANDLE is a communications device. Its PnP
            // identity is established by the exact device-interface path
            // passed to CreateFile, not by trying to reverse a COM HANDLE to
            // a filesystem path (which Win32 does not support).
            windows_com_binding::verify_opened_communications_handle(port.as_raw_handle())
                .map_err(SerialDmxError::Identity)?;
            let post_open = resolve_windows_com_port_binding(identity)?;
            require_exact_verified_usb_port(&list_serial_ports()?, identity)?;
            verify_opened_windows_com_direct_binding(
                identity,
                &pre_open,
                direct_interface_path,
                &post_open,
            )?;
            return Self::from_open_port(Box::new(port), safety_write_gate);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = identity;
            Err(SerialDmxError::Identity(
                "verified Open DMX output requires the Windows PnP COM interface binding"
                    .to_string(),
            ))
        }
    }

    fn from_open_port(
        port: Box<dyn SerialPort>,
        safety_write_gate: OpenDmxSafetyWriteGate,
    ) -> Result<Self, SerialDmxError> {
        Self::from_open_port_with_before_physical_write(port, safety_write_gate, || {})
    }

    fn from_open_port_with_before_physical_write<F>(
        port: Box<dyn SerialPort>,
        safety_write_gate: OpenDmxSafetyWriteGate,
        before_physical_write: F,
    ) -> Result<Self, SerialDmxError>
    where
        F: Fn() + Send + 'static,
    {
        let frames = Arc::new(ArrayQueue::new(2));
        let running = Arc::new(AtomicBool::new(true));
        let worker_failed = Arc::new(AtomicBool::new(false));
        let worker_error = Arc::new(Mutex::new(None));
        let zero_write_generation = Arc::new(AtomicU64::new(0));
        let (worker_done_tx, worker_done_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("syndocal-open-dmx".to_string())
            .spawn({
                let frames = Arc::clone(&frames);
                let running = Arc::clone(&running);
                let worker_failed = Arc::clone(&worker_failed);
                let worker_error = Arc::clone(&worker_error);
                let zero_write_generation = Arc::clone(&zero_write_generation);
                let safety_write_gate = safety_write_gate.clone();
                move || {
                    run_enttec_open_dmx_worker(
                        SerialPortOpenDmxWriter { port },
                        frames,
                        running,
                        worker_failed,
                        worker_error,
                        zero_write_generation,
                        safety_write_gate,
                        before_physical_write,
                    );
                    let _ = worker_done_tx.send(());
                }
            })
            .map_err(SerialDmxError::WorkerStart)?;
        Ok(Self {
            frames,
            running,
            worker_failed,
            worker_error,
            zero_write_generation,
            worker: Some(worker),
            worker_done: Some(worker_done_rx),
        })
    }

    /// Debug/test-only seam for a fake `SerialPort` that still runs the real
    /// Open DMX worker and its BREAK/MAB/write_all/flush implementation. It
    /// is excluded from optimized product artifacts and exists solely for
    /// deterministic physical-write authority proof.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn from_test_serial_port<F>(
        port: OpenDmxTestSerialPort,
        safety_write_gate: OpenDmxSafetyWriteGate,
        before_physical_write: F,
    ) -> Result<Self, SerialDmxError>
    where
        F: Fn() + Send + 'static,
    {
        Self::from_open_port_with_before_physical_write(
            Box::new(port),
            safety_write_gate,
            before_physical_write,
        )
    }

    pub fn send_dmx_frame(&mut self, frame: &[u8; 512]) -> Result<usize, SerialDmxError> {
        self.worker_failure_if_any()?;
        enqueue_latest_open_dmx_frame(&self.frames, *frame);
        Ok(ENTTEC_OPEN_DMX_PAYLOAD_LEN)
    }

    /// Snapshot the physical-zero completion generation before queueing an
    /// all-zero frame. Pair with [`wait_for_zero_frame_physical_write`] while
    /// holding an [`OpenDmxZeroWriteReservation`] to prove one later zero
    /// wire transaction completed without promoting it to fixture proof.
    pub fn zero_write_receipt(&self) -> Result<OpenDmxZeroWriteReceipt, SerialDmxError> {
        self.worker_failure_if_any()?;
        Ok(OpenDmxZeroWriteReceipt {
            completed_generation_before_queue: self.zero_write_generation.load(Ordering::Acquire),
        })
    }

    pub fn wait_for_zero_frame_physical_write(
        &self,
        receipt: OpenDmxZeroWriteReceipt,
        timeout: Duration,
    ) -> Result<(), SerialDmxError> {
        let deadline = Instant::now() + timeout;
        loop {
            self.worker_failure_if_any()?;
            if self.zero_write_generation.load(Ordering::Acquire)
                > receipt.completed_generation_before_queue
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(SerialDmxError::Worker(format!(
                    "no physical zero-frame completion within {} ms",
                    timeout.as_millis()
                )));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn worker_failure_if_any(&self) -> Result<(), SerialDmxError> {
        if !self.worker_failed.load(Ordering::Acquire) {
            return Ok(());
        }
        let error = self
            .worker_error
            .lock()
            .map_err(|_| SerialDmxError::Worker("worker error lock was poisoned".to_string()))?
            .clone()
            .unwrap_or_else(|| "worker stopped without an error message".to_string());
        Err(SerialDmxError::Worker(error))
    }

    /// Stop the Open DMX worker without allowing a caller to block forever on
    /// a broken serial driver. A timeout takes and drops the `JoinHandle`, so
    /// the later sender `Drop` cannot re-enter an unbounded join. Callers must
    /// have latched S0 first. That latch makes every *later* worker selection
    /// zero-only, but does not interrupt an in-progress driver call and is
    /// never a physical-zero or fixture-delivery acknowledgement.
    pub fn shutdown_bounded(&mut self, timeout: Duration) -> Result<(), SerialDmxError> {
        self.running.store(false, Ordering::Release);
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let Some(done) = self.worker_done.take() else {
            drop(worker);
            return Err(SerialDmxError::Worker(
                "worker completion channel was unavailable during shutdown".to_string(),
            ));
        };
        match done.recv_timeout(timeout) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => worker
                .join()
                .map_err(|_| SerialDmxError::Worker("worker panicked during shutdown".to_string())),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Dropping JoinHandle detaches only after the caller has
                // reserved the zero-only physical direction.  Joining here
                // would make a stop operation unbounded on a wedged driver.
                drop(worker);
                Err(SerialDmxError::Worker(format!(
                    "worker did not stop within {} ms; S0 remains reserved",
                    timeout.as_millis()
                )))
            }
        }
    }
}

impl Drop for EnttecOpenDmxSender {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        // Destruction is never the physical-zero acknowledgement path. The
        // route stop/fault code must use `shutdown_bounded` after it has
        // latched S0 and, where possible, obtained a zero receipt. Dropping a
        // still-owned JoinHandle detaches it; this prevents an ownership
        // transition or error unwind from blocking forever in a wedged serial
        // driver.
        let _detached_worker = self.worker.take();
        self.worker_done = None;
    }
}

/// Observable operations emitted by the debug-only fake serial port. The
/// payload is retained so an engine integration test can prove that S0 chose
/// an all-zero DMX frame before the real `write_all` path.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenDmxTestSerialOperation {
    SetBreak,
    ClearBreak,
    WriteAll(Vec<u8>),
    Flush,
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct OpenDmxTestSerialState {
    operations: Vec<OpenDmxTestSerialOperation>,
    fail_next_write: bool,
}

/// Debug/test-only `SerialPort` implementation used to exercise the actual
/// Open DMX worker transaction without a physical COM interface.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Clone)]
pub struct OpenDmxTestSerialPort {
    state: Arc<Mutex<OpenDmxTestSerialState>>,
    baud_rate: u32,
    timeout: Duration,
    before_write_all: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Clone)]
pub struct OpenDmxTestSerialObservation {
    state: Arc<Mutex<OpenDmxTestSerialState>>,
}

#[cfg(debug_assertions)]
impl OpenDmxTestSerialPort {
    pub fn new() -> (Self, OpenDmxTestSerialObservation) {
        Self::new_with_before_write_all(|| {})
    }

    /// Invokes `before_write_all` after BREAK/MAB, while the real worker still
    /// owns the physical gate and immediately before its `write_all` call.
    pub fn new_with_before_write_all<F>(before_write_all: F) -> (Self, OpenDmxTestSerialObservation)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let state = Arc::new(Mutex::new(OpenDmxTestSerialState::default()));
        (
            Self {
                state: Arc::clone(&state),
                baud_rate: ENTTEC_OPEN_DMX_BAUD_RATE,
                timeout: Duration::from_millis(2),
                before_write_all: Some(Arc::new(before_write_all)),
            },
            OpenDmxTestSerialObservation { state },
        )
    }
}

#[cfg(debug_assertions)]
impl OpenDmxTestSerialObservation {
    pub fn operations(&self) -> Result<Vec<OpenDmxTestSerialOperation>, String> {
        self.state
            .lock()
            .map(|state| state.operations.clone())
            .map_err(|_| "Open DMX test serial observation was poisoned".to_string())
    }

    /// Cause the next real worker `write_all` to fail. This stays behind the
    /// debug-only fake-port seam so the engine can prove its disconnect/fault
    /// path without opening a real COM interface.
    pub fn fail_next_write(&self) -> Result<(), String> {
        self.state
            .lock()
            .map(|mut state| state.fail_next_write = true)
            .map_err(|_| "Open DMX test serial observation was poisoned".to_string())
    }
}

#[cfg(debug_assertions)]
impl io::Read for OpenDmxTestSerialPort {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::WouldBlock))
    }
}

#[cfg(debug_assertions)]
impl Write for OpenDmxTestSerialPort {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if let Some(before_write_all) = &self.before_write_all {
            before_write_all();
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("Open DMX test serial state was poisoned"))?;
        if state.fail_next_write {
            state.fail_next_write = false;
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected Open DMX test write failure",
            ));
        }
        state
            .operations
            .push(OpenDmxTestSerialOperation::WriteAll(buffer.to_vec()));
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.state
            .lock()
            .map_err(|_| io::Error::other("Open DMX test serial state was poisoned"))?
            .operations
            .push(OpenDmxTestSerialOperation::Flush);
        Ok(())
    }
}

#[cfg(debug_assertions)]
impl SerialPort for OpenDmxTestSerialPort {
    fn name(&self) -> Option<String> {
        None
    }

    fn baud_rate(&self) -> serialport::Result<u32> {
        Ok(self.baud_rate)
    }

    fn data_bits(&self) -> serialport::Result<DataBits> {
        Ok(DataBits::Eight)
    }

    fn flow_control(&self) -> serialport::Result<serialport::FlowControl> {
        Ok(serialport::FlowControl::None)
    }

    fn parity(&self) -> serialport::Result<Parity> {
        Ok(Parity::None)
    }

    fn stop_bits(&self) -> serialport::Result<StopBits> {
        Ok(StopBits::Two)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> serialport::Result<()> {
        self.baud_rate = baud_rate;
        Ok(())
    }

    fn set_data_bits(&mut self, _data_bits: DataBits) -> serialport::Result<()> {
        Ok(())
    }

    fn set_flow_control(
        &mut self,
        _flow_control: serialport::FlowControl,
    ) -> serialport::Result<()> {
        Ok(())
    }

    fn set_parity(&mut self, _parity: Parity) -> serialport::Result<()> {
        Ok(())
    }

    fn set_stop_bits(&mut self, _stop_bits: StopBits) -> serialport::Result<()> {
        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) -> serialport::Result<()> {
        self.timeout = timeout;
        Ok(())
    }

    fn write_request_to_send(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }

    fn write_data_terminal_ready(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }

    fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }

    fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }

    fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }

    fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }

    fn bytes_to_read(&self) -> serialport::Result<u32> {
        Ok(0)
    }

    fn bytes_to_write(&self) -> serialport::Result<u32> {
        Ok(0)
    }

    fn clear(&self, _buffer_to_clear: serialport::ClearBuffer) -> serialport::Result<()> {
        Ok(())
    }

    fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
        Ok(Box::new(self.clone()))
    }

    fn set_break(&self) -> serialport::Result<()> {
        self.state
            .lock()
            .map_err(|_| {
                serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    "Open DMX test serial state was poisoned",
                )
            })?
            .operations
            .push(OpenDmxTestSerialOperation::SetBreak);
        Ok(())
    }

    fn clear_break(&self) -> serialport::Result<()> {
        self.state
            .lock()
            .map_err(|_| {
                serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    "Open DMX test serial state was poisoned",
                )
            })?
            .operations
            .push(OpenDmxTestSerialOperation::ClearBreak);
        Ok(())
    }
}

pub fn list_serial_ports() -> Result<Vec<SerialPortSummary>, SerialDmxError> {
    serialport::available_ports()
        .map_err(SerialDmxError::List)
        .map(|ports| {
            ports
                .into_iter()
                .map(|port| {
                    let (usb_vid, usb_pid, serial_number, manufacturer, product) =
                        serial_port_usb_metadata(&port.port_type);
                    let recommended_protocol = recommended_serial_dmx_protocol(
                        manufacturer.as_deref(),
                        product.as_deref(),
                    );
                    #[cfg(target_os = "windows")]
                    let windows_device_instance_id =
                        windows_com_binding::resolve_for_com_alias(&port.port_name)
                            .ok()
                            .map(|binding| binding.device_instance_id);
                    #[cfg(not(target_os = "windows"))]
                    let windows_device_instance_id = None;
                    SerialPortSummary {
                        name: port.port_name,
                        port_type: serial_port_type_label(&port.port_type),
                        usb_vid,
                        usb_pid,
                        serial_number,
                        manufacturer,
                        product,
                        windows_device_instance_id,
                        recommended_protocol,
                    }
                })
                .collect()
        })
}

fn require_exact_verified_usb_port(
    ports: &[SerialPortSummary],
    identity: &VerifiedUsbSerialPortIdentity,
) -> Result<(), SerialDmxError> {
    identity.validate_for_verified_open()?;
    let same_name = ports
        .iter()
        .filter(|port| port.name == identity.port_name)
        .collect::<Vec<_>>();
    match same_name.as_slice() {
        [] => Err(SerialDmxError::Identity(format!(
            "{} is no longer enumerated",
            identity.port_name
        ))),
        [port] if identity.matches_summary(port) => Ok(()),
        [_] => Err(SerialDmxError::Identity(format!(
            "{} now resolves to a different USB hardware instance",
            identity.port_name
        ))),
        _ => Err(SerialDmxError::Identity(format!(
            "{} resolved to multiple serial hardware instances",
            identity.port_name
        ))),
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_com_port_binding(
    identity: &VerifiedUsbSerialPortIdentity,
) -> Result<WindowsComPortBinding, SerialDmxError> {
    windows_com_binding::resolve_for_com_alias(&identity.port_name)
        .map_err(SerialDmxError::Identity)
}

#[cfg(target_os = "windows")]
fn require_windows_binding_matches_identity(
    identity: &VerifiedUsbSerialPortIdentity,
    binding: &WindowsComPortBinding,
) -> Result<(), SerialDmxError> {
    let expected_instance = identity
        .windows_device_instance_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            SerialDmxError::Identity(
                "the verified show serial identity did not capture a Windows PnP device instance"
                    .to_string(),
            )
        })?;
    if binding.port_name != identity.port_name
        || binding.device_instance_id != expected_instance
        || binding.device_interface_path.trim().is_empty()
    {
        return Err(SerialDmxError::Identity(format!(
            "the Windows COM binding does not match the approved USB instance (expected COM {} / {}, observed COM {} / {})",
            identity.port_name,
            expected_instance,
            binding.port_name,
            binding.device_instance_id,
        )));
    }
    Ok(())
}

/// A direct device-interface open is the kernel-level handle binding: the
/// interface path names one present PnP devnode, while COM3 is only an alias.
/// `serialport` receives that exact SetupAPI path, and the current PnP
/// observation must still match it after the successful communications-handle
/// check. A replacement cannot be admitted through a mutable COM alias.
#[cfg(target_os = "windows")]
fn verify_opened_windows_com_direct_binding(
    identity: &VerifiedUsbSerialPortIdentity,
    pre_open: &WindowsComPortBinding,
    direct_interface_path: &str,
    post_open: &WindowsComPortBinding,
) -> Result<(), SerialDmxError> {
    require_windows_binding_matches_identity(identity, pre_open)?;
    if direct_interface_path != pre_open.device_interface_path {
        return Err(SerialDmxError::Identity(
            "the Windows COM open did not use the approved direct device-interface path"
                .to_string(),
        ));
    }
    if post_open != pre_open {
        return Err(SerialDmxError::Identity(format!(
            "the Windows COM interface changed while opening (expected {} / {}, observed {} / {})",
            pre_open.port_name,
            pre_open.device_instance_id,
            post_open.port_name,
            post_open.device_instance_id,
        )));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows_com_binding {
    use std::{mem::size_of, os::windows::io::RawHandle, slice};

    use windows::{
        core::{HRESULT, PCWSTR},
        Win32::{
            Devices::Communication::{GetCommState, DCB},
            Devices::DeviceAndDriverInstallation::{
                SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
                SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW,
                SetupDiGetDeviceRegistryPropertyW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
                SPDRP_FRIENDLYNAME, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
                SP_DEVINFO_DATA,
            },
            Foundation::{ERROR_NO_MORE_ITEMS, HANDLE},
            System::Ioctl::GUID_DEVINTERFACE_COMPORT,
        },
    };

    use super::WindowsComPortBinding;

    struct DeviceInfoSet(HDEVINFO);

    impl Drop for DeviceInfoSet {
        fn drop(&mut self) {
            // Closing an enumeration set cannot make an already-open device
            // handle safe; all selection/verification has already happened.
            unsafe {
                let _ = SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }

    pub(super) fn resolve_for_com_alias(
        expected_com_alias: &str,
    ) -> Result<WindowsComPortBinding, String> {
        if expected_com_alias.trim().is_empty() {
            return Err("the approved Windows COM alias is empty".to_string());
        }
        let matches = present_bindings()?
            .into_iter()
            .filter(|binding| binding.port_name == expected_com_alias)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [binding] => Ok(binding.clone()),
            [] => Err(format!(
                "no present Windows PnP COM device interface resolves to {expected_com_alias}"
            )),
            _ => Err(format!(
                "multiple present Windows PnP COM device interfaces resolve to {expected_com_alias}"
            )),
        }
    }

    /// `GetCommState` is defined for a communications-device handle returned
    /// by `CreateFile`. It does not identify a PnP instance; that identity is
    /// the exact SetupAPI device-interface path used for the open and checked
    /// again by the caller before the worker receives the handle.
    pub(super) fn verify_opened_communications_handle(raw_handle: RawHandle) -> Result<(), String> {
        let mut dcb = DCB {
            DCBlength: size_of::<DCB>() as u32,
            ..Default::default()
        };
        unsafe { GetCommState(HANDLE(raw_handle.cast()), &mut dcb) }.map_err(|error| {
            format!("GetCommState could not verify the opened communications handle: {error}")
        })
    }

    fn present_bindings() -> Result<Vec<WindowsComPortBinding>, String> {
        let device_info_set = unsafe {
            SetupDiGetClassDevsW(
                Some(&GUID_DEVINTERFACE_COMPORT),
                PCWSTR::null(),
                None,
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        }
        .map_err(|error| format!("Windows COM interface enumeration failed: {error}"))?;
        let device_info_set = DeviceInfoSet(device_info_set);
        let mut bindings = Vec::new();
        for index in 0.. {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            match unsafe {
                SetupDiEnumDeviceInterfaces(
                    device_info_set.0,
                    None,
                    &GUID_DEVINTERFACE_COMPORT,
                    index,
                    &mut interface,
                )
            } {
                Ok(()) => {}
                Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) => break,
                Err(error) => {
                    return Err(format!(
                        "Windows COM interface enumeration failed at index {index}: {error}"
                    ));
                }
            }
            let (device_interface_path, device_info) =
                interface_detail_and_device_info(device_info_set.0, &interface)?;
            let friendly_name = device_friendly_name(device_info_set.0, &device_info)?;
            let Some(port_name) = com_alias_from_friendly_name(&friendly_name) else {
                continue;
            };
            let device_instance_id = device_instance_id(device_info_set.0, &device_info)?;
            bindings.push(WindowsComPortBinding {
                port_name,
                device_instance_id,
                device_interface_path,
            });
        }
        Ok(bindings)
    }

    fn com_alias_from_friendly_name(friendly_name: &str) -> Option<String> {
        let (label, suffix) = friendly_name.rsplit_once(" (")?;
        let alias = suffix.strip_suffix(')')?.trim();
        (!label.trim().is_empty()
            && !alias.is_empty()
            && alias.to_ascii_uppercase().starts_with("COM"))
        .then(|| alias.to_ascii_uppercase())
    }

    fn interface_detail_and_device_info(
        device_info_set: HDEVINFO,
        interface: &SP_DEVICE_INTERFACE_DATA,
    ) -> Result<(String, SP_DEVINFO_DATA), String> {
        let mut required = 0;
        // The first call is specified to report the required buffer size.
        // Its insufficient-buffer error is expected; a zero size is not.
        unsafe {
            let _ = SetupDiGetDeviceInterfaceDetailW(
                device_info_set,
                interface,
                None,
                0,
                Some(&mut required),
                None,
            );
        }
        if required == 0 {
            return Err("Windows COM interface detail did not report a buffer size".to_string());
        }
        let mut bytes = vec![0u8; required as usize];
        let detail = bytes.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        unsafe {
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }
        let mut device_info = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                device_info_set,
                interface,
                Some(detail),
                required,
                None,
                Some(&mut device_info),
            )
        }
        .map_err(|error| format!("Windows COM interface detail read failed: {error}"))?;
        let path_offset = unsafe { std::ptr::addr_of!((*detail).DevicePath) as usize }
            .checked_sub(bytes.as_ptr() as usize)
            .ok_or_else(|| "Windows COM interface path offset was invalid".to_string())?;
        let path_units = (required as usize)
            .checked_sub(path_offset)
            .ok_or_else(|| "Windows COM interface path length was invalid".to_string())?
            / size_of::<u16>();
        let path = unsafe {
            slice::from_raw_parts(
                (bytes.as_ptr() as usize + path_offset) as *const u16,
                path_units,
            )
        };
        let path = utf16_nul_terminated(path)?;
        if path.trim().is_empty() {
            return Err("Windows COM interface path was empty".to_string());
        }
        Ok((path, device_info))
    }

    fn device_friendly_name(
        device_info_set: HDEVINFO,
        device_info: &SP_DEVINFO_DATA,
    ) -> Result<String, String> {
        let mut required = 0;
        unsafe {
            let _ = SetupDiGetDeviceRegistryPropertyW(
                device_info_set,
                device_info,
                SPDRP_FRIENDLYNAME,
                None,
                None,
                Some(&mut required),
            );
        }
        if required == 0 {
            return Err("Windows COM device did not expose a friendly name".to_string());
        }
        let mut bytes = vec![0u8; required as usize];
        unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                device_info_set,
                device_info,
                SPDRP_FRIENDLYNAME,
                None,
                Some(bytes.as_mut_slice()),
                None,
            )
        }
        .map_err(|error| format!("Windows COM friendly-name query failed: {error}"))?;
        let values = unsafe {
            slice::from_raw_parts(bytes.as_ptr() as *const u16, bytes.len() / size_of::<u16>())
        };
        utf16_nul_terminated(values)
    }

    fn device_instance_id(
        device_info_set: HDEVINFO,
        device_info: &SP_DEVINFO_DATA,
    ) -> Result<String, String> {
        let mut required = 0;
        unsafe {
            let _ = SetupDiGetDeviceInstanceIdW(
                device_info_set,
                device_info,
                None,
                Some(&mut required),
            );
        }
        if required == 0 {
            return Err("Windows COM device did not expose a PnP instance id".to_string());
        }
        let mut units = vec![0u16; required as usize];
        unsafe {
            SetupDiGetDeviceInstanceIdW(
                device_info_set,
                device_info,
                Some(units.as_mut_slice()),
                None,
            )
        }
        .map_err(|error| format!("Windows COM device-instance query failed: {error}"))?;
        utf16_nul_terminated(&units)
    }

    fn utf16_nul_terminated(values: &[u16]) -> Result<String, String> {
        let end = values
            .iter()
            .position(|value| *value == 0)
            .ok_or_else(|| "Windows COM identity text was not NUL-terminated".to_string())?;
        let value = String::from_utf16(&values[..end])
            .map_err(|_| "Windows COM identity text was invalid UTF-16".to_string())?;
        if value.trim().is_empty() {
            return Err("Windows COM identity text was empty".to_string());
        }
        Ok(value)
    }
}

type SerialPortUsbMetadata = (
    Option<u16>,
    Option<u16>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn serial_port_usb_metadata(port_type: &SerialPortType) -> SerialPortUsbMetadata {
    match port_type {
        SerialPortType::UsbPort(info) => (
            Some(info.vid),
            Some(info.pid),
            info.serial_number.clone(),
            info.manufacturer.clone(),
            info.product.clone(),
        ),
        _ => (None, None, None, None, None),
    }
}

fn recommended_serial_dmx_protocol(
    manufacturer: Option<&str>,
    product: Option<&str>,
) -> Option<DmxOutputProtocol> {
    let identity = format!(
        "{} {}",
        manufacturer.unwrap_or_default(),
        product.unwrap_or_default()
    )
    .to_ascii_lowercase();
    if identity.contains("pro mk2")
        || identity.contains("ultra pro")
        || identity.contains("ultradmx pro")
    {
        return None;
    }
    if identity.contains("dmxking") || identity.contains("ultradmx") {
        return Some(DmxOutputProtocol::DmxKingUltraDmx);
    }
    if identity.contains("open dmx") || identity.contains("usb dmx open") {
        return Some(DmxOutputProtocol::EnttecOpenDmx);
    }
    if identity.contains("dmx usb pro")
        || (identity.contains("enttec") && identity.contains("usb pro"))
    {
        return Some(DmxOutputProtocol::EnttecUsbPro);
    }
    None
}

pub fn build_enttec_open_dmx_payload(frame: &[u8; 512]) -> [u8; ENTTEC_OPEN_DMX_PAYLOAD_LEN] {
    let mut payload = [0u8; ENTTEC_OPEN_DMX_PAYLOAD_LEN];
    payload[0] = ENTTEC_OPEN_DMX_START_CODE;
    payload[1..].copy_from_slice(frame);
    payload
}

pub fn write_enttec_open_dmx_frame(
    port: &mut dyn SerialPort,
    frame: &[u8; 512],
) -> io::Result<usize> {
    port.set_break().map_err(serial_error_to_io)?;
    precise_wait(Duration::from_micros(ENTTEC_OPEN_DMX_BREAK_US));
    port.clear_break().map_err(serial_error_to_io)?;
    precise_wait(Duration::from_micros(ENTTEC_OPEN_DMX_MAB_US));

    let payload = build_enttec_open_dmx_payload(frame);
    port.write_all(&payload)?;
    port.flush()?;
    Ok(payload.len())
}

fn enqueue_latest_open_dmx_frame(queue: &ArrayQueue<[u8; 512]>, frame: [u8; 512]) {
    if let Err(frame) = queue.push(frame) {
        let _ = queue.pop();
        let _ = queue.push(frame);
    }
}

trait OpenDmxPhysicalWriter: Send {
    fn write_open_dmx_frame(&mut self, frame: &[u8; 512]) -> io::Result<usize>;
}

struct SerialPortOpenDmxWriter {
    port: Box<dyn SerialPort>,
}

impl OpenDmxPhysicalWriter for SerialPortOpenDmxWriter {
    fn write_open_dmx_frame(&mut self, frame: &[u8; 512]) -> io::Result<usize> {
        write_enttec_open_dmx_frame(&mut *self.port, frame)
    }
}

fn run_enttec_open_dmx_worker<W, F>(
    mut writer: W,
    frames: Arc<ArrayQueue<[u8; 512]>>,
    running: Arc<AtomicBool>,
    worker_failed: Arc<AtomicBool>,
    worker_error: Arc<Mutex<Option<String>>>,
    zero_write_generation: Arc<AtomicU64>,
    safety_write_gate: OpenDmxSafetyWriteGate,
    before_physical_write: F,
) where
    W: OpenDmxPhysicalWriter,
    F: Fn() + Send,
{
    let mut current_frame = None;
    while running.load(Ordering::Acquire) {
        while let Some(frame) = frames.pop() {
            current_frame = Some(frame);
        }
        let Some(frame) = current_frame.as_ref() else {
            thread::sleep(Duration::from_millis(1));
            continue;
        };
        let frame_started = Instant::now();
        // The deterministic test hook deliberately runs immediately before
        // entering the physical gate. Production supplies an empty hook.
        before_physical_write();
        match safety_write_gate
            .write_selected_frame(frame, |selected| writer.write_open_dmx_frame(selected))
        {
            Err(error) => {
                if let Ok(mut slot) = worker_error.lock() {
                    *slot = Some(error.to_string());
                }
                worker_failed.store(true, Ordering::Release);
                running.store(false, Ordering::Release);
            }
            Ok((_, wrote_zero)) => {
                if wrote_zero {
                    zero_write_generation.fetch_add(1, Ordering::AcqRel);
                }
                // Pace to the wire rate: without this the loop re-sends far faster
                // than 250 kbaud can transmit, and each frame's break lands inside
                // the previous frame still draining from the FTDI buffer.
                let wait = open_dmx_frame_pacing_wait(frame_started.elapsed());
                if !wait.is_zero() {
                    precise_wait(wait);
                }
            }
        }
    }
}

pub fn open_dmx_frame_pacing_wait(elapsed: Duration) -> Duration {
    Duration::from_micros(ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US)
        .saturating_sub(elapsed)
}

fn precise_wait(duration: Duration) {
    let start = Instant::now();
    if duration > Duration::from_millis(2) {
        thread::sleep(duration - Duration::from_millis(1));
    }
    while start.elapsed() < duration {
        spin_loop();
    }
}

pub fn build_enttec_usb_pro_dmx_packet(frame: &[u8; 512]) -> [u8; ENTTEC_PRO_SEND_PACKET_LEN] {
    let mut packet = [0u8; ENTTEC_PRO_SEND_PACKET_LEN];
    packet[0] = ENTTEC_PRO_START_DELIMITER;
    packet[1] = ENTTEC_PRO_SEND_DMX_LABEL;
    packet[2] = (ENTTEC_PRO_DMX_PAYLOAD_LEN & 0xff) as u8;
    packet[3] = (ENTTEC_PRO_DMX_PAYLOAD_LEN >> 8) as u8;
    packet[4] = ENTTEC_PRO_DMX_START_CODE;
    packet[5..517].copy_from_slice(frame);
    packet[517] = ENTTEC_PRO_END_DELIMITER;
    packet
}

pub fn write_enttec_usb_pro_dmx_frame<W: Write + ?Sized>(
    writer: &mut W,
    frame: &[u8; 512],
) -> io::Result<usize> {
    let packet = build_enttec_usb_pro_dmx_packet(frame);
    writer.write_all(&packet)?;
    writer.flush()?;
    Ok(packet.len())
}

fn serial_error_to_io(error: serialport::Error) -> io::Error {
    io::Error::other(error.to_string())
}

fn serial_port_type_label(port_type: &SerialPortType) -> String {
    match port_type {
        SerialPortType::UsbPort(info) => {
            let product = info
                .product
                .as_ref()
                .map(|value| format!(" {value}"))
                .unwrap_or_default();
            format!("USB {:04x}:{:04x}{product}", info.vid, info.pid)
        }
        SerialPortType::BluetoothPort => "Bluetooth".to_string(),
        SerialPortType::PciPort => "PCI".to_string(),
        SerialPortType::Unknown => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Barrier};

    struct FakeOpenDmxWriter {
        writes: Arc<Mutex<Vec<[u8; 512]>>>,
        entered_physical_write: Option<mpsc::Sender<()>>,
        release_physical_write: Option<mpsc::Receiver<()>>,
        block_once: bool,
    }

    impl OpenDmxPhysicalWriter for FakeOpenDmxWriter {
        fn write_open_dmx_frame(&mut self, frame: &[u8; 512]) -> io::Result<usize> {
            if self.block_once {
                self.block_once = false;
                if let Some(entered) = self.entered_physical_write.take() {
                    entered
                        .send(())
                        .map_err(|_| io::Error::other("physical-write observer dropped"))?;
                }
                if let Some(release) = self.release_physical_write.take() {
                    release
                        .recv_timeout(Duration::from_secs(1))
                        .map_err(|_| io::Error::other("physical-write release timed out"))?;
                }
            }
            self.writes
                .lock()
                .map_err(|_| io::Error::other("fake write collection poisoned"))?
                .push(*frame);
            Ok(ENTTEC_OPEN_DMX_PAYLOAD_LEN)
        }
    }

    fn wait_for_fake_writes(writes: &Arc<Mutex<Vec<[u8; 512]>>>, minimum: usize) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if writes.lock().is_ok_and(|writes| writes.len() >= minimum) {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("Open DMX fake worker did not complete {minimum} writes within the bounded test");
    }

    #[test]
    fn open_dmx_zero_receipt_requires_a_later_real_zero_transaction() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        safety_gate
            .engage_blackout()
            .expect("the test must establish S0 before reserving physical zero output");
        let (port, observation) = OpenDmxTestSerialPort::new();
        let mut sender =
            EnttecOpenDmxSender::from_test_serial_port(port, safety_gate.clone(), || {})
                .expect("the fake serial port must start the real Open DMX worker");
        let _zero_hold = safety_gate.reserve_zero_write().expect(
            "the S0 reservation must prevent a concurrent release from selecting live bytes",
        );
        let receipt = sender
            .zero_write_receipt()
            .expect("a healthy worker must expose a receipt before its zero frame is queued");
        sender
            .send_dmx_frame(&[0u8; 512])
            .expect("the initial zero frame must enter the bounded worker queue");
        sender
            .wait_for_zero_frame_physical_write(receipt, Duration::from_secs(1))
            .expect("the receipt must advance only after BREAK/MAB/write_all/flush completed");

        let operations = observation
            .operations()
            .expect("the fake serial observation must remain readable");
        let write_index = operations
            .iter()
            .position(|operation| matches!(operation, OpenDmxTestSerialOperation::WriteAll(payload) if payload[1..].iter().all(|value| *value == 0)))
            .expect("the acknowledged transaction must have a zero DMX payload");
        assert!(
            operations[write_index + 1..]
                .iter()
                .any(|operation| matches!(operation, OpenDmxTestSerialOperation::Flush)),
            "the receipt is invalid if the zero payload was not flushed"
        );
        sender
            .shutdown_bounded(Duration::from_secs(1))
            .expect("the fake worker must stop within the bounded interval");
    }

    #[test]
    fn open_dmx_s0_latch_preempts_a_wedged_physical_gate_without_claiming_a_zero_write() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        let held_physical_gate = safety_gate
            .lock()
            .expect("the test must hold the physical gate without serial I/O");

        let started = Instant::now();
        safety_gate
            .engage_blackout()
            .expect("the atomic S0 latch must not wait for a wedged physical gate");
        assert!(
            started.elapsed() < Duration::from_millis(25),
            "S0 latch unexpectedly waited for the physical mutex"
        );
        assert!(
            safety_gate
                .blackout_engaged()
                .expect("the lock-free S0 latch must remain observable"),
            "a physical-gate holder cannot hide the safer selection direction"
        );

        drop(held_physical_gate);
        let mut requested = [0u8; 512];
        requested[17] = 99;
        let mut selected = [0u8; 512];
        let (_, selected_zero) = safety_gate
            .write_selected_frame(&requested, |frame| {
                selected.copy_from_slice(frame);
                Ok(())
            })
            .expect("the recovered writer must be selectable");
        assert!(selected_zero);
        assert!(
            selected.iter().all(|value| *value == 0),
            "the latch proves only future selection is zero-only; it does not claim a prior zero transaction"
        );
    }

    #[test]
    fn open_dmx_zero_reservation_drop_is_nonblocking_and_allows_explicit_rearm_release() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        safety_gate
            .engage_blackout()
            .expect("the test must establish S0 before reserving zero output");
        let reservation = safety_gate
            .reserve_zero_write()
            .expect("the zero reservation must increment while the physical gate is owned");
        let held_physical_gate = safety_gate
            .lock()
            .expect("the test must emulate a wedged physical writer");

        let started = Instant::now();
        drop(reservation);
        assert!(
            started.elapsed() < Duration::from_millis(25),
            "reservation Drop must not wait for the driver-owned physical mutex"
        );
        drop(held_physical_gate);

        // This models a later, explicit successful recovery/re-arm/release
        // in the same process. The reservation count must not remain silently
        // leaked and force zero while the logical status says live.
        safety_gate
            .set_blackout_engaged(false)
            .expect("the explicit release must clear only the atomic S0 latch");
        let mut requested = [0u8; 512];
        requested[31] = 123;
        let mut selected = [0u8; 512];
        let (_, selected_zero) = safety_gate
            .write_selected_frame(&requested, |frame| {
                selected.copy_from_slice(frame);
                Ok(())
            })
            .expect("the recovered physical writer must be selectable");
        assert!(!selected_zero);
        assert_eq!(selected, requested);
    }

    #[test]
    fn open_dmx_shutdown_timeout_detaches_before_sender_drop_can_join_a_wedged_worker() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        let physical_entered = Arc::new(Barrier::new(2));
        let physical_release = Arc::new(Barrier::new(2));
        let first_write = Arc::new(AtomicBool::new(true));
        let (port, _observation) = OpenDmxTestSerialPort::new_with_before_write_all({
            let physical_entered = Arc::clone(&physical_entered);
            let physical_release = Arc::clone(&physical_release);
            let first_write = Arc::clone(&first_write);
            move || {
                if first_write.swap(false, Ordering::AcqRel) {
                    physical_entered.wait();
                    physical_release.wait();
                }
            }
        });
        let mut sender =
            EnttecOpenDmxSender::from_test_serial_port(port, safety_gate.clone(), || {})
                .expect("the fake serial port must start the real worker");
        let mut live = [0u8; 512];
        live[9] = 77;
        sender
            .send_dmx_frame(&live)
            .expect("the fake worker must accept the queued live frame");
        physical_entered.wait();
        safety_gate.latch_blackout();

        let shutdown_started = Instant::now();
        assert!(
            sender.shutdown_bounded(Duration::from_millis(25)).is_err(),
            "the deliberately wedged worker must exceed the bounded shutdown receipt"
        );
        assert!(
            shutdown_started.elapsed() < Duration::from_millis(250),
            "bounded shutdown unexpectedly waited for the wedged driver"
        );
        let drop_started = Instant::now();
        drop(sender);
        assert!(
            drop_started.elapsed() < Duration::from_millis(25),
            "sender Drop must not re-enter an unbounded worker join after timeout"
        );
        physical_release.wait();
    }

    fn verified_usb_port(serial_number: &str) -> SerialPortSummary {
        SerialPortSummary {
            name: "COM3".to_string(),
            port_type: "USB 0403:6001 USB Serial Port".to_string(),
            usb_vid: Some(0x0403),
            usb_pid: Some(0x6001),
            serial_number: Some(serial_number.to_string()),
            manufacturer: Some("FTDI".to_string()),
            product: Some("USB Serial Port".to_string()),
            windows_device_instance_id: Some(format!(
                r"FTDIBUS\VID_0403+PID_6001+{serial_number}\0000"
            )),
            recommended_protocol: None,
        }
    }

    #[test]
    fn open_dmx_worker_s0_reservation_before_physical_gate_writes_zero_only() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        let frames = Arc::new(ArrayQueue::new(2));
        let mut live = [0u8; 512];
        live[7] = 91;
        frames.push(live).expect("test queue has capacity");
        let running = Arc::new(AtomicBool::new(true));
        let failed = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let zero_write_generation = Arc::new(AtomicU64::new(0));
        let writes = Arc::new(Mutex::new(Vec::new()));
        let (before_gate_entered_tx, before_gate_entered_rx) = mpsc::channel();
        let (release_before_gate_tx, release_before_gate_rx) = mpsc::channel();
        let block_once = Arc::new(AtomicBool::new(true));

        let worker = thread::spawn({
            let frames = Arc::clone(&frames);
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            let error = Arc::clone(&error);
            let zero_write_generation = Arc::clone(&zero_write_generation);
            let writes = Arc::clone(&writes);
            let safety_gate = safety_gate.clone();
            let block_once = Arc::clone(&block_once);
            move || {
                run_enttec_open_dmx_worker(
                    FakeOpenDmxWriter {
                        writes,
                        entered_physical_write: None,
                        release_physical_write: None,
                        block_once: false,
                    },
                    frames,
                    running,
                    failed,
                    error,
                    zero_write_generation,
                    safety_gate,
                    move || {
                        if block_once.swap(false, Ordering::AcqRel) {
                            before_gate_entered_tx
                                .send(())
                                .expect("test observer must receive worker barrier");
                            release_before_gate_rx
                                .recv_timeout(Duration::from_secs(1))
                                .expect("test must release worker barrier");
                        }
                    },
                );
            }
        });

        before_gate_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker must reach the exact pre-physical-write barrier");
        safety_gate
            .engage_blackout()
            .expect("S0 reservation must acquire the physical gate before release");
        release_before_gate_tx
            .send(())
            .expect("worker barrier must still be waiting");
        wait_for_fake_writes(&writes, 1);
        running.store(false, Ordering::Release);
        worker
            .join()
            .expect("worker must terminate without deadlock");

        assert!(!failed.load(Ordering::Acquire));
        let writes = writes.lock().expect("writes must be readable");
        assert!(!writes.is_empty());
        assert!(writes
            .iter()
            .all(|frame| frame.iter().all(|value| *value == 0)));
    }

    #[test]
    fn open_dmx_worker_write_winner_allows_one_live_frame_then_s0_forces_zero() {
        let safety_gate = OpenDmxSafetyWriteGate::new();
        let frames = Arc::new(ArrayQueue::new(2));
        let mut live = [0u8; 512];
        live[9] = 127;
        frames.push(live).expect("test queue has capacity");
        let running = Arc::new(AtomicBool::new(true));
        let failed = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let zero_write_generation = Arc::new(AtomicU64::new(0));
        let writes = Arc::new(Mutex::new(Vec::new()));
        let (physical_entered_tx, physical_entered_rx) = mpsc::channel();
        let (physical_release_tx, physical_release_rx) = mpsc::channel();

        let worker = thread::spawn({
            let frames = Arc::clone(&frames);
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            let error = Arc::clone(&error);
            let zero_write_generation = Arc::clone(&zero_write_generation);
            let writes = Arc::clone(&writes);
            let safety_gate = safety_gate.clone();
            move || {
                run_enttec_open_dmx_worker(
                    FakeOpenDmxWriter {
                        writes,
                        entered_physical_write: Some(physical_entered_tx),
                        release_physical_write: Some(physical_release_rx),
                        block_once: true,
                    },
                    frames,
                    running,
                    failed,
                    error,
                    zero_write_generation,
                    safety_gate,
                    || {},
                );
            }
        });

        physical_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker must own the gate before its first wire write");
        let (s0_done_tx, s0_done_rx) = mpsc::channel();
        let s0_gate = safety_gate.clone();
        let s0 = thread::spawn(move || {
            let result = s0_gate.engage_blackout();
            let _ = s0_done_tx.send(result);
        });
        s0_done_rx
            .recv_timeout(Duration::from_millis(25))
            .expect("the atomic S0 latch must not wait for the worker-owned physical write")
            .expect("the S0 latch must not be poisoned");
        s0.join()
            .expect("S0 latch thread must terminate before the wedged write releases");
        physical_release_tx
            .send(())
            .expect("worker must still be in its first physical write");
        wait_for_fake_writes(&writes, 2);
        running.store(false, Ordering::Release);
        worker
            .join()
            .expect("worker must terminate without deadlock");

        assert!(!failed.load(Ordering::Acquire));
        let writes = writes.lock().expect("writes must be readable");
        assert!(writes[0].iter().any(|value| *value != 0));
        assert!(writes[1..]
            .iter()
            .all(|frame| frame.iter().all(|value| *value == 0)));
    }

    #[test]
    fn builds_enttec_usb_pro_send_dmx_packet() {
        let mut frame = [0u8; 512];
        frame[0] = 255;
        frame[511] = 64;

        let packet = build_enttec_usb_pro_dmx_packet(&frame);

        assert_eq!(packet.len(), ENTTEC_PRO_SEND_PACKET_LEN);
        assert_eq!(packet[0], ENTTEC_PRO_START_DELIMITER);
        assert_eq!(packet[1], ENTTEC_PRO_SEND_DMX_LABEL);
        assert_eq!(packet[2], 0x01);
        assert_eq!(packet[3], 0x02);
        assert_eq!(packet[4], ENTTEC_PRO_DMX_START_CODE);
        assert_eq!(packet[5], 255);
        assert_eq!(packet[516], 64);
        assert_eq!(packet[517], ENTTEC_PRO_END_DELIMITER);
    }

    #[test]
    fn enttec_usb_pro_packet_preserves_all_slots_after_start_code() {
        let mut frame = [0u8; 512];
        frame[0] = 1;
        frame[1] = 2;
        frame[510] = 254;
        frame[511] = 255;

        let packet = build_enttec_usb_pro_dmx_packet(&frame);

        assert_eq!(
            u16::from_le_bytes([packet[2], packet[3]]) as usize,
            ENTTEC_PRO_DMX_PAYLOAD_LEN
        );
        assert_eq!(packet[4], ENTTEC_PRO_DMX_START_CODE);
        assert_eq!(&packet[5..517], frame);
    }

    #[test]
    fn builds_enttec_open_dmx_payload() {
        let mut frame = [0u8; 512];
        frame[0] = 255;
        frame[511] = 64;

        let payload = build_enttec_open_dmx_payload(&frame);

        assert_eq!(payload.len(), ENTTEC_OPEN_DMX_PAYLOAD_LEN);
        assert_eq!(payload[0], ENTTEC_OPEN_DMX_START_CODE);
        assert_eq!(payload[1], 255);
        assert_eq!(payload[512], 64);
    }

    #[test]
    fn enttec_open_dmx_payload_preserves_slot_order_after_start_code() {
        let mut frame = [0u8; 512];
        frame[0] = 1;
        frame[1] = 2;
        frame[510] = 254;
        frame[511] = 255;

        let payload = build_enttec_open_dmx_payload(&frame);

        assert_eq!(payload[0], ENTTEC_OPEN_DMX_START_CODE);
        assert_eq!(&payload[1..], frame);
    }

    #[test]
    fn open_dmx_frame_wire_time_matches_250kbaud_8n2_full_frame() {
        // 176us break + 16us MAB + 513 bytes x 44us (1 start + 8 data + 2 stop).
        assert_eq!(ENTTEC_OPEN_DMX_FRAME_WIRE_US, 176 + 16 + 513 * 44);
        assert_eq!(ENTTEC_OPEN_DMX_FRAME_WIRE_US, 22_764);
    }

    #[test]
    fn exact_ft232r_default_pacing_is_about_32_point_5_fps_not_a_44hz_usb_guarantee() {
        // The exact-rig default includes the accepted 8ms drain guard, so a
        // latest-frame worker cannot promise a distinct physical USB frame
        // for every 44Hz engine tick. Keep the integral bounds deterministic
        // and leave any 36-40fps promotion to new physical evidence.
        assert_eq!(ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US, 30_764);
        assert!(32 * ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US < 1_000_000);
        assert!(33 * ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US > 1_000_000);
    }

    #[test]
    fn open_dmx_pacing_waits_the_full_frame_budget_from_a_fast_write() {
        let wait = open_dmx_frame_pacing_wait(Duration::ZERO);
        assert_eq!(
            wait,
            Duration::from_micros(ENTTEC_OPEN_DMX_EXACT_FT232R_DEFAULT_FRAME_PERIOD_US)
        );
    }

    #[test]
    fn open_dmx_pacing_does_not_wait_when_the_write_already_took_the_budget() {
        let budget =
            Duration::from_micros(ENTTEC_OPEN_DMX_FRAME_WIRE_US + ENTTEC_OPEN_DMX_FRAME_GUARD_US);
        assert_eq!(open_dmx_frame_pacing_wait(budget), Duration::ZERO);
        assert_eq!(
            open_dmx_frame_pacing_wait(budget + Duration::from_millis(5)),
            Duration::ZERO
        );
    }

    #[test]
    fn open_dmx_mailbox_replaces_the_oldest_frame_when_full() {
        let queue = ArrayQueue::new(2);
        enqueue_latest_open_dmx_frame(&queue, [1; 512]);
        enqueue_latest_open_dmx_frame(&queue, [2; 512]);
        enqueue_latest_open_dmx_frame(&queue, [3; 512]);

        assert_eq!(queue.pop().unwrap()[0], 2);
        assert_eq!(queue.pop().unwrap()[0], 3);
    }

    #[test]
    fn writes_enttec_usb_pro_packet_to_writer() {
        let mut frame = [0u8; 512];
        frame[10] = 123;
        let mut writer = Vec::new();

        let written = write_enttec_usb_pro_dmx_frame(&mut writer, &frame).unwrap();

        assert_eq!(written, ENTTEC_PRO_SEND_PACKET_LEN);
        assert_eq!(writer, build_enttec_usb_pro_dmx_packet(&frame));
    }

    #[test]
    fn serial_senders_reject_blank_port_paths_before_opening() {
        assert!(matches!(
            EnttecUsbProSender::new(" ", 57_600),
            Err(SerialDmxError::MissingPort)
        ));
        assert!(matches!(
            EnttecOpenDmxSender::new(""),
            Err(SerialDmxError::MissingPort)
        ));
    }

    #[cfg(target_os = "windows")]
    fn windows_binding(instance: &str, interface_suffix: &str) -> WindowsComPortBinding {
        WindowsComPortBinding {
            port_name: "COM3".to_string(),
            device_instance_id: instance.to_string(),
            device_interface_path: format!(
                r"\\?\USB#VID_0403&PID_6001#{interface_suffix}#{{86E0D1E0-8089-11D0-9CE4-08003E301F73}}"
            ),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn verified_windows_direct_com_open_rejects_identity_or_path_drift_without_port_name_trust() {
        let before_open = verified_usb_port("FTDI-INSTANCE-A");
        let mut identity = VerifiedUsbSerialPortIdentity::from_summary(&before_open)
            .expect("a complete enumerated USB identity must be capturable");
        let device_a = r"FTDIBUS\VID_0403+PID_6001+FTDI-INSTANCE-A\0000";
        identity.windows_device_instance_id = Some(device_a.to_string());
        let pre_open_a = windows_binding(device_a, "FTDI-INSTANCE-A");
        let post_open_a = pre_open_a.clone();

        assert!(verify_opened_windows_com_direct_binding(
            &identity,
            &pre_open_a,
            &pre_open_a.device_interface_path,
            &post_open_a,
        )
        .is_ok());

        // The direct SetupAPI path is not a mutable COM alias. A→B→A must not
        // become permissive merely because the post-open alias again shows A:
        // the actual CreateFile target must remain A's exact interface path.
        let device_b = r"FTDIBUS\VID_0403+PID_6001+FTDI-INSTANCE-B\0000";
        let binding_b = windows_binding(device_b, "FTDI-INSTANCE-B");
        assert!(verify_opened_windows_com_direct_binding(
            &identity,
            &pre_open_a,
            &binding_b.device_interface_path,
            &post_open_a,
        )
        .is_err());

        assert!(verify_opened_windows_com_direct_binding(
            &identity,
            &pre_open_a,
            &pre_open_a.device_interface_path,
            &binding_b,
        )
        .is_err());

        let mut wrong_identity = identity.clone();
        wrong_identity.windows_device_instance_id = Some(device_b.to_string());
        assert!(verify_opened_windows_com_direct_binding(
            &wrong_identity,
            &pre_open_a,
            &pre_open_a.device_interface_path,
            &post_open_a,
        )
        .is_err());

        let mut missing_instance = before_open;
        missing_instance.serial_number = Some(" ".to_string());
        assert!(matches!(
            VerifiedUsbSerialPortIdentity::from_summary(&missing_instance),
            Err(SerialDmxError::Identity(_))
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn verified_windows_com_handle_rejects_a_non_communications_handle() {
        use std::os::windows::io::AsRawHandle;

        let executable =
            std::env::current_exe().expect("the test process executable path must be available");
        let file = std::fs::File::open(executable)
            .expect("the test process executable must be openable as an ordinary file");
        let error = windows_com_binding::verify_opened_communications_handle(file.as_raw_handle())
            .expect_err("GetCommState must reject an ordinary file handle");
        assert!(error.contains("GetCommState"));
    }

    #[test]
    fn verified_usb_identity_rejects_zero_vid_or_pid() {
        let complete = verified_usb_port("FTDI-INSTANCE-A");
        for zero_vid in [true, false] {
            let mut invalid = complete.clone();
            if zero_vid {
                invalid.usb_vid = Some(0);
            } else {
                invalid.usb_pid = Some(0);
            }
            let error = VerifiedUsbSerialPortIdentity::from_summary(&invalid)
                .expect_err("zero USB identity components must fail closed");
            assert!(error.to_string().contains("nonzero USB VID and PID"));
        }
    }

    #[test]
    fn verified_open_rejects_manual_incomplete_or_zero_identity_before_enumeration() {
        let enumerated = verified_usb_port("FTDI-INSTANCE-A");
        let mut complete = VerifiedUsbSerialPortIdentity::from_summary(&enumerated)
            .expect("the complete summary must establish the non-PnP identity fields");
        complete.windows_device_instance_id = enumerated.windows_device_instance_id.clone();

        for missing_field in [
            "port_name",
            "port_type",
            "serial_number",
            "manufacturer",
            "product",
            "windows_device_instance_id",
        ] {
            let mut invalid = complete.clone();
            match missing_field {
                "port_name" => invalid.port_name = " ".to_string(),
                "port_type" => invalid.port_type = " ".to_string(),
                "serial_number" => invalid.serial_number = " ".to_string(),
                "manufacturer" => invalid.manufacturer = " ".to_string(),
                "product" => invalid.product = " ".to_string(),
                "windows_device_instance_id" => {
                    invalid.windows_device_instance_id = Some(" ".to_string())
                }
                _ => unreachable!("the test owns every incomplete identity case"),
            }
            assert!(
                require_exact_verified_usb_port(std::slice::from_ref(&enumerated), &invalid)
                    .is_err(),
                "manual {missing_field} identity must fail before any open"
            );
        }

        let mut zero_vid = complete.clone();
        zero_vid.usb_vid = 0;
        let exact_error =
            require_exact_verified_usb_port(std::slice::from_ref(&enumerated), &zero_vid)
                .expect_err("manual zero VID must fail the exact enumeration boundary");
        assert!(exact_error.to_string().contains("nonzero USB VID and PID"));
        let open_error = match EnttecOpenDmxSender::new_verified_with_safety_write_gate(
            &zero_vid,
            OpenDmxSafetyWriteGate::new(),
        ) {
            Ok(_) => {
                panic!("manual zero VID must fail before serial enumeration or a physical open")
            }
            Err(error) => error,
        };
        assert!(open_error.to_string().contains("nonzero USB VID and PID"));
    }

    #[test]
    fn verified_enumeration_requires_the_exact_windows_pnp_instance() {
        let enumerated = verified_usb_port("FTDI-INSTANCE-A");
        let mut identity = VerifiedUsbSerialPortIdentity::from_summary(&enumerated)
            .expect("the complete summary must establish the non-PnP identity fields");
        identity.windows_device_instance_id = enumerated.windows_device_instance_id.clone();
        assert!(
            require_exact_verified_usb_port(std::slice::from_ref(&enumerated), &identity).is_ok()
        );

        let mut replacement = enumerated;
        replacement.windows_device_instance_id =
            Some(r"FTDIBUS\VID_0403+PID_6001+FTDI-INSTANCE-B\0000".to_string());
        assert!(require_exact_verified_usb_port(&[replacement], &identity).is_err());
    }

    #[test]
    fn recommends_only_known_single_port_serial_dmx_products() {
        assert_eq!(
            recommended_serial_dmx_protocol(Some("ENTTEC"), Some("DMX USB Pro")),
            Some(DmxOutputProtocol::EnttecUsbPro)
        );
        assert_eq!(
            recommended_serial_dmx_protocol(Some("DMXKing"), Some("ultraDMX Micro")),
            Some(DmxOutputProtocol::DmxKingUltraDmx)
        );
        assert_eq!(
            recommended_serial_dmx_protocol(Some("ENTTEC"), Some("Open DMX USB")),
            Some(DmxOutputProtocol::EnttecOpenDmx)
        );
        assert_eq!(
            recommended_serial_dmx_protocol(Some("FTDI"), Some("FT232R USB UART")),
            None
        );
        assert_eq!(
            recommended_serial_dmx_protocol(Some("ENTTEC"), Some("DMX USB Pro Mk2")),
            None
        );
        assert_eq!(
            recommended_serial_dmx_protocol(Some("DMXKing"), Some("ultraDMX Pro")),
            None
        );
    }
}
