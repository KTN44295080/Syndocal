//! Demand-driven Syphon input. The native client never leaves its owner thread.
//! Render-side handoff is bounded and lock-free; retirement retains JoinHandles.

#[cfg(any(test, all(target_os = "macos", feature = "syphon")))]
mod worker {
    use crate::syphon_transport::syphon_diagnostics::InputRecorder;
    use crossbeam_queue::ArrayQueue;
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
            Arc, Mutex, OnceLock,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    const MAX_CLIENTS: usize = 64;
    static LIVE_CLIENTS: AtomicUsize = AtomicUsize::new(0);
    // Only construction and decoder destruction take this cold-path lock.
    // An unfinished owner is never detached, even if native close fails.
    struct RetiringOwner {
        worker: JoinHandle<()>,
        state: Arc<Handoff>,
    }
    static RETIRING: Mutex<Vec<RetiringOwner>> = Mutex::new(Vec::new());

    pub(super) trait Receiver {
        fn receive(&mut self, pixels: &mut Vec<u8>) -> Result<Option<(u32, u32)>, String>;
        fn close(&mut self) -> Result<(), String>;
    }
    #[cfg(all(target_os = "macos", feature = "syphon"))]
    impl Receiver for crate::syphon_native::SyphonReceiver {
        fn receive(&mut self, pixels: &mut Vec<u8>) -> Result<Option<(u32, u32)>, String> {
            self.receive_rgba(pixels)
        }
        fn close(&mut self) -> Result<(), String> {
            self.close()
        }
    }

    type FrameResult = Result<Arc<video::VideoFrame>, String>;
    struct Control {
        endpoint: String,
        enabled: Arc<AtomicBool>,
        clients: Vec<Arc<Handoff>>,
    }
    static CONTROLS: Mutex<Vec<(u64, Control)>> = Mutex::new(Vec::new());

    pub(crate) fn start_route(
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        let error = |message: &str| video::ExternalVideoTransportDriverError {
            message: message.to_owned(),
        };
        if route.backend_id != "syphon"
            || route.direction != video::ExternalVideoTransportDirection::Input
        {
            return Err(error("Syphon input accepts only its input routes"));
        }
        if route.endpoint_name.trim().is_empty() || route.endpoint_name.len() > 4096 {
            return Err(error(
                "Select a discovered Syphon server identity (1..4096 UTF-8 bytes)",
            ));
        }
        let mut controls = CONTROLS.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((_, control)) = controls.iter_mut().find(|(id, _)| *id == route.route_id) {
            if control.enabled.load(Ordering::Acquire) && control.endpoint == route.endpoint_name {
                return Ok(());
            }
            control.enabled.store(false, Ordering::Release);
            if !stop_clients(control) {
                return Err(error(
                    "Previous Syphon clients are still closing; replacement is blocked",
                ));
            }
            control.endpoint = route.endpoint_name.clone();
            control.enabled = Arc::new(AtomicBool::new(true));
            control.clients.clear();
            return Ok(());
        }
        controls.retain(|(_, control)| {
            control.enabled.load(Ordering::Acquire) || !stop_clients(control)
        });
        if controls.len() >= MAX_CLIENTS {
            return Err(error(
                "Syphon input route limit reached; stop an existing source first",
            ));
        }
        controls.push((
            route.route_id,
            Control {
                endpoint: route.endpoint_name.clone(),
                enabled: Arc::new(AtomicBool::new(true)),
                clients: Vec::new(),
            },
        ));
        Ok(())
    }
    fn stop_clients(control: &Control) -> bool {
        let mut finished = true;
        for state in &control.clients {
            state.stop.store(true, Ordering::Release);
            if let Some(thread) = state.thread.get() {
                thread.unpark();
            }
            finished &= state.finished.load(Ordering::Acquire);
        }
        finished
    }
    pub(crate) fn stop_route(
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        let mut controls = CONTROLS.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((_, control)) = controls.iter_mut().find(|(id, _)| *id == route.route_id) {
            control.enabled.store(false, Ordering::Release);
            if !stop_clients(control) {
                return Err(video::ExternalVideoTransportDriverError {
                    message: "Syphon input stop is pending until every native client is released"
                        .to_owned(),
                });
            }
        }
        controls.retain(|(id, _)| *id != route.route_id);
        Ok(())
    }

