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
    #[error("failed to write serial DMX packet: {0}")]
    Write(#[source] io::Error),
    #[error("failed to start Open DMX worker: {0}")]
    WorkerStart(#[source] io::Error),
    #[error("Open DMX worker stopped: {0}")]
    Worker(String),
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
                move || {
                    run_enttec_open_dmx_worker(port, frames, running, worker_failed, worker_error)
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
                    SerialPortSummary {
                        name: port.port_name,
                        port_type: serial_port_type_label(&port.port_type),
                        usb_vid,
                        usb_pid,
                        serial_number,
                        manufacturer,
                        product,
                        recommended_protocol,
                    }
                })
                .collect()
        })
}

fn serial_port_usb_metadata(
    port_type: &SerialPortType,
) -> (
    Option<u16>,
    Option<u16>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
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

fn run_enttec_open_dmx_worker(
    mut port: Box<dyn SerialPort>,
    frames: Arc<ArrayQueue<[u8; 512]>>,
    running: Arc<AtomicBool>,
    worker_failed: Arc<AtomicBool>,
    worker_error: Arc<Mutex<Option<String>>>,
) {
    let mut current_frame = None;
    while running.load(Ordering::Acquire) {
        while let Some(frame) = frames.pop() {
            current_frame = Some(frame);
        }
        let Some(frame) = current_frame.as_ref() else {
            thread::sleep(Duration::from_millis(1));
            continue;
        };
        if let Err(error) = write_enttec_open_dmx_frame(&mut *port, frame) {
            if let Ok(mut slot) = worker_error.lock() {
                *slot = Some(error.to_string());
            }
            worker_failed.store(true, Ordering::Release);
            running.store(false, Ordering::Release);
        }
    }
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
    io::Error::new(io::ErrorKind::Other, error.to_string())
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
