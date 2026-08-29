use std::{
    collections::HashMap,
    ffi::OsString,
    io::Read,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use protocol::VideoLayerId;

pub const SCREEN_CAPTURE_WIDTH: u32 = 1280;
pub const SCREEN_CAPTURE_HEIGHT: u32 = 720;
pub const SCREEN_CAPTURE_FRAME_RATE: u32 = 30;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptureFrameSpec {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frame_rate_numerator: u32,
    pub(crate) frame_rate_denominator: u32,
}

impl CaptureFrameSpec {
    fn duration_ms(self) -> u64 {
        let milliseconds = (1000_u128 * u128::from(self.frame_rate_denominator))
            / u128::from(self.frame_rate_numerator);
        u64::try_from(milliseconds).unwrap_or(u64::MAX).max(1)
    }
}

pub(crate) struct CaptureFfmpegPlan {
    pub(crate) args: Vec<OsString>,
    pub(crate) frame: CaptureFrameSpec,
}

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
        // Stop an existing route before its replacement starts.  This keeps a
        // failing old worker from evicting the new worker's latest frame.
        if let Some(previous) = self.workers.remove(&route.route_id) {
            previous.stop();
        }
        self.inputs
            .lock()
            .map_err(|_| driver_error("Capture input frame registry lock was poisoned"))?
            .remove(&route.route_id);
        let worker = CaptureRouteWorker::start(
            route.route_id,
            &route.backend_id,
            &route.endpoint_name,
            Arc::clone(&self.inputs),
        )?;
        self.workers.insert(route.route_id, worker);
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

    #[cfg(test)]
    pub(crate) fn with_post_ready_fault_for_test(
        inputs: CaptureInputRegistry,
        route_id: VideoLayerId,
        message: &str,
    ) -> Self {
        let mut state = Self::new(inputs);
        state
            .workers
            .insert(route_id, CaptureRouteWorker::faulted_for_test(message));
        state
    }
}

struct CaptureRouteWorker {
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    post_ready_fault: Arc<Mutex<Option<String>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl CaptureRouteWorker {
    fn start(
        layer_id: VideoLayerId,
        backend_id: &str,
        endpoint_name: &str,
        frames: CaptureInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let ffmpeg = crate::capture_catalog::verified_ffmpeg_executable().map_err(driver_error)?;
        let plan = capture_ffmpeg_plan(backend_id, endpoint_name)?;
        let mut process = Command::new(&ffmpeg)
            .args(&plan.args)
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
        let mut stdout = match process.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_process(&mut process);
                return Err(driver_error(
                    "FFmpeg capture did not provide a raw-video stdout pipe",
                ));
            }
        };
        let child = Arc::new(Mutex::new(Some(process)));
        let worker_child = Arc::clone(&child);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let post_ready_fault = Arc::new(Mutex::new(None));
        let worker_post_ready_fault = Arc::clone(&post_ready_fault);
        let endpoint = endpoint_name.to_string();
        let backend = backend_id.to_string();
        let frame = plan.frame;
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = match std::thread::Builder::new()
            .name(format!("syndocal-capture-input-{layer_id}"))
            .spawn(move || {
                let frame_bytes = match crate::capture_catalog::checked_frame_bytes(
                    frame.width,
                    frame.height,
                ) {
                    Ok(frame_bytes) => frame_bytes,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        terminate_child(&worker_child);
                        return;
                    }
                };
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
                        } else if !worker_stop.load(Ordering::Acquire) {
                            let fault = format!(
                                "FFmpeg {} capture '{}' failed after readiness: {error}",
                                capture_kind_label(&backend),
                                endpoint
                            );
                            if let Err(error) = record_post_ready_capture_fault(
                                &frames,
                                &worker_post_ready_fault,
                                layer_id,
                                fault,
                            ) {
                                eprintln!("Capture route {layer_id} fault publication failed: {error}");
                            }
                        }
                        break;
                    }
                    let mut registry = match frames.lock() {
                        Ok(registry) => registry,
                        Err(_) => {
                            let fault = "Capture input frame registry lock was poisoned".to_string();
                            if !ready {
                                let _ = ready_tx.send(Err(fault));
                            } else if !worker_stop.load(Ordering::Acquire) {
                                if let Err(error) = record_post_ready_capture_fault(
                                    &frames,
                                    &worker_post_ready_fault,
                                    layer_id,
                                    fault,
                                ) {
                                    eprintln!("Capture route {layer_id} fault publication failed: {error}");
                                }
                            }
                            break;
                        }
                    };
                    let frame = video::VideoFrame {
                        layer_id,
                        width: frame.width,
                        height: frame.height,
                        pts_ms: 0,
                        duration_ms: frame.duration_ms(),
                        format: video::VideoPixelFormat::Rgba8,
                        data: pixels,
                    };
                    pixels = registry
                        .insert(layer_id, frame)
                        .map(|previous| previous.data)
                        .unwrap_or_else(|| vec![0_u8; frame_bytes]);
                    pixels.resize(frame_bytes, 0);
                    drop(registry);
                    if !ready {
                        ready = true;
                        let _ = ready_tx.send(Ok(()));
                    }
                }
                terminate_child(&worker_child);
            })
        {
            Ok(worker) => worker,
            Err(error) => {
                stop.store(true, Ordering::Release);
                terminate_child(&child);
                return Err(driver_error(format!(
                    "Failed to start capture worker: {error}"
                )));
            }
        };

        match ready_rx.recv_timeout(Duration::from_secs(7)) {
            Ok(Ok(())) => Ok(Self {
                stop,
                child,
                post_ready_fault,
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

    fn current_post_ready_fault(&self) -> Result<Option<String>, String> {
        self.post_ready_fault
            .lock()
            .map(|fault| fault.clone())
            .map_err(|_| "Capture route post-ready fault state lock was poisoned".to_string())
    }

    #[cfg(test)]
    fn faulted_for_test(message: &str) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(true)),
            child: Arc::new(Mutex::new(None)),
            post_ready_fault: Arc::new(Mutex::new(Some(message.to_string()))),
            worker: None,
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
            terminate_process(&mut process);
        }
    }
}