    // Read only from control/status adapters; the decoder never takes this lock.
    pub(crate) fn current_input_fault(
        layer_id: u64,
        nonblocking: bool,
    ) -> Result<Option<String>, String> {
        let controls = if nonblocking {
            CONTROLS
                .try_lock()
                .map_err(|_| "Syphon input status is busy; retry".to_owned())?
        } else {
            CONTROLS
                .lock()
                .map_err(|_| "Syphon input control state lock was poisoned".to_owned())?
        };
        let Some((_, control)) = controls.iter().find(|(id, _)| *id == layer_id) else {
            return Ok(None);
        };
        for state in &control.clients {
            if state.finished.load(Ordering::Acquire) {
                continue;
            }
            let fault = if nonblocking {
                state
                    .fault
                    .try_lock()
                    .map_err(|_| "Syphon input fault status is busy; retry".to_owned())?
            } else {
                state
                    .fault
                    .lock()
                    .map_err(|_| "Syphon input fault state lock was poisoned".to_owned())?
            };
            if fault.is_some() {
                return Ok(fault.clone());
            }
        }
        Ok(None)
    }
    fn transition_fault(state: &Handoff, previous: &mut Option<String>, error: Option<&str>) {
        if previous.as_deref() == error {
            return;
        }
        let bounded = error.map(|error| error.chars().take(2048).collect::<String>());
        *state
            .fault
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = bounded.clone();
        *previous = bounded;
    }

    struct Handoff {
        demand: AtomicBool,
        position_ms: AtomicU64,
        finished: AtomicBool,
        thread: OnceLock<thread::Thread>,
        stop: AtomicBool,
        fault: Mutex<Option<String>>,
        results: ArrayQueue<FrameResult>,
        buffers: ArrayQueue<Vec<u8>>,
    }
    struct ClientCount {
        state: Option<Arc<Handoff>>,
    }
    impl Drop for ClientCount {
        fn drop(&mut self) {
            if self
                .state
                .as_ref()
                .is_none_or(|state| state.finished.load(Ordering::Acquire))
            {
                LIVE_CLIENTS.fetch_sub(1, Ordering::AcqRel);
            }
        }
    }

    fn recycle(handoff: &Handoff, result: FrameResult) {
        if let Ok(frame) = result {
            if let Ok(frame) = Arc::try_unwrap(frame) {
                let _ = handoff.buffers.push(frame.data);
            }
        }
    }
    fn publish(handoff: &Handoff, result: FrameResult) {
        if let Some(old) = handoff.results.pop() {
            recycle(handoff, old);
        }
        // Single producer; consumer only removes, so this slot cannot be full.
        if let Err(result) = handoff.results.push(result) {
            recycle(handoff, result);
        }
    }

