use std::{
    hint::spin_loop,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
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
pub const ENTTEC_OPEN_DMX_FRAME_GUARD_US: u64 = 8_000;
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
        identity.windows_device_instance_id = Some(binding.device_instance_id);
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
        summary.name == self.port_name
            && summary.port_type == self.port_type
            && summary.usb_vid == Some(self.usb_vid)
            && summary.usb_pid == Some(self.usb_pid)
            && summary.serial_number.as_deref() == Some(self.serial_number.as_str())
            && summary.manufacturer.as_deref() == Some(self.manufacturer.as_str())
            && summary.product.as_deref() == Some(self.product.as_str())
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
    worker: Option<JoinHandle<()>>,
}

/// The physical linearization boundary for Open DMX output.
///
/// The worker holds this short mutex from deciding whether a queued frame is
/// live or zero through BREAK, MAB, `write_all`, and `flush`.  An S0 enqueue
/// uses the same mutex before marking the shared blackout bit.  Therefore a
/// write and an S0 reservation have one deterministic order: a worker that
/// wins may finish one live wire write, while a reservation that wins forces
/// this and every later worker write to an all-zero frame.  This is runtime
/// authority only; callers must never persist it in an `.sdc` project.
#[derive(Clone, Debug)]
pub struct OpenDmxSafetyWriteGate {
    state: Arc<Mutex<OpenDmxSafetyWriteState>>,
}

#[derive(Debug, Clone, Copy)]
struct OpenDmxSafetyWriteState {
    blackout_engaged: bool,
}

impl Default for OpenDmxSafetyWriteGate {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenDmxSafetyWriteGate {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(OpenDmxSafetyWriteState {
                blackout_engaged: false,
            })),
        }
    }

    /// Atomically reserve the safer physical direction before the priority S0
    /// command becomes observable.  The caller may hold other engine gates;
    /// this method itself contains no serial I/O.
    pub fn engage_blackout(&self) -> Result<(), String> {
        self.set_blackout_engaged(true)
    }

    /// Acquire the physical-write authority.  The returned guard is kept
    /// across the complete Open DMX wire transaction, not merely frame
    /// selection, so an S0 reservation and an actual `write_all` have one
    /// deterministic order.
    pub fn lock(&self) -> Result<OpenDmxSafetyWriteGateGuard<'_>, String> {
        self.state
            .lock()
            .map(|state| OpenDmxSafetyWriteGateGuard { state })
            .map_err(|_| "Open DMX physical-write gate was poisoned".to_string())
    }

    /// Synchronize a successful authority transition (or a failed enqueue
    /// rollback) with the physical writer.  The only less-safe transition is
    /// the engine's separately authorized release path.
    pub fn set_blackout_engaged(&self, blackout_engaged: bool) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Open DMX physical-write gate was poisoned".to_string())?;
        state.blackout_engaged = blackout_engaged;
        Ok(())
    }

    pub fn blackout_engaged(&self) -> Result<bool, String> {
        self.lock().map(|state| state.blackout_engaged())
    }

    fn write_selected_frame<T>(
        &self,
        requested: &[u8; 512],
        write: impl FnOnce(&[u8; 512]) -> io::Result<T>,
    ) -> io::Result<T> {
        let state = self.lock().map_err(io::Error::other)?;
        let blackout = [0u8; 512];
        let selected = if state.blackout_engaged() {
            &blackout
        } else {
            requested
        };
        // Keep `state` alive until the complete physical transaction returns.
        write(selected)
    }
}

/// Guard shared by the engine's S0 reservation path and the Open DMX worker.
/// The state is intentionally private so callers cannot observe an unlocked
/// decision and reuse it for a later physical write.
pub struct OpenDmxSafetyWriteGateGuard<'a> {
    state: std::sync::MutexGuard<'a, OpenDmxSafetyWriteState>,
}