fn terminate_process(process: &mut Child) {
    let _ = process.kill();
    let _ = process.wait();
}

fn record_post_ready_capture_fault(
    frames: &CaptureInputRegistry,
    fault_state: &Arc<Mutex<Option<String>>>,
    layer_id: VideoLayerId,
    fault: String,
) -> Result<(), String> {
    // A capture route owns exactly one replaceable frame per layer.  Evict it
    // before publishing the fault so no consumer can continue showing a frozen
    // camera image after FFmpeg has stopped producing frames.
    let eviction_error = match frames.lock() {
        Ok(mut registry) => {
            registry.remove(&layer_id);
            None
        }
        Err(_) => Some(
            "Capture input frame registry lock was poisoned; latest frame could not be evicted"
                .to_string(),
        ),
    };
    let fault = match &eviction_error {
        Some(eviction_error) => format!("{fault}; {eviction_error}"),
        None => fault,
    };
    let mut current = fault_state
        .lock()
        .map_err(|_| "Capture route post-ready fault state lock was poisoned".to_string())?;
    *current = Some(fault);
    if let Some(eviction_error) = eviction_error {
        return Err(eviction_error);
    }
    Ok(())
}

fn capture_kind_label(backend_id: &str) -> &'static str {
    if backend_id == "camera" {
        "camera"
    } else {
        "screen"
    }
}

impl CaptureTransportState {
    /// Returns the retained fault for an active route without draining it.
    /// Status polling therefore cannot turn a failed capture back into a
    /// silently-active route.  Stop/replacement removes the worker and its
    /// fault together.
    pub fn current_route_fault(&self, route_id: VideoLayerId) -> Result<Option<String>, String> {
        self.workers
            .get(&route_id)
            .map(CaptureRouteWorker::current_post_ready_fault)
            .unwrap_or(Ok(None))
    }
}

pub fn default_capture_endpoint(backend_id: &str) -> &'static str {
    match (backend_id, std::env::consts::OS) {
        ("screen_capture", "windows") => "desktop",
        ("screen_capture", "macos") => "1",
        ("screen_capture", _) => ":0.0",
        _ => "",
    }
}

#[cfg(test)]
pub fn capture_ffmpeg_args(
    backend_id: &str,
    endpoint_name: &str,
) -> Result<Vec<OsString>, video::ExternalVideoTransportDriverError> {
    Ok(capture_ffmpeg_plan(backend_id, endpoint_name)?.args)
}

