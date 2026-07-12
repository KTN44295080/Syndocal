use std::{
    collections::HashMap,
    ffi::OsString,
    io::Read,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use protocol::VideoLayerId;

pub const CAPTURE_WIDTH: u32 = 1280;
pub const CAPTURE_HEIGHT: u32 = 720;
pub const CAPTURE_FRAME_RATE: u32 = 30;

pub type CaptureInputRegistry = Arc<Mutex<HashMap<VideoLayerId, video::VideoFrame>>>;

pub struct CaptureTransportState {
    inputs: CaptureInputRegistry,
    workers: HashMap<VideoLayerId, CaptureRouteWorker>,
}

impl CaptureTransportState {
    pub fn new(inputs: CaptureInputRegistry) -> Self {
        Self {
            inputs,
            workers: HashMap::new(),
        }
    }

    pub fn start_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        if route.direction != video::ExternalVideoTransportDirection::Input {
            return Err(driver_error("Capture backends support input routes only"));
        }
        let worker = CaptureRouteWorker::start(
            route.route_id,
            &route.backend_id,
            &route.endpoint_name,
            Arc::clone(&self.inputs),
        )?;
        if let Some(previous) = self.workers.insert(route.route_id, worker) {
            previous.stop();
        }
        Ok(())
    }

    pub fn stop_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        if let Some(worker) = self.workers.remove(&route.route_id) {
            worker.stop();
        }
        self.inputs
            .lock()
            .map_err(|_| driver_error("Capture input frame registry lock was poisoned"))?
            .remove(&route.route_id);
        Ok(())
    }
}

struct CaptureRouteWorker {
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl CaptureRouteWorker {
    fn start(
        layer_id: VideoLayerId,
        backend_id: &str,
        endpoint_name: &str,
        frames: CaptureInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let args = capture_ffmpeg_args(backend_id, endpoint_name)?;
        let mut process = Command::new(&ffmpeg)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                driver_error(format!(
                    "Failed to start FFmpeg {} capture '{}': {error}",
                    capture_kind_label(backend_id),
                    endpoint_name
                ))
            })?;
        let mut stdout = process.stdout.take().ok_or_else(|| {
            driver_error("FFmpeg capture did not provide a raw-video stdout pipe")
        })?;
        let child = Arc::new(Mutex::new(Some(process)));
        let worker_child = Arc::clone(&child);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let endpoint = endpoint_name.to_string();
        let backend = backend_id.to_string();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name(format!("syndocal-capture-input-{layer_id}"))
            .spawn(move || {
                let frame_bytes = (CAPTURE_WIDTH * CAPTURE_HEIGHT * 4) as usize;
                let mut pixels = vec![0_u8; frame_bytes];
                let mut ready = false;
                while !worker_stop.load(Ordering::Acquire) {
                    if let Err(error) = stdout.read_exact(&mut pixels) {
                        if !ready {
                            let status = worker_child
                                .lock()
                                .ok()
                                .and_then(|mut child| child.as_mut()?.try_wait().ok().flatten())
                                .map(|status| format!("; FFmpeg exited with {status}"))
                                .unwrap_or_default();
                            let _ = ready_tx.send(Err(format!(
                                "FFmpeg {} capture '{}' produced no complete frame: {error}{status}",
                                capture_kind_label(&backend),
                                endpoint
                            )));
                        }
                        break;
                    }
                    if let Ok(mut registry) = frames.lock() {
                        let frame = video::VideoFrame {
                            layer_id,
                            width: CAPTURE_WIDTH,
                            height: CAPTURE_HEIGHT,
                            pts_ms: 0,
                            duration_ms: 1000 / CAPTURE_FRAME_RATE as u64,
                            format: video::VideoPixelFormat::Rgba8,
                            data: pixels,
                        };
                        pixels = registry
                            .insert(layer_id, frame)
                            .map(|previous| previous.data)
                            .unwrap_or_else(|| vec![0_u8; frame_bytes]);
                        pixels.resize(frame_bytes, 0);
                    }
                    if !ready {
                        ready = true;
                        let _ = ready_tx.send(Ok(()));
                    }
                }
                terminate_child(&worker_child);
            })
            .map_err(|error| driver_error(format!("Failed to start capture worker: {error}")))?;

