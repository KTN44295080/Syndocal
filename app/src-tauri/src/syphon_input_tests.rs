use super::worker::{Inputs, Receiver};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    missing: AtomicBool,
    panic_receive: AtomicBool,
    fail_close: AtomicBool,
    closed: AtomicUsize,
    created: AtomicUsize,
    receives: AtomicUsize,
}
struct FakeReceiver {
    state: Arc<State>,
}
impl Receiver for FakeReceiver {
    fn receive(&mut self, pixels: &mut Vec<u8>) -> Result<Option<(u32, u32)>, String> {
        self.state.receives.fetch_add(1, Ordering::AcqRel);
        if self.state.panic_receive.swap(false, Ordering::AcqRel) {
            panic!("injected receive panic");
        }
        if self.state.missing.load(Ordering::Acquire) {
            return Err(
                "Selected Syphon server vanished; start it again or refresh the picker".to_owned(),
            );
        }
        pixels.clear();
        pixels.extend_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255]);
        Ok(Some((2, 1)))
    }
    fn close(&mut self) -> Result<(), String> {
        if self.state.fail_close.load(Ordering::Acquire) {
            return Err("injected close failure".to_owned());
        }
        self.state.closed.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}
fn request(endpoint: &str) -> video::VideoFrameRequest {
    video::VideoFrameRequest {
        layer_id: 100,
        label: "Syphon input".to_owned(),
        source: protocol::VideoSourceSummary {
            kind: protocol::VideoSourceKind::Syphon,
            name: Some(endpoint.to_owned()),
            path: None,
            codec: None,
            metadata: None,
        },
        position_ms: 500,
        width: 2,
        height: 1,
    }
}
fn decode(
    inputs: &mut Inputs,
    request: &video::VideoFrameRequest,
    state: &Arc<State>,
) -> Result<Arc<video::VideoFrame>, String> {
    let state = Arc::clone(state);
    inputs.decode_with(request, move |_| {
        if state.missing.load(Ordering::Acquire) {
            return Err(
                "Selected Syphon server is missing; start it again or refresh the picker"
                    .to_owned(),
            );
        }
        state.created.fetch_add(1, Ordering::AcqRel);
        Ok(FakeReceiver {
            state: Arc::clone(&state),
        })
    })
}
fn eventually(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "Syphon fake worker did not reach the expected state"
        );
        std::thread::yield_now();
    }
}

#[test]
fn syphon_input_missing_server_reports_error_and_recovers_without_stale_pixels() {
    let state = Arc::new(State::default());
    state.missing.store(true, Ordering::Release);
    let mut inputs = Inputs::default();
    let request = request("{\"uuid\":\"test\",\"name\":\"Program\",\"app_name\":\"Server\"}");
    eventually(|| {
        decode(&mut inputs, &request, &state).is_err_and(|error| error.contains("missing"))
    });
    state.missing.store(false, Ordering::Release);
    eventually(|| decode(&mut inputs, &request, &state).is_ok());
    let frame = decode(&mut inputs, &request, &state).unwrap();
    assert_eq!((frame.width, frame.height), (2, 1));
    assert_eq!(frame.format, video::VideoPixelFormat::Rgba8);
    assert_eq!(frame.data, [255, 0, 0, 255, 0, 255, 0, 255]);
    drop(frame);
    state.missing.store(true, Ordering::Release);
    eventually(|| {
        decode(&mut inputs, &request, &state)
            .is_err_and(|error| error.contains("vanished") || error.contains("missing"))
    });
    state.missing.store(false, Ordering::Release);
    eventually(|| decode(&mut inputs, &request, &state).is_ok());
    assert!(state.created.load(Ordering::Acquire) >= 2);
    inputs.retain_layers(&[]);
    eventually(|| {
        inputs.retain_layers(&[]);
        state.closed.load(Ordering::Acquire) >= 2
    });
}

#[test]
fn syphon_input_failed_close_keeps_owner_and_blocks_replacement() {
    let state = Arc::new(State::default());
    let mut inputs = Inputs::default();
    eventually(|| decode(&mut inputs, &request("first"), &state).is_ok());
    state.fail_close.store(true, Ordering::Release);
    let error = decode(&mut inputs, &request("second"), &state).unwrap_err();
    assert!(error.contains("still closing"));
    assert_eq!(state.created.load(Ordering::Acquire), 1);
    assert_eq!(state.closed.load(Ordering::Acquire), 0);
    state.fail_close.store(false, Ordering::Release);
    eventually(|| decode(&mut inputs, &request("second"), &state).is_ok());
    assert_eq!(state.created.load(Ordering::Acquire), 2);
    inputs.release_layer(100);
    eventually(|| {
        inputs.release_layer(100);
        state.closed.load(Ordering::Acquire) >= 2
    });
}

#[test]
fn syphon_input_worker_sleeps_without_decode_demand_and_drop_retires_client() {
    let state = Arc::new(State::default());
    let mut inputs = Inputs::default();
    eventually(|| decode(&mut inputs, &request("source"), &state).is_ok());
    // Let the one coalesced request issued by the last decode finish.
    let before = state.receives.load(Ordering::Acquire);
    std::thread::sleep(Duration::from_millis(20));
    let settled = state.receives.load(Ordering::Acquire);
    assert!(settled <= before + 1);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(settled, state.receives.load(Ordering::Acquire));
    drop(inputs);
    eventually(|| state.closed.load(Ordering::Acquire) == 1);
}