pub(crate) fn capture_ffmpeg_plan(
    backend_id: &str,
    endpoint_name: &str,
) -> Result<CaptureFfmpegPlan, video::ExternalVideoTransportDriverError> {
    let mut args = vec![
        OsString::from("-nostdin"),
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-thread_queue_size"),
        OsString::from("8"),
    ];
    let frame = match (backend_id, std::env::consts::OS) {
        ("camera", "windows") => {
            let selection = crate::capture_catalog::canonical_camera_selection(endpoint_name)
                .map_err(driver_error)?;
            args.extend(os_args(["-f", "dshow", "-video_size"]));
            args.push(OsString::from(format!(
                "{}x{}",
                selection.width, selection.height
            )));
            args.extend(os_args(["-framerate"]));
            args.push(OsString::from(format!(
                "{}/{}",
                selection.frame_rate_numerator, selection.frame_rate_denominator
            )));
            match selection.input_format {
                crate::capture_catalog::CameraInputFormat::Vcodec(value) => {
                    args.extend(os_args(["-vcodec"]));
                    args.push(OsString::from(value));
                }
                crate::capture_catalog::CameraInputFormat::PixelFormat(value) => {
                    args.extend(os_args(["-pixel_format"]));
                    args.push(OsString::from(value));
                }
            }
            args.extend(os_args(["-i"]));
            args.push(OsString::from(format!(
                "video={}",
                selection.device_alternative_name
            )));
            CaptureFrameSpec {
                width: selection.width,
                height: selection.height,
                frame_rate_numerator: selection.frame_rate_numerator,
                frame_rate_denominator: selection.frame_rate_denominator,
            }
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
            let endpoint = normalized_screen_capture_endpoint(endpoint_name);
            args.push(OsString::from(endpoint));
            CaptureFrameSpec {
                width: SCREEN_CAPTURE_WIDTH,
                height: SCREEN_CAPTURE_HEIGHT,
                frame_rate_numerator: SCREEN_CAPTURE_FRAME_RATE,
                frame_rate_denominator: 1,
            }
        }
        ("screen_capture", "macos") => {
            let endpoint = normalized_screen_capture_endpoint(endpoint_name);
            args.extend(os_args(["-f", "avfoundation", "-framerate", "30", "-i"]));
            args.push(OsString::from(if endpoint.contains(':') {
                endpoint.to_string()
            } else {
                format!("{endpoint}:none")
            }));
            CaptureFrameSpec {
                width: SCREEN_CAPTURE_WIDTH,
                height: SCREEN_CAPTURE_HEIGHT,
                frame_rate_numerator: SCREEN_CAPTURE_FRAME_RATE,
                frame_rate_denominator: 1,
            }
        }
        ("screen_capture", "linux") => {
            let endpoint = normalized_screen_capture_endpoint(endpoint_name);
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
            CaptureFrameSpec {
                width: SCREEN_CAPTURE_WIDTH,
                height: SCREEN_CAPTURE_HEIGHT,
                frame_rate_numerator: SCREEN_CAPTURE_FRAME_RATE,
                frame_rate_denominator: 1,
            }
        }
        ("camera", platform) => {
            return Err(driver_error(format!(
                "Exact camera-profile capture is currently supported only on Windows DirectShow, not '{platform}'"
            )));
        }
        ("screen_capture", platform) => {
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
    };
    args.extend(os_args(["-an"]));
    if backend_id == "screen_capture" {
        args.extend(os_args([
            "-vf",
            "fps=30,scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2",
        ]));
    }
    args.extend(os_args(["-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]));
    Ok(CaptureFfmpegPlan { args, frame })
}

fn normalized_screen_capture_endpoint(endpoint_name: &str) -> &str {
    let endpoint = endpoint_name.trim();
    if endpoint.is_empty() {
        default_capture_endpoint("screen_capture")
    } else {
        endpoint
    }
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
    fn screen_capture_commands_remain_persistent_1280x720_raw_rgba_pipelines() {
        let args = arg_strings(&capture_ffmpeg_args("screen_capture", "sample endpoint").unwrap());
        assert_eq!(args.last().map(String::as_str), Some("pipe:1"));
        assert!(args.windows(2).any(|pair| pair == ["-pix_fmt", "rgba"]));
        assert!(args.iter().any(|arg| arg.contains("scale=1280:720")));
        assert!(args.iter().any(|arg| arg.contains("sample endpoint")));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn camera_commands_require_an_opaque_exact_profile_without_scaling() {
        let endpoint = crate::capture_catalog::canonical_camera_endpoint(
            &crate::capture_catalog::CanonicalCameraSelection {
                device_alternative_name: "@device_pnp_\\\\?\\usb#camera-a".to_string(),
                input_format: crate::capture_catalog::CameraInputFormat::Vcodec(
                    "mjpeg".to_string(),
                ),
                width: 1920,
                height: 1080,
                frame_rate_numerator: 60,
                frame_rate_denominator: 1,
            },
        )
        .unwrap();
        let plan = capture_ffmpeg_plan("camera", &endpoint).unwrap();
        let args = arg_strings(&plan.args);
        assert_eq!(args.last().map(String::as_str), Some("pipe:1"));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-video_size", "1920x1080"]));
        assert!(args.windows(2).any(|pair| pair == ["-framerate", "60/1"]));
        assert!(args.windows(2).any(|pair| pair == ["-vcodec", "mjpeg"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-i", "video=@device_pnp_\\\\?\\usb#camera-a"]));
        assert!(!args.iter().any(|arg| arg.starts_with("fps=30,scale=")));
        assert_eq!((plan.frame.width, plan.frame.height), (1920, 1080));
        assert_eq!(plan.frame.duration_ms(), 16);
        assert!(capture_ffmpeg_args("camera", "Camera A").is_err());
        assert!(capture_ffmpeg_args("camera", "").is_err());
    }

    #[test]
    fn capture_commands_reject_unknown_backends() {
        assert!(capture_ffmpeg_args("unknown", "source").is_err());
    }

    #[test]
    fn post_ready_capture_failure_evicts_the_latest_frame_and_retains_a_route_fault() {
        let frames = Arc::new(Mutex::new(HashMap::from([(
            77,
            video::VideoFrame {
                layer_id: 77,
                width: 1,
                height: 1,
                pts_ms: 0,
                duration_ms: 33,
                format: video::VideoPixelFormat::Rgba8,
                data: vec![1, 2, 3, 255],
            },
        )])));
        let fault_state = Arc::new(Mutex::new(None));
        record_post_ready_capture_fault(
            &frames,
            &fault_state,
            77,
            "camera disconnected".to_string(),
        )
        .unwrap();
        assert!(frames.lock().unwrap().get(&77).is_none());
        assert_eq!(
            fault_state.lock().unwrap().as_deref(),
            Some("camera disconnected")
        );

        let mut transport = CaptureTransportState::new(Arc::clone(&frames));
        transport.workers.insert(
            77,
            CaptureRouteWorker::faulted_for_test("camera disconnected"),
        );
        assert_eq!(
            transport.current_route_fault(77).unwrap().as_deref(),
            Some("camera disconnected")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn post_spawn_capture_cleanup_kills_and_reaps_the_child() {
        let command_shell = std::env::var_os("COMSPEC").expect("COMSPEC must be available");
        let mut process = Command::new(command_shell)
            .args(["/d", "/s", "/c", "for /L %i in (1,1,2147483647) do @rem"])
            .spawn()
            .unwrap();
        terminate_process(&mut process);
        assert!(process.try_wait().unwrap().is_some());
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
        assert_eq!(
            (frame.width, frame.height),
            (SCREEN_CAPTURE_WIDTH, SCREEN_CAPTURE_HEIGHT)
        );
        assert_eq!(
            frame.data.len(),
            (SCREEN_CAPTURE_WIDTH * SCREEN_CAPTURE_HEIGHT * 4) as usize
        );
        worker.stop();
    }

    #[test]
    #[ignore = "requires FFmpeg and SYNDOCAL_TEST_CAMERA_ENDPOINT containing a canonical DirectShow profile endpoint"]
    fn camera_worker_captures_a_real_frame_and_restarts_cleanly() {
        let endpoint = std::env::var("SYNDOCAL_TEST_CAMERA_ENDPOINT")
            .expect("set SYNDOCAL_TEST_CAMERA_ENDPOINT to a canonical DirectShow profile endpoint");
        let frames = Arc::new(Mutex::new(HashMap::new()));
        let plan = capture_ffmpeg_plan("camera", &endpoint).unwrap();

        for attempt in 0..2 {
            let layer_id = 80 + attempt;
            let worker =
                CaptureRouteWorker::start(layer_id, "camera", &endpoint, Arc::clone(&frames))
                    .unwrap();
            let frame = frames.lock().unwrap().get(&layer_id).cloned().unwrap();
            assert_eq!(
                (frame.width, frame.height),
                (plan.frame.width, plan.frame.height)
            );
            assert_eq!(
                frame.data.len(),
                crate::capture_catalog::checked_frame_bytes(plan.frame.width, plan.frame.height)
                    .unwrap()
            );
            assert!(
                frame.data.chunks_exact(4).any(|pixel| pixel[3] != 0),
                "physical camera returned a fully transparent frame"
            );
            worker.stop();
        }
    }
}
