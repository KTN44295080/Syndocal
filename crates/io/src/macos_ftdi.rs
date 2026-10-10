//! Driver-free FT232R transport. The serial adapter delegates the existing
//! Open DMX worker's BREAK/MAB/write/flush lifecycle without extra frame copies.

const FT232R_BAUD_DIVISOR: u16 = 12; // 3 MHz / 250000 baud
const FT232R_8N2: u16 = 0x1008;
const FT232R_BREAK: u16 = FT232R_8N2 | 0x4000;

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};
    use std::{
        ffi::c_void,
        io::{self, Read, Write},
        ptr::NonNull,
        time::{Duration, Instant},
    };

    unsafe extern "C" {
        fn syndocal_ftdi_open(registry_id: u64, status: *mut i32) -> *mut c_void;
        fn syndocal_ftdi_close(port: *mut c_void);
        fn syndocal_ftdi_control(
            port: *mut c_void,
            direction: u8,
            request: u8,
            value: u16,
            index: u16,
            data: *mut c_void,
            size: u16,
            timeout: u32,
        ) -> i32;
        fn syndocal_ftdi_write(
            port: *mut c_void,
            data: *const c_void,
            size: u32,
            timeout: u32,
        ) -> i32;
    }

    pub(crate) struct FtdiTransport {
        handle: NonNull<c_void>,
        name: String,
        timeout: Duration,
    }
    // One exclusive USB handle moves into one worker. It is never cloned,
    // shared, accessed concurrently, or dropped before that worker is reaped.
    unsafe impl Send for FtdiTransport {}
    impl Drop for FtdiTransport {
        fn drop(&mut self) {
            unsafe {
                syndocal_ftdi_close(self.handle.as_ptr());
            }
        }
    }
    fn unsupported() -> serialport::Error {
        serialport::Error::new(
            serialport::ErrorKind::Unknown,
            "FT232R USB-DMX is an exclusive output-only 250000/8N2 transport",
        )
    }
    fn checked(status: i32) -> io::Result<()> {
        if status == 0 {
            Ok(())
        } else {
            Err(io::Error::other(format!("macOS FTDI USB operation failed (IOKit {status:#010x}); close QLC+ and reconnect the selected device")))
        }
    }
    impl FtdiTransport {
        pub(crate) fn open(registry_id: u64, name: &str) -> io::Result<Self> {
            let mut status = 0;
            let handle = NonNull::new(unsafe { syndocal_ftdi_open(registry_id, &mut status) });
            checked(status)?;
            Ok(Self {
                handle: handle.ok_or_else(|| io::Error::other("IOKit returned no USB handle"))?,
                name: name.into(),
                timeout: Duration::from_millis(100),
            })
        }
        fn timeout_ms(&self) -> u32 {
            self.timeout.as_millis().clamp(1, u32::MAX as u128) as u32
        }
        fn control(&self, request: u8, value: u16, index: u16) -> io::Result<()> {
            checked(unsafe {
                syndocal_ftdi_control(
                    self.handle.as_ptr(),
                    0x40,
                    request,
                    value,
                    index,
                    std::ptr::null_mut(),
                    0,
                    self.timeout_ms(),
                )
            })
        }
        // Called only after exact opened-device identity has been revalidated.
        pub(crate) fn configure(&self) -> io::Result<()> {
            self.control(0, 0, 1)?; // reset UART
            self.control(3, FT232R_BAUD_DIVISOR, 0)?;
            self.control(4, FT232R_8N2, 1)?;
            self.control(2, 0, 1)?; // no flow control
            self.control(1, 0x0100, 1)?; // DTR off
            self.control(1, 0x0200, 1)?; // RTS off
            self.control(9, 2, 1)?; // 2ms latency
            Ok(())
        }
    }
    impl Write for FtdiTransport {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            let size =
                u32::try_from(data.len()).map_err(|_| io::Error::other("USB write exceeds u32"))?;
            checked(unsafe {
                syndocal_ftdi_write(
                    self.handle.as_ptr(),
                    data.as_ptr().cast(),
                    size,
                    self.timeout_ms(),
                )
            })?;
            Ok(data.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            let deadline = Instant::now() + self.timeout;
            loop {
                let mut status = [0u8; 2];
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "FTDI UART drain did not complete",
                    ));
                }
                let timeout = remaining.as_millis().clamp(1, 10) as u32;
                checked(unsafe {
                    syndocal_ftdi_control(
                        self.handle.as_ptr(),
                        0xc0,
                        5,
                        0,
                        1,
                        status.as_mut_ptr().cast(),
                        2,
                        timeout,
                    )
                })?;
                if status[1] & 0x40 != 0 {
                    return Ok(());
                } // transmitter empty
                std::thread::sleep(Duration::from_micros(250));
            }
        }
    }
    impl Read for FtdiTransport {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "USB-DMX input is not enabled",
            ))
        }
    }
    impl SerialPort for FtdiTransport {
        fn name(&self) -> Option<String> {
            Some(self.name.clone())
        }
        fn baud_rate(&self) -> serialport::Result<u32> {
            Ok(250000)
        }
        fn data_bits(&self) -> serialport::Result<DataBits> {
            Ok(DataBits::Eight)
        }
        fn flow_control(&self) -> serialport::Result<FlowControl> {
            Ok(FlowControl::None)
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
        fn set_baud_rate(&mut self, v: u32) -> serialport::Result<()> {
            if v == 250000 {
                Ok(())
            } else {
                Err(unsupported())
            }
        }
        fn set_data_bits(&mut self, v: DataBits) -> serialport::Result<()> {
            if v == DataBits::Eight {
                Ok(())
            } else {
                Err(unsupported())
            }
        }
        fn set_flow_control(&mut self, v: FlowControl) -> serialport::Result<()> {
            if v == FlowControl::None {
                Ok(())
            } else {
                Err(unsupported())
            }
        }
        fn set_parity(&mut self, v: Parity) -> serialport::Result<()> {
            if v == Parity::None {
                Ok(())
            } else {
                Err(unsupported())
            }
        }
        fn set_stop_bits(&mut self, v: StopBits) -> serialport::Result<()> {
            if v == StopBits::Two {
                Ok(())
            } else {
                Err(unsupported())
            }
        }
        fn set_timeout(&mut self, v: Duration) -> serialport::Result<()> {
            if v.is_zero() {
                Err(unsupported())
            } else {
                self.timeout = v;
                Ok(())
            }
        }
        fn write_request_to_send(&mut self, _: bool) -> serialport::Result<()> {
            Err(unsupported())
        }
        fn write_data_terminal_ready(&mut self, _: bool) -> serialport::Result<()> {
            Err(unsupported())
        }
        fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
            Err(unsupported())
        }
        fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
            Err(unsupported())
        }
        fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
            Err(unsupported())
        }
        fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
            Err(unsupported())
        }
        fn bytes_to_read(&self) -> serialport::Result<u32> {
            Err(unsupported())
        }
        fn bytes_to_write(&self) -> serialport::Result<u32> {
            Err(unsupported())
        }
        fn clear(&self, _: ClearBuffer) -> serialport::Result<()> {
            Err(unsupported())
        }
        fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
            Err(unsupported())
        }
        fn set_break(&self) -> serialport::Result<()> {
            self.control(4, FT232R_BREAK, 1).map_err(Into::into)
        }
        fn clear_break(&self) -> serialport::Result<()> {
            self.control(4, FT232R_8N2, 1).map_err(Into::into)
        }
    }
}
#[cfg(target_os = "macos")]
pub(crate) use native::FtdiTransport;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ft232r_usb_uart_encoding_is_250k_8n2_with_explicit_break_bit() {
        assert_eq!(3_000_000 / u32::from(FT232R_BAUD_DIVISOR), 250000);
        assert_eq!(FT232R_8N2 & 0xff, 8);
        assert_eq!(FT232R_8N2 >> 12, 1);
        assert_eq!(FT232R_BREAK ^ FT232R_8N2, 0x4000);
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn native_usb_open_rejects_a_missing_registry_generation_without_transfers() {
        assert!(native::FtdiTransport::open(u64::MAX, "usb-ftdi://ffffffffffffffff").is_err());
    }
}