#[cfg(not(all(target_os = "macos", feature = "syphon")))]
#[test]
fn syphon_decoder_without_native_backend_returns_actionable_error() {
    use video::VideoFrameDecoder;
    let mut decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::from_env();
    let error = decoder.decode_frame(&request("source")).unwrap_err();
    match error {
        video::VideoDecodeError::Decode {
            message, layer_id, ..
        } => {
            assert_eq!(layer_id, 100);
            assert!(message.contains("macOS build with the syphon feature"));
        }
        other => panic!("Expected actionable native-backend decode error, got {other:?}"),
    }
}

#[test]
fn syphon_input_control_routes_are_bounded_and_direction_scoped() {
    let route = video::ExternalVideoTransportRoute {
        direction: video::ExternalVideoTransportDirection::Input,
        route_id: 10099,
        label: "test".to_owned(),
        backend_id: "syphon".to_owned(),
        endpoint_name: "test identity".to_owned(),
    };
    super::worker::start_route(&route).unwrap();
    super::worker::start_route(&route).unwrap();
    super::worker::stop_route(&route).unwrap();
    let mut invalid = route;
    invalid.direction = video::ExternalVideoTransportDirection::Output;
    assert!(super::worker::start_route(&invalid).is_err());
}

#[test]
fn syphon_input_registered_stop_retains_failed_close_and_receive_panic_recovers() {
    let state = Arc::new(State::default());
    let mut inputs = Inputs::default();
    let mut request = request("registered source");
    request.layer_id = 10098;
    let route = video::ExternalVideoTransportRoute {
        direction: video::ExternalVideoTransportDirection::Input,
        route_id: request.layer_id,
        label: "test".to_owned(),
        backend_id: "syphon".to_owned(),
        endpoint_name: request.source.name.clone().unwrap(),
    };
    super::worker::start_route(&route).unwrap();
    let read = |inputs: &mut Inputs| {
        let state = Arc::clone(&state);
        inputs.decode_registered_with(&request, move |_| {
            state.created.fetch_add(1, Ordering::AcqRel);
            Ok(FakeReceiver {
                state: Arc::clone(&state),
            })
        })
    };
    eventually(|| read(&mut inputs).is_ok());
    state.panic_receive.store(true, Ordering::Release);
    eventually(|| {
        let _ = read(&mut inputs);
        state.created.load(Ordering::Acquire) >= 2
    });
    eventually(|| read(&mut inputs).is_ok());
    state.fail_close.store(true, Ordering::Release);
    assert!(super::worker::stop_route(&route).is_err());
    assert!(read(&mut inputs).is_err());
    assert!(super::worker::start_route(&route).is_err());
    state.fail_close.store(false, Ordering::Release);
    eventually(|| super::worker::stop_route(&route).is_ok());
    assert!(read(&mut inputs).is_err());
    assert_eq!(
        state.created.load(Ordering::Acquire),
        state.closed.load(Ordering::Acquire)
    );
}

#[test]
fn syphon_input_status_reports_missing_and_vanished_server_and_clears_on_frame() {
    let state = Arc::new(State::default());
    state.missing.store(true, Ordering::Release);
    let mut inputs = Inputs::default();
    let mut request = request("status source");
    request.layer_id = 10097;
    let route = video::ExternalVideoTransportRoute {
        direction: video::ExternalVideoTransportDirection::Input,
        route_id: request.layer_id,
        label: "test".to_owned(),
        backend_id: "syphon".to_owned(),
        endpoint_name: request.source.name.clone().unwrap(),
    };
    super::worker::start_route(&route).unwrap();
    let read = |inputs: &mut Inputs| {
        let state = Arc::clone(&state);
        inputs.decode_registered_with(&request, move |_| {
            if state.missing.load(Ordering::Acquire) {
                return Err(
                    "Selected Syphon server is missing; start it again or refresh the picker"
                        .to_owned(),
                );
            }
            state.created.fetch_add(1, Ordering::AcqRel);
            Ok(FakeReceiver {
                state: Arc::clone(&state),
            })
        })
    };
    eventually(|| {
        let _ = read(&mut inputs);
        super::worker::current_input_fault(request.layer_id, true)
            .is_ok_and(|fault| fault.is_some_and(|error| error.contains("missing")))
    });
    state.missing.store(false, Ordering::Release);
    eventually(|| {
        read(&mut inputs).is_ok()
            && super::worker::current_input_fault(request.layer_id, false)
                .unwrap()
                .is_none()
    });
    state.missing.store(true, Ordering::Release);
    eventually(|| {
        let _ = read(&mut inputs);
        super::worker::current_input_fault(request.layer_id, true).is_ok_and(|fault| {
            fault.is_some_and(|error| error.contains("vanished") || error.contains("missing"))
        })
    });
    state.missing.store(false, Ordering::Release);
    eventually(|| {
        read(&mut inputs).is_ok()
            && super::worker::current_input_fault(request.layer_id, false)
                .unwrap()
                .is_none()
    });
    eventually(|| super::worker::stop_route(&route).is_ok());
    assert!(super::worker::current_input_fault(request.layer_id, false)
        .unwrap()
        .is_none());
}