    pub(super) struct Route {
        endpoint: String,
        handoff: Arc<Handoff>,
        worker: Option<JoinHandle<()>>,
        latest: Option<Arc<video::VideoFrame>>,
        error: Option<String>,
    }
    impl Route {
        fn start<R: Receiver + 'static>(
            layer_id: u64,
            endpoint: &str,
            mut create: impl FnMut(&str) -> Result<R, String> + Send + 'static,
        ) -> Result<Self, String> {
            {
                let mut retired = RETIRING.lock().unwrap_or_else(|p| p.into_inner());
                let mut index = 0;
                while index < retired.len() {
                    if retired[index].worker.is_finished()
                        && retired[index].state.finished.load(Ordering::Acquire)
                    {
                        let _ = retired.swap_remove(index).worker.join();
                    } else {
                        index += 1;
                    }
                }
            }
            LIVE_CLIENTS
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                    (count < MAX_CLIENTS).then_some(count + 1)
                })
                .map_err(|_| {
                    "Syphon input limit reached; stop an existing source first".to_owned()
                })?;
            let count = ClientCount { state: None };
            let handoff = Arc::new(Handoff {
                demand: AtomicBool::new(false),
                position_ms: AtomicU64::new(0),
                finished: AtomicBool::new(false),
                thread: OnceLock::new(),
                stop: AtomicBool::new(false),
                fault: Mutex::new(Some(
                    "Waiting for a Syphon frame; start the selected server and publish video"
                        .to_owned(),
                )),
                results: ArrayQueue::new(1),
                buffers: ArrayQueue::new(2),
            });
            let state = Arc::clone(&handoff);
            let selected = endpoint.to_owned();
            let worker = thread::Builder::new()
                .name(format!("syphon-input-{layer_id}"))
                .spawn(move || {
                    let mut count = count;
                    count.state = Some(Arc::clone(&state));
                    let _count = count;
                    let mut receiver: Option<R> = None;
                    let mut retiring = false;
                    let mut close_failure_reported = false;
                    let mut last_fault = Some(
                        "Waiting for a Syphon frame; start the selected server and publish video"
                            .to_owned(),
                    );
                    let mut recorder = InputRecorder::new(layer_id, &selected);
                    recorder.snapshot();
                    loop {
                        if state.stop.load(Ordering::Acquire) || retiring {
                            if let Some(client) = receiver.as_mut() {
                                if let Err(error) =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        client.close()
                                    }))
                                    .unwrap_or_else(|_| {
                                        Err("Syphon input close panicked; native owner retained"
                                            .to_owned())
                                    })
                                {
                                    recorder.error(&error);
                                    transition_fault(&state, &mut last_fault, Some(&error));
                                    publish(
                                        &state,
                                        Err(format!("Syphon input close pending: {error}")),
                                    );
                                    if state.stop.load(Ordering::Acquire) {
                                        // Retry only at the ownership boundary, never on the render tick.
                                        if !close_failure_reported {
                                            recorder.snapshot();
                                            close_failure_reported = true;
                                        }
                                        thread::sleep(Duration::from_millis(50));
                                        continue;
                                    }
                                    retiring = true;
                                } else {
                                    receiver = None;
                                    retiring = false;
                                    close_failure_reported = false;
                                    recorder.native_closed();
                                    recorder.snapshot();
                                }
                            }
                            if state.stop.load(Ordering::Acquire) && receiver.is_none() {
                                recorder.native_closed();
                                recorder.snapshot();
                                break;
                            }
                        }
                        if !state.demand.swap(false, Ordering::AcqRel) {
                            thread::park();
                            continue;
                        }
                        if retiring {
                            continue;
                        }
                        if receiver.is_none() {
                            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                create(&selected)
                            }))
                            .unwrap_or_else(|_| {
                                Err("Syphon input construction panicked".to_owned())
                            }) {
                                Ok(client) => {
                                    receiver = Some(client);
                                    // Retire even a failed/no-client construction record before replacing it.
                                    recorder.native_closed();
                                    recorder.snapshot();
                                    recorder = InputRecorder::new(layer_id, &selected);
                                    recorder.snapshot();
                                }
                                Err(error) => {
                                    recorder.error(&error);
                                    transition_fault(&state, &mut last_fault, Some(&error));
                                    publish(&state, Err(error));
                                    // No profile write on per-frame missing-server retries.
                                    continue;
                                }
                            }
                        }
                        let mut pixels = state.buffers.pop().unwrap_or_default();
                        let started = Instant::now();
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            receiver.as_mut().unwrap().receive(&mut pixels)
                        }))
                        .unwrap_or_else(|_| {
                            Err("Syphon input receive panicked; retiring native owner".to_owned())
                        }) {
                            Ok(Some((width, height))) => {
                                transition_fault(&state, &mut last_fault, None);
                                recorder.received(width, height, started.elapsed());
                                publish(
                                    &state,
                                    Ok(Arc::new(video::VideoFrame {
                                        layer_id,
                                        width,
                                        height,
                                        pts_ms: state.position_ms.load(Ordering::Acquire),
                                        duration_ms: 0,
                                        format: video::VideoPixelFormat::Rgba8,
                                        data: pixels,
                                    })),
                                );
                            }
                            Ok(None) => {
                                let _ = state.buffers.push(pixels);
                            }
                            Err(error) => {
                                let _ = state.buffers.push(pixels);
                                recorder.error(&error);
                                transition_fault(&state, &mut last_fault, Some(&error));
                                publish(&state, Err(error));
                                retiring = true;
                            }
                        }
                    }
                    // A finished OS thread alone is never native-release evidence.
                    state.finished.store(true, Ordering::Release);
                })
                .map_err(|error| format!("Could not start Syphon input owner: {error}"))?;
            let _ = handoff.thread.set(worker.thread().clone());
            Ok(Self {
                endpoint: endpoint.to_owned(),
                handoff,
                worker: Some(worker),
                latest: None,
                error: None,
            })
        }
        fn stop(&mut self) {
            self.latest = None;
            self.handoff.stop.store(true, Ordering::Release);
            if let Some(worker) = &self.worker {
                worker.thread().unpark();
            }
        }
        fn reap(&mut self) -> bool {
            if self.handoff.finished.load(Ordering::Acquire)
                && self.worker.as_ref().is_some_and(JoinHandle::is_finished)
            {
                let _ = self.worker.take().unwrap().join();
            }
            self.worker.is_none()
        }
        fn frame(&mut self, position_ms: u64) -> Result<Arc<video::VideoFrame>, String> {
            if self.handoff.stop.load(Ordering::Acquire) {
                self.latest = None;
                return Err(
                    "Syphon input is stopped; enable the source route to receive frames".to_owned(),
                );
            }
            if let Some(result) = self.handoff.results.pop() {
                if let Some(old) = self.latest.take() {
                    recycle(&self.handoff, Ok(old));
                }
                match result {
                    Ok(frame) => {
                        self.latest = Some(frame);
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            if self.worker.as_ref().is_none_or(JoinHandle::is_finished) {
                return Err(
                    "Syphon input owner stopped; refresh and select the server again".to_owned(),
                );
            }
            self.handoff
                .position_ms
                .store(position_ms, Ordering::Release);
            self.handoff.demand.store(true, Ordering::Release);
            self.worker.as_ref().unwrap().thread().unpark();
            if let Some(error) = &self.error {
                return Err(error.clone());
            }
            self.latest.as_ref().map(Arc::clone).ok_or_else(|| {
                "Waiting for a Syphon frame; start the selected server and publish video".to_owned()
            })
        }
    }
    impl Drop for Route {
        fn drop(&mut self) {
            self.stop();
            if !self.reap() {
                // Keep the owner handle until the client has actually stopped.
                RETIRING
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push(RetiringOwner {
                        worker: self.worker.take().unwrap(),
                        state: Arc::clone(&self.handoff),
                    });
            }
        }
    }

    #[derive(Default)]
    pub(crate) struct Inputs {
        routes: HashMap<u64, Route>,
    }
    impl Inputs {
        pub(crate) fn retain_layers(&mut self, layer_ids: &[u64]) {
            self.routes.retain(|id, route| {
                if !layer_ids.contains(id) {
                    route.stop();
                    return !route.reap();
                }
                true
            });
        }
        pub(crate) fn release_layer(&mut self, layer_id: u64) {
            if let Some(route) = self.routes.get_mut(&layer_id) {
                route.stop();
            }
            self.routes
                .retain(|id, route| *id != layer_id || !route.reap());
        }
        #[cfg(all(target_os = "macos", feature = "syphon"))]
        pub(crate) fn decode(
            &mut self,
            request: &video::VideoFrameRequest,
        ) -> Result<Arc<video::VideoFrame>, String> {
            self.decode_registered_with(request, crate::syphon_native::SyphonReceiver::new)
        }
        pub(super) fn decode_registered_with<R: Receiver + 'static>(
            &mut self,
            request: &video::VideoFrameRequest,
            create: impl FnMut(&str) -> Result<R, String> + Send + 'static,
        ) -> Result<Arc<video::VideoFrame>, String> {
            let endpoint = request
                .source
                .name
                .as_deref()
                .ok_or_else(|| "Select a discovered Syphon server".to_owned())?;
            if !self.routes.contains_key(&request.layer_id) {
                let mut controls = CONTROLS.lock().unwrap_or_else(|p| p.into_inner());
                let control = controls
                    .iter_mut()
                    .find(|(id, control)| {
                        *id == request.layer_id
                            && control.endpoint == endpoint
                            && control.enabled.load(Ordering::Acquire)
                    })
                    .map(|(_, control)| control)
                    .ok_or_else(|| {
                        "Syphon input route is not active; enable the selected source".to_owned()
                    })?;
                let route = Route::start(request.layer_id, endpoint, create)?;
                control
                    .clients
                    .retain(|client| !client.finished.load(Ordering::Acquire));
                control.clients.push(Arc::clone(&route.handoff));
                self.routes.insert(request.layer_id, route);
            }
            let route = self.routes.get_mut(&request.layer_id).unwrap();
            if route.endpoint != endpoint || route.handoff.stop.load(Ordering::Acquire) {
                route.stop();
                if route.reap() {
                    self.routes.remove(&request.layer_id);
                }
                return Err(
                    "Previous Syphon client is closing; source replacement is pending".to_owned(),
                );
            }
            route.frame(request.position_ms)
        }
        #[cfg(test)]
        pub(super) fn decode_with<R: Receiver + 'static>(
            &mut self,
            request: &video::VideoFrameRequest,
            create: impl FnMut(&str) -> Result<R, String> + Send + 'static,
        ) -> Result<Arc<video::VideoFrame>, String> {
            let endpoint = request
                .source
                .name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| {
                    "Select a discovered Syphon server and refresh the list if it is missing"
                        .to_owned()
                })?;
            if endpoint.len() > 4096 {
                return Err("Syphon server identity exceeds 4096 UTF-8 bytes".to_owned());
            }
            if let Some(route) = self.routes.get_mut(&request.layer_id) {
                if route.endpoint != endpoint || route.handoff.stop.load(Ordering::Acquire) {
                    route.stop();
                    if !route.reap() {
                        return Err("Previous Syphon client is still closing; source replacement is blocked".to_owned());
                    }
                    self.routes.remove(&request.layer_id);
                }
            }
            if !self.routes.contains_key(&request.layer_id) {
                self.routes.insert(
                    request.layer_id,
                    Route::start(request.layer_id, endpoint, create)?,
                );
            }
            self.routes
                .get_mut(&request.layer_id)
                .unwrap()
                .frame(request.position_ms)
        }
    }
}

#[cfg(all(target_os = "macos", feature = "syphon"))]
pub(crate) use worker::{current_input_fault, start_route, stop_route, Inputs};

#[cfg(test)]
#[path = "syphon_input_tests.rs"]
mod tests;