        match ready_rx.recv_timeout(Duration::from_secs(7)) {
            Ok(Ok(())) => Ok(Self {
                stop,
                child,
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                terminate_child(&child);
                let _ = worker.join();
                Err(driver_error(error))
            }
            Err(_) => {
                stop.store(true, Ordering::Release);
                terminate_child(&child);
                let _ = worker.join();
                Err(driver_error(format!(
                    "Timed out waiting for the first FFmpeg {} capture frame from '{}'",
                    capture_kind_label(backend_id),
                    endpoint_name
                )))
            }
        }
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        terminate_child(&self.child);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for CaptureRouteWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        terminate_child(&self.child);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn terminate_child(child: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut child) = child.lock() {
        if let Some(mut process) = child.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

fn capture_kind_label(backend_id: &str) -> &'static str {
    if backend_id == "camera" {
        "camera"
    } else {
        "screen"
    }
}

pub fn default_capture_endpoint(backend_id: &str) -> &'static str {
    match (backend_id, std::env::consts::OS) {
        ("camera", "windows") => "video=default",
        ("camera", "macos") => "0",
        ("camera", _) => "/dev/video0",
        ("screen_capture", "windows") => "desktop",
        ("screen_capture", "macos") => "1",
        ("screen_capture", _) => ":0.0",
        _ => "",
    }
}

pub fn capture_ffmpeg_args(
    backend_id: &str,
    endpoint_name: &str,
) -> Result<Vec<OsString>, video::ExternalVideoTransportDriverError> {
    let endpoint = endpoint_name.trim();
    let endpoint = if endpoint.is_empty() {
        default_capture_endpoint(backend_id)
    } else {
        endpoint
    };
    let mut args = vec![
        OsString::from("-nostdin"),
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-thread_queue_size"),
        OsString::from("8"),
    ];
    match (backend_id, std::env::consts::OS) {
        ("camera", "windows") => {
            args.extend(os_args(["-f", "dshow", "-i"]));
            args.push(OsString::from(if endpoint.starts_with("video=") {
                endpoint.to_string()
            } else {
                format!("video={endpoint}")
            }));
        }
        ("screen_capture", "windows") => {
            args.extend(os_args([
                "-f",
                "gdigrab",
                "-framerate",
                "30",
                "-draw_mouse",
                "1",
                "-i",
            ]));
            args.push(OsString::from(endpoint));
        }
        ("camera", "macos") | ("screen_capture", "macos") => {
            args.extend(os_args(["-f", "avfoundation", "-framerate", "30", "-i"]));
            args.push(OsString::from(if endpoint.contains(':') {
                endpoint.to_string()
            } else {
                format!("{endpoint}:none")
            }));
        }
        ("camera", "linux") => {
            args.extend(os_args(["-f", "v4l2", "-framerate", "30", "-i"]));
            args.push(OsString::from(endpoint));
        }
        ("screen_capture", "linux") => {
            args.extend(os_args([
                "-f",
                "x11grab",
                "-framerate",
                "30",
                "-draw_mouse",
                "1",
                "-i",
            ]));
            args.push(OsString::from(endpoint));
        }
        ("camera" | "screen_capture", platform) => {
            return Err(driver_error(format!(
                "FFmpeg {} capture is not configured for platform '{platform}'",
                capture_kind_label(backend_id)
            )));
        }
        _ => {
            return Err(driver_error(format!(
                "Unknown capture backend '{backend_id}'"
            )))
        }
    }
    args.extend(os_args([
        "-an",
        "-vf",
        "fps=30,scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2",
        "-pix_fmt",
        "rgba",
        "-f",
        "rawvideo",
        "pipe:1",
    ]));
    Ok(args)
}

fn os_args<const N: usize>(values: [&str; N]) -> Vec<OsString> {
    values.into_iter().map(OsString::from).collect()
}

fn driver_error(message: impl Into<String>) -> video::ExternalVideoTransportDriverError {
    video::ExternalVideoTransportDriverError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg_strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn capture_commands_are_persistent_raw_rgba_pipelines() {
        for backend in ["camera", "screen_capture"] {
            let args = arg_strings(&capture_ffmpeg_args(backend, "sample endpoint").unwrap());
            assert_eq!(args.last().map(String::as_str), Some("pipe:1"));
            assert!(args.windows(2).any(|pair| pair == ["-pix_fmt", "rgba"]));
            assert!(args.iter().any(|arg| arg.contains("scale=1280:720")));
            assert!(args.iter().any(|arg| arg.contains("sample endpoint")));
        }
    }

    #[test]
    fn capture_commands_reject_unknown_backends() {
        assert!(capture_ffmpeg_args("unknown", "source").is_err());
    }

    #[test]
    #[ignore = "requires FFmpeg and a visible desktop capture session"]
    fn screen_worker_captures_a_real_frame() {
        let frames = Arc::new(Mutex::new(HashMap::new()));
        let worker = CaptureRouteWorker::start(
            77,
            "screen_capture",
            default_capture_endpoint("screen_capture"),
            Arc::clone(&frames),
        )
        .unwrap();
        let frame = frames.lock().unwrap().get(&77).cloned().unwrap();
        assert_eq!((frame.width, frame.height), (CAPTURE_WIDTH, CAPTURE_HEIGHT));
        assert_eq!(
            frame.data.len(),
            (CAPTURE_WIDTH * CAPTURE_HEIGHT * 4) as usize
        );
        worker.stop();
    }
}
