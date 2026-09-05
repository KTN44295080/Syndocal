use super::{ENTTEC_OPEN_DMX_BAUD_RATE, ENTTEC_OPEN_DMX_WRITE_TIMEOUT_MS};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Observable operations emitted by the test-support fake serial port. The
/// payload is retained so an engine integration test can prove that S0 chose
/// an all-zero DMX frame before the real `write_all` path.
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenDmxTestSerialOperation {
    SetBreak,
    ClearBreak,
    WriteAll(Vec<u8>),
    Flush,
}

#[derive(Default)]
struct OpenDmxTestSerialState {
    operations: Vec<OpenDmxTestSerialOperation>,
    fail_next_write: bool,
    zero_next_write: bool,
}

/// Test-support `SerialPort` implementation used to exercise the actual
/// Open DMX worker transaction without a physical COM interface.
#[doc(hidden)]
#[derive(Clone)]
pub struct OpenDmxTestSerialPort {
    state: Arc<Mutex<OpenDmxTestSerialState>>,
    baud_rate: u32,
    timeout: Duration,
    before_write_all: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[doc(hidden)]
#[derive(Clone)]
pub struct OpenDmxTestSerialObservation {
    state: Arc<Mutex<OpenDmxTestSerialState>>,
}

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
                timeout: Duration::from_millis(ENTTEC_OPEN_DMX_WRITE_TIMEOUT_MS),
                before_write_all: Some(Arc::new(before_write_all)),
            },
            OpenDmxTestSerialObservation { state },
        )
    }
}

impl OpenDmxTestSerialObservation {
    pub fn operations(&self) -> Result<Vec<OpenDmxTestSerialOperation>, String> {
        self.state
            .lock()
            .map(|state| state.operations.clone())
            .map_err(|_| "Open DMX test serial observation was poisoned".to_string())
    }

    /// Cause the next real worker `write_all` to fail. This stays behind the
    /// test-support fake-port seam so the engine can prove its disconnect/fault
    /// path without opening a real COM interface.
    pub fn fail_next_write(&self) -> Result<(), String> {
        self.state
            .lock()
            .map(|mut state| state.fail_next_write = true)
            .map_err(|_| "Open DMX test serial observation was poisoned".to_string())
    }

    /// Cause the next worker write to report `Ok(0)`, which is the exact
    /// timeout/backpressure shape that `Write::write_all` promotes to
    /// `ErrorKind::WriteZero`.
    pub fn zero_next_write(&self) -> Result<(), String> {
        self.state
            .lock()
            .map(|mut state| state.zero_next_write = true)
            .map_err(|_| "Open DMX test serial observation was poisoned".to_string())
    }
}

impl io::Read for OpenDmxTestSerialPort {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::WouldBlock))
    }
}

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
        if state.zero_next_write {
            state.zero_next_write = false;
            return Ok(0);
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