impl OpenDmxSafetyWriteGateGuard<'_> {
    pub fn blackout_engaged(&self) -> bool {
        self.state.blackout_engaged
    }

    pub fn set_blackout_engaged(&mut self, blackout_engaged: bool) {
        self.state.blackout_engaged = blackout_engaged;
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
        require_exact_verified_usb_port(&list_serial_ports()?, identity)?;
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::io::AsRawHandle;

            let pre_open = resolve_windows_com_port_binding(identity)?;
            require_windows_binding_matches_identity(identity, &pre_open)?;
            let port = serialport::new(&pre_open.device_interface_path, ENTTEC_OPEN_DMX_BAUD_RATE)
                .data_bits(DataBits::Eight)
                .parity(Parity::None)
                .stop_bits(StopBits::Two)
                .timeout(Duration::from_millis(2))
                .open_native()
                .map_err(|source| SerialDmxError::Open {
                    path: identity.port_name.clone(),
                    source,
                })?;
            // Resolve the actual opened HANDLE through Win32, not through
            // serialport's self-reported name. Re-resolve the live PnP
            // binding before the worker can own it; a removed/replaced
            // devnode cannot be promoted merely because COM3 returns to A.
            let opened_handle =
                windows_com_binding::binding_for_opened_handle(port.as_raw_handle())
                    .map_err(SerialDmxError::Identity)?;
            let post_open = resolve_windows_com_port_binding(identity)?;
            require_exact_verified_usb_port(&list_serial_ports()?, identity)?;
            verify_opened_windows_com_handle_binding(
                identity,
                &pre_open,
                &opened_handle,
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
        let worker = thread::Builder::new()
            .name("syndocal-open-dmx".to_string())
            .spawn({
                let frames = Arc::clone(&frames);
                let running = Arc::clone(&running);
                let worker_failed = Arc::clone(&worker_failed);
                let worker_error = Arc::clone(&worker_error);
                let safety_write_gate = safety_write_gate.clone();
                move || {
                    run_enttec_open_dmx_worker(
                        SerialPortOpenDmxWriter { port },
                        frames,
                        running,
                        worker_failed,
                        worker_error,
                        safety_write_gate,
                        before_physical_write,
                    )
                }
            })
            .map_err(SerialDmxError::WorkerStart)?;
        Ok(Self {
            frames,
            running,
            worker_failed,
            worker_error,
            worker: Some(worker),
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
        if self.worker_failed.load(Ordering::Acquire) {
            let error = self
                .worker_error
                .lock()
                .map_err(|_| SerialDmxError::Worker("worker error lock was poisoned".to_string()))?
                .clone()
                .unwrap_or_else(|| "worker stopped without an error message".to_string());
            return Err(SerialDmxError::Worker(error));
        }
        enqueue_latest_open_dmx_frame(&self.frames, *frame);
        Ok(ENTTEC_OPEN_DMX_PAYLOAD_LEN)
    }
}

impl Drop for EnttecOpenDmxSender {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
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
        self.state
            .lock()
            .map_err(|_| io::Error::other("Open DMX test serial state was poisoned"))?
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
/// Verify all three observations to reject a swap even when A returns to COM3
/// after B occupied it during the enumerate/open interval.
#[cfg(target_os = "windows")]
fn verify_opened_windows_com_handle_binding(
    identity: &VerifiedUsbSerialPortIdentity,
    pre_open: &WindowsComPortBinding,
    opened_handle: &WindowsComPortBinding,
    post_open: &WindowsComPortBinding,
) -> Result<(), SerialDmxError> {
    require_windows_binding_matches_identity(identity, pre_open)?;
    if opened_handle != pre_open {
        return Err(SerialDmxError::Identity(format!(
            "the opened Windows COM handle bound to {} / {}, not approved {} / {}",
            opened_handle.port_name,
            opened_handle.device_instance_id,
            pre_open.port_name,
            pre_open.device_instance_id,
        )));
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
            Devices::DeviceAndDriverInstallation::{
                SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
                SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW,
                SetupDiGetDeviceRegistryPropertyW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
                SPDRP_FRIENDLYNAME, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
                SP_DEVINFO_DATA,
            },
            Foundation::{ERROR_NO_MORE_ITEMS, HANDLE},
            Storage::FileSystem::{GetFinalPathNameByHandleW, FILE_NAME_OPENED},
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

    /// Resolve the real kernel handle's *opened* path through Win32, then
    /// join it back to exactly one current SetupAPI PnP interface. The caller
    /// never supplies a claimed handle identity and this intentionally has no
    /// dependency on `SerialPort::name()`.
    pub(super) fn binding_for_opened_handle(
        raw_handle: RawHandle,
    ) -> Result<WindowsComPortBinding, String> {
        let opened_path = opened_handle_device_interface_path(raw_handle)?;
        let matches = present_bindings()?
            .into_iter()
            .filter(|binding| {
                windows_device_path_matches(&binding.device_interface_path, &opened_path)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [binding] => Ok(binding.clone()),
            [] => Err(format!(
                "the opened Windows COM handle path {opened_path} did not resolve to one present PnP COM interface"
            )),
            _ => Err(format!(
                "the opened Windows COM handle path {opened_path} resolved to multiple present PnP COM interfaces"
            )),
        }
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

    fn opened_handle_device_interface_path(raw_handle: RawHandle) -> Result<String, String> {
        let handle = HANDLE(raw_handle.cast());
        let mut capacity = 512usize;
        for _ in 0..4 {
            let mut units = vec![0u16; capacity];
            let written = unsafe {
                GetFinalPathNameByHandleW(handle, units.as_mut_slice(), FILE_NAME_OPENED)
            } as usize;
            if written == 0 {
                return Err(format!(
                    "GetFinalPathNameByHandleW could not resolve the opened COM handle: {}",
                    windows::core::Error::from_win32(),
                ));
            }
            if written < units.len() {
                return String::from_utf16(&units[..written]).map_err(|_| {
                    "GetFinalPathNameByHandleW returned invalid UTF-16 for the opened COM handle"
                        .to_string()
                });
            }
            capacity = written.saturating_add(1);
        }
        Err("GetFinalPathNameByHandleW exceeded the bounded opened-COM path buffer".to_string())
    }

    fn windows_device_path_matches(expected: &str, observed: &str) -> bool {
        expected.eq_ignore_ascii_case(observed)
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
        if let Err(error) = safety_write_gate
            .write_selected_frame(frame, |selected| writer.write_open_dmx_frame(selected))
        {
            if let Ok(mut slot) = worker_error.lock() {
                *slot = Some(error.to_string());
            }
            worker_failed.store(true, Ordering::Release);
            running.store(false, Ordering::Release);
        } else {
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

pub fn open_dmx_frame_pacing_wait(elapsed: Duration) -> Duration {
    Duration::from_micros(ENTTEC_OPEN_DMX_FRAME_WIRE_US + ENTTEC_OPEN_DMX_FRAME_GUARD_US)
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
    use std::sync::mpsc;

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

    fn verified_usb_port(serial_number: &str) -> SerialPortSummary {
        SerialPortSummary {
            name: "COM3".to_string(),
            port_type: "USB 0403:6001 USB Serial Port".to_string(),
            usb_vid: Some(0x0403),
            usb_pid: Some(0x6001),
            serial_number: Some(serial_number.to_string()),
            manufacturer: Some("FTDI".to_string()),
            product: Some("USB Serial Port".to_string()),
            windows_device_instance_id: None,
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
        let writes = Arc::new(Mutex::new(Vec::new()));
        let (before_gate_entered_tx, before_gate_entered_rx) = mpsc::channel();
        let (release_before_gate_tx, release_before_gate_rx) = mpsc::channel();
        let block_once = Arc::new(AtomicBool::new(true));

        let worker = thread::spawn({
            let frames = Arc::clone(&frames);
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            let error = Arc::clone(&error);
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
        let writes = Arc::new(Mutex::new(Vec::new()));
        let (physical_entered_tx, physical_entered_rx) = mpsc::channel();
        let (physical_release_tx, physical_release_rx) = mpsc::channel();

        let worker = thread::spawn({
            let frames = Arc::clone(&frames);
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            let error = Arc::clone(&error);
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
        assert!(s0_done_rx.recv_timeout(Duration::from_millis(25)).is_err());
        physical_release_tx
            .send(())
            .expect("worker must still be in its first physical write");
        s0_done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("S0 must complete after the single winning write")
            .expect("S0 gate must not be poisoned");
        s0.join().expect("S0 reservation thread must terminate");
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
    fn open_dmx_pacing_waits_the_full_frame_budget_from_a_fast_write() {
        let wait = open_dmx_frame_pacing_wait(Duration::ZERO);
        assert_eq!(
            wait,
            Duration::from_micros(ENTTEC_OPEN_DMX_FRAME_WIRE_US + ENTTEC_OPEN_DMX_FRAME_GUARD_US)
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
    fn verified_windows_com_handle_rejects_an_a_to_b_to_a_swap_without_port_name_trust() {
        let before_open = verified_usb_port("FTDI-INSTANCE-A");
        let mut identity = VerifiedUsbSerialPortIdentity::from_summary(&before_open)
            .expect("a complete enumerated USB identity must be capturable");
        let device_a = r"FTDIBUS\VID_0403+PID_6001+FTDI-INSTANCE-A\0000";
        identity.windows_device_instance_id = Some(device_a.to_string());
        let pre_open_a = windows_binding(device_a, "FTDI-INSTANCE-A");
        let opened_a = pre_open_a.clone();
        let post_open_a = pre_open_a.clone();

        assert!(verify_opened_windows_com_handle_binding(
            &identity,
            &pre_open_a,
            &opened_a,
            &post_open_a,
        )
        .is_ok());

        // A→B→A: the normal COM alias is back on A by the post-open check,
        // but the captured handle was opened on B. The comparison is between
        // PnP instance/interface identities, not `SerialPort::name()`.
        let device_b = r"FTDIBUS\VID_0403+PID_6001+FTDI-INSTANCE-B\0000";
        let opened_b = windows_binding(device_b, "FTDI-INSTANCE-B");
        assert!(verify_opened_windows_com_handle_binding(
            &identity,
            &pre_open_a,
            &opened_b,
            &post_open_a,
        )
        .is_err());

        let post_open_b = windows_binding(device_b, "FTDI-INSTANCE-B");
        assert!(verify_opened_windows_com_handle_binding(
            &identity,
            &pre_open_a,
            &opened_a,
            &post_open_b,
        )
        .is_err());

        let mut missing_instance = before_open;
        missing_instance.serial_number = Some(" ".to_string());
        assert!(matches!(
            VerifiedUsbSerialPortIdentity::from_summary(&missing_instance),
            Err(SerialDmxError::Identity(_))
        ));
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
