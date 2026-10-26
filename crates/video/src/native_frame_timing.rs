//! Opt-in, bounded host-wall attribution. This module owns no product resource.
//!
//! `prepare_thread` and `retire_thread` are cold ownership boundaries. The render
//! loop only touches its TLS primitive buffer. Export requires capture stop,
//! completion of every parent, retirement, and the existing Stop acknowledgement.
use serde::Serialize;
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const MAX_PARENTS: usize = 3072;
pub const MAX_CHILDREN: usize = 64;
const MAX_ATTEMPTS: usize = 8;
pub const MAX_DECODER_REQUESTS: usize = 8;
const ROOT: u8 = u8::MAX;
const PARTIAL: u32 = 1;
const CHILD_OVERFLOW: u32 = 2;
const INVALID_TREE: u32 = 4;
const TIME_OVERFLOW: u32 = 8;
const METADATA_OVERFLOW: u32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Prepare,
    Retention,
    RetentionJoin,
    RetentionRecv,
    Demand,
    ChainPreparation,
    Queue,
    Isf,
    IsfUpload,
    IsfEncode,
    IsfSubmit,
    IsfMapWait,
    IsfMappedCopy,
    IsfUnmap,
    IsfCleanup,
    CpuComposition,
    LastValidLookup,
    LastValidClone,
    LastValidReplace,
    PresenterAcquire,
    PresenterReconfigure,
    PresenterUpload,
    PresenterEncode,
    PresenterSubmit,
    PresenterValidation,
    PresenterPresent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum IsfPath {
    #[default]
    Ordinary,
    Batch2,
    Batch3,
    Recovery,
    Decline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Outcome {
    Presented,
    Cancelled,
    Revoked,
    ZeroPresent,
    #[default]
    Failed,
    Rejected,
}

#[derive(Clone, Copy, Debug, Default)]
struct Child {
    start_us: u32,
    end_us: u32,
    parent: u8,
    phase: Phase,
    complete: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct IsfAttempt {
    pub layer_id: u64,
    pub frame_index: u64,
    pub stage_count: u32,
    pub path: IsfPath,
    pub span_id: u8,
}

pub const DECODER_CACHED: u8 = 1;
pub const DECODER_PREFETCH: u8 = 2;
pub const DECODER_SYNC: u8 = 4;
pub const DECODER_RESET: u8 = 8;
pub const DECODER_REOPEN: u8 = 16;
pub const DECODER_PROVIDER: u8 = 32;

/// Worker host-wall times are metadata, never children of the caller's tree.
/// Their sum across parallel requests must not be used as parent elapsed time.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct DecoderWorkSample {
    pub worker_id: u64,
    pub last_worker_id: u64,
    pub queued_us: u64,
    pub start_us: u64,
    pub end_us: u64,
    pub decode_io_us: u32,
    pub hardware_download_us: u32,
    pub swscale_us: u32,
    pub alloc_copy_us: u32,
    pub invalid: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct DecoderRequestSnapshot {
    pub layer_id: u64,
    pub project_render_epoch: u64,
    pub input_id: u64,
    pub position_ms: u64,
    pub previous_request_ms: Option<u64>,
    pub cached_pts_ms: Option<u64>,
    pub next_pts_ms: Option<u64>,
    pub selected_pts_ms: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub start_us: u32,
    pub end_us: u32,
    pub demand_span_id: u8,
    pub source_request_id: Option<u8>,
    /// Bitset: cached=1, prefetch=2, synchronous=4, reset=8, reopen=16, provider=32.
    pub paths: u8,
    pub complete: bool,
    pub cancelled: bool,
    pub work_count: u8,
    pub work: DecoderWorkSample,
}

#[derive(Clone, Copy)]
struct Parent {
    id: u64,
    output: u64,
    epoch: u64,
    start_us: u64,
    end_us: u64,
    gap_before_us: u64,
    width: u32,
    height: u32,
    flags: u32,
    outcome: Outcome,
    complete: bool,
    child_count: u8,
    stack: u8,
    attempt_count: u8,
    requested_count: u8,
    failed_count: u8,
    decoder_request_count: u8,
    requested_layers: [u64; 8],
    failed: [(u64, u64); 3],
    retired_requested: u32,
    retired_other: u32,
    demand_before: [u64; 4],
    demand_after: [u64; 4],
    attempts: [IsfAttempt; MAX_ATTEMPTS],
    decoder_requests: [DecoderRequestSnapshot; MAX_DECODER_REQUESTS],
    children: [Child; MAX_CHILDREN],
}

impl Default for Parent {
    fn default() -> Self {
        Self {
            id: 0,
            output: 0,
            epoch: 0,
            start_us: 0,
            end_us: 0,
            gap_before_us: 0,
            width: 0,
            height: 0,
            flags: 0,
            outcome: Outcome::Failed,
            complete: false,
            child_count: 0,
            stack: ROOT,
            attempt_count: 0,
            requested_count: 0,
            failed_count: 0,
            decoder_request_count: 0,
            requested_layers: [0; 8],
            failed: [(0, 0); 3],
            retired_requested: 0,
            retired_other: 0,
            demand_before: [0; 4],
            demand_after: [0; 4],
            attempts: [IsfAttempt::default(); MAX_ATTEMPTS],
            decoder_requests: [DecoderRequestSnapshot::default(); MAX_DECODER_REQUESTS],
            children: [Child::default(); MAX_CHILDREN],
        }
    }
}

struct Buffer {
    generation: u64,
    parents: Box<[Parent]>,
    len: usize,
    current: Option<usize>,
    attempted: u64,
    overflow: bool,
    previous_end: u64,
}

pub const RETAINED_BYTES: usize =
    MAX_PARENTS * std::mem::size_of::<Parent>() + std::mem::size_of::<Buffer>();
// Eight compact request records per parent retain explicit worker attribution
// without expanding the 64-child tree. Allocation remains cold and bounded.
pub const MAX_RETAINED_BYTES: usize = 8 * 1024 * 1024;
const _: () = assert!(RETAINED_BYTES <= MAX_RETAINED_BYTES);

#[derive(Default)]
struct Control {
    generation: u64,
    start_us: u64,
    start_unix_us: u64,
    clock_calibration_uncertainty_us: u64,
    stop_us: u64,
    buffer: Option<Buffer>,
}

static CONTROL: Mutex<Control> = Mutex::new(Control {
    generation: 0,
    start_us: 0,
    start_unix_us: 0,
    clock_calibration_uncertainty_us: 0,
    stop_us: 0,
    buffer: None,
});
static CLOCK: OnceLock<Instant> = OnceLock::new();
static ACTIVE: AtomicU64 = AtomicU64::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static OWNER: AtomicBool = AtomicBool::new(false);
static ACK: AtomicBool = AtomicBool::new(true);
static ORIGIN_US: AtomicU64 = AtomicU64::new(0);
static UNPREPARED: AtomicU64 = AtomicU64::new(0);
// Written once at startup, before any render/decode thread is started. Stop
// changes ACTIVE only: an admitted parent's spans must still finish afterwards.
static PROCESS_ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static BUFFER_INITIALIZATIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(not(test))]
thread_local! { static BUFFER: RefCell<Option<Buffer>> = const { RefCell::new(None) }; }
#[cfg(test)]
thread_local! {
    static BUFFER: RefCell<Option<Buffer>> = {
        BUFFER_INITIALIZATIONS.fetch_add(1, Ordering::Relaxed);
        RefCell::new(None)
    };
}

#[inline]
fn process_enabled() -> bool {
    PROCESS_ENABLED.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn enable_for_tests() {
    // Capture unit tests bypass the application startup boundary. The disabled
    // startup regression runs in a separate process and never calls this helper.
    static ENABLE: std::sync::Once = std::sync::Once::new();
    ENABLE.call_once(|| PROCESS_ENABLED.store(true, Ordering::Relaxed));
}

// Startup-only configuration: no environment lookup or file access in the loop.
const OUTPUT_ENV: &str = "SYNDOCAL_QA_FRAME_TIMING_OUT";
static AUTOMATIC: OnceLock<Option<AutomaticCapture>> = OnceLock::new();

struct AutomaticCapture {
    output: PathBuf,
    start_claimed: AtomicBool,
    started: AtomicBool,
    export_attempted: AtomicBool,
}

impl AutomaticCapture {
    fn new(output: PathBuf) -> Result<Self, String> {
        if !output.is_absolute() || output.file_name().is_none() {
            return Err("Native frame timing output must be an absolute file path".into());
        }
        if output.exists() || !output.parent().is_some_and(|parent| parent.is_dir()) {
            return Err(
                "Native frame timing output must be new and its directory must exist".into(),
            );
        }
        Ok(Self {
            output,
            start_claimed: AtomicBool::new(false),
            started: AtomicBool::new(false),
            export_attempted: AtomicBool::new(false),
        })
    }

    fn start_once(&self) -> Result<(), &'static str> {
        if self.start_claimed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        start()?;
        self.started.store(true, Ordering::Release);
        Ok(())
    }

    fn pending(&self) -> bool {
        self.started.load(Ordering::Acquire) && !self.export_attempted.load(Ordering::Acquire)
    }

    fn export_once(&self) -> Result<bool, String> {
        if !self.pending() {
            return Ok(false);
        }
        // Check completion/ACK before consuming the one permitted attempt.
        let snapshot = snapshot().map_err(str::to_string)?;
        if self.export_attempted.swap(true, Ordering::AcqRel) {
            return Ok(false);
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.output)
            .map_err(|error| format!("Unable to create native frame timing output: {error}"))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &snapshot)
            .map_err(|error| format!("Unable to serialize native frame timing output: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("Unable to finish native frame timing output: {error}"))?;
        Ok(true)
    }
}

fn automatic_from_value(value: Option<OsString>) -> Result<Option<AutomaticCapture>, String> {
    value
        .map(|path| AutomaticCapture::new(PathBuf::from(path)))
        .transpose()
}

pub fn configure_from_environment() -> Result<(), String> {
    let configured = automatic_from_value(std::env::var_os(OUTPUT_ENV))?;
    let enabled = configured.is_some();
    AUTOMATIC
        .set(configured)
        .map_err(|_| "Native frame timing startup configuration was already read".to_string())?;
    PROCESS_ENABLED.store(enabled, Ordering::Relaxed);
    Ok(())
}

/// Cold first-iteration boundary; the initial frame hands the same primitive
/// buffer to the established render worker through retire_thread/prepare_thread.
pub fn prepare_automatic_thread() -> Result<(), String> {
    let Some(automatic) = AUTOMATIC.get().and_then(Option::as_ref) else {
        return Ok(());
    };
    automatic.start_once().map_err(str::to_string)?;
    prepare_thread().map_err(str::to_string)
}

/// Existing native Stop request: primitive admission/timestamp only; no dump.
pub fn request_automatic_stop() {
    if AUTOMATIC
        .get()
        .and_then(Option::as_ref)
        .is_some_and(|automatic| automatic.started.load(Ordering::Acquire))
    {
        let _ = stop_capture();
    }
}

pub fn automatic_export_pending() -> bool {
    AUTOMATIC
        .get()
        .and_then(Option::as_ref)
        .is_some_and(AutomaticCapture::pending)
}

/// Normal close only, after Stop reply, successful product exit boundary and
/// fresh all-deny/retirement checks. No new thread or IPC participates.
pub fn export_automatic_once() -> Result<bool, String> {
    match AUTOMATIC.get().and_then(Option::as_ref) {
        Some(automatic) => automatic.export_once(),
        None => Ok(false),
    }
}

fn absolute_us() -> u64 {
    CLOCK
        .get_or_init(Instant::now)
        .elapsed()
        .as_micros()
        .min(u128::from(u64::MAX)) as u64
}
fn now_us() -> u64 {
    absolute_us().saturating_sub(ORIGIN_US.load(Ordering::Relaxed))
}

pub fn recording_active() -> bool {
    if !process_enabled() {
        return false;
    }
    ACTIVE.load(Ordering::Acquire) != 0
        || BUFFER.with(|cell| cell.borrow().as_ref().is_some_and(|b| b.current.is_some()))
}

/// Start at the first native iteration. Capture one monotonic/UTC calibration;
/// warmup and measurement membership are assigned later from external polls.
pub fn start() -> Result<u64, &'static str> {
    let mut control = CONTROL.lock().map_err(|_| "trace control poisoned")?;
    if ACTIVE.load(Ordering::Acquire) != 0
        || OWNER.load(Ordering::Acquire)
        || !ACK.load(Ordering::Acquire)
    {
        return Err("previous capture has not completed Stop acknowledgement");
    }
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let before = absolute_us();
    let unix_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock predates epoch")?
        .as_micros()
        .min(u128::from(u64::MAX)) as u64;
    let after = absolute_us();
    let origin = before + (after - before) / 2;
    *control = Control {
        generation,
        start_us: origin,
        start_unix_us: unix_us,
        clock_calibration_uncertainty_us: after - before,
        stop_us: 0,
        buffer: None,
    };
    ORIGIN_US.store(origin, Ordering::Relaxed);
    UNPREPARED.store(0, Ordering::Relaxed);
    ACK.store(false, Ordering::Release);
    ACTIVE.store(generation, Ordering::Release);
    Ok(generation)
}

/// Cold allocation/ownership handoff; call before the render loop, never per frame.
pub fn prepare_thread() -> Result<(), &'static str> {
    if !process_enabled() {
        return Ok(());
    }
    let generation = ACTIVE.load(Ordering::Acquire);
    if generation == 0 {
        return Ok(());
    }
    if BUFFER.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|b| b.generation == generation)
    }) {
        return Ok(());
    }
    if OWNER
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("another trace thread still owns the finite buffer");
    }
    let result = (|| {
        let mut control = CONTROL.lock().map_err(|_| "trace control poisoned")?;
        if control.generation != generation || ACTIVE.load(Ordering::Acquire) != generation {
            return Err("capture stopped during thread preparation");
        }
        let buffer = control.buffer.take().unwrap_or_else(|| Buffer {
            generation,
            parents: vec![Parent::default(); MAX_PARENTS].into_boxed_slice(),
            len: 0,
            current: None,
            attempted: 0,
            overflow: false,
            previous_end: 0,
        });
        BUFFER.with(|cell| *cell.borrow_mut() = Some(buffer));
        Ok(())
    })();
    if result.is_err() {
        OWNER.store(false, Ordering::Release);
    }
    result
}

pub struct ThreadRetirementGuard;
impl ThreadRetirementGuard {
    pub fn new() -> Self {
        Self
    }
}
impl Default for ThreadRetirementGuard {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for ThreadRetirementGuard {
    fn drop(&mut self) {
        retire_thread();
    }
}

/// Move only primitive storage after the existing renderer/presenter owners drop.
pub fn retire_thread() {
    if !process_enabled() {
        return;
    }
    let buffer = BUFFER.with(|cell| cell.borrow_mut().take());
    if let Some(mut buffer) = buffer {
        if let Some(index) = buffer.current.take() {
            let parent = &mut buffer.parents[index];
            parent.flags |= PARTIAL;
            parent.end_us = now_us();
        }
        if let Ok(mut control) = CONTROL.lock() {
            if control.generation == buffer.generation {
                control.buffer = Some(buffer);
            }
        }
        OWNER.store(false, Ordering::Release);
    }
}

pub fn stop_capture() -> Result<(), &'static str> {
    let mut control = CONTROL.lock().map_err(|_| "trace control poisoned")?;
    if control.generation == 0 {
        return Err("capture is inactive");
    }
    if ACTIVE.swap(0, Ordering::AcqRel) != 0 {
        control.stop_us = now_us();
    }
    Ok(())
}

/// Invoke only after the product's existing Stop/retirement ACK has completed.
pub fn acknowledge_stop() -> Result<(), &'static str> {
    // Serialize the guard and ACK with start/stop; a new generation must not
    // slip between the inactive check and publishing its acknowledgement.
    let _control = CONTROL.lock().map_err(|_| "trace control poisoned")?;
    if ACTIVE.load(Ordering::Acquire) != 0 {
        return Err("capture is still running");
    }
    if OWNER.load(Ordering::Acquire) {
        return Err("trace owner has not retired");
    }
    // The OFF control run has no generation or buffer. Its post-product-Stop
    // export is valid and explicitly differs from an active capture with zero records.
    ACK.store(true, Ordering::Release);
    Ok(())
}

pub struct ParentGuard {
    generation: u64,
    index: usize,
}
impl ParentGuard {
    pub fn set_dimensions(&self, width: u32, height: u32) {
        update_parent(self.generation, self.index, |p| {
            p.width = width;
            p.height = height;
        });
    }
}
impl Drop for ParentGuard {
    fn drop(&mut self) {
        if !process_enabled() {
            return;
        }
        if self.generation == 0 {
            return;
        }
        BUFFER.with(|cell| {
            let mut slot = cell.borrow_mut();
            let Some(buffer) = slot.as_mut().filter(|b| b.generation == self.generation) else {
                return;
            };
            if buffer.current != Some(self.index) {
                return;
            }
            let parent = &mut buffer.parents[self.index];
            if parent.stack != ROOT {
                parent.flags |= PARTIAL | INVALID_TREE;
            }
            parent.end_us = now_us();
            parent.complete = true;
            buffer.previous_end = parent.end_us;
            buffer.current = None;
        });
    }
}

pub fn begin_parent(output: u64, epoch: u64, width: u32, height: u32) -> ParentGuard {
    let inactive = || ParentGuard {
        generation: 0,
        index: 0,
    };
    if !process_enabled() {
        return inactive();
    }
    let generation = ACTIVE.load(Ordering::Acquire);
    if generation == 0 {
        return inactive();
    }
    BUFFER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(buffer) = slot.as_mut().filter(|b| b.generation == generation) else {
            UNPREPARED.fetch_add(1, Ordering::Relaxed);
            return inactive();
        };
        buffer.attempted += 1;
        if let Some(index) = buffer.current {
            buffer.parents[index].flags |= PARTIAL | INVALID_TREE;
            return inactive();
        }
        if buffer.len == MAX_PARENTS {
            buffer.overflow = true;
            return inactive();
        }
        let index = buffer.len;
        buffer.len += 1;
        let start_us = now_us();
        buffer.parents[index] = Parent {
            id: buffer.attempted,
            output,
            epoch,
            width,
            height,
            start_us,
            gap_before_us: if buffer.previous_end == 0 {
                0
            } else {
                start_us.saturating_sub(buffer.previous_end)
            },
            ..Parent::default()
        };
        buffer.current = Some(index);
        ParentGuard { generation, index }
    })
}

fn update_parent(generation: u64, index: usize, f: impl FnOnce(&mut Parent)) {
    if !process_enabled() {
        return;
    }
    if generation == 0 {
        return;
    }
    BUFFER.with(|cell| {
        if let Some(buffer) = cell
            .borrow_mut()
            .as_mut()
            .filter(|b| b.generation == generation && b.current == Some(index))
        {
            f(&mut buffer.parents[index]);
        }
    });
}
fn with_current(f: impl FnOnce(&mut Parent)) {
    if !process_enabled() {
        return;
    }
    BUFFER.with(|cell| {
        if let Some(buffer) = cell.borrow_mut().as_mut() {
            if let Some(index) = buffer.current {
                f(&mut buffer.parents[index]);
            }
        }
    });
}

/// Caller-owned address into its prepared parent. Workers only receive a copy
/// for timing admission; they return samples and never dereference this address.
#[derive(Clone, Copy, Default)]
pub struct DecoderRequest {
    generation: u64,
    parent: usize,
    request: u8,
    queued_us: u64,
}

pub fn begin_decoder_request(
    layer_id: u64,
    project_render_epoch: u64,
    input_id: u64,
    position_ms: u64,
) -> DecoderRequest {
    if !process_enabled() {
        return DecoderRequest::default();
    }
    BUFFER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(buffer) = slot.as_mut() else {
            return DecoderRequest::default();
        };
        let Some(parent) = buffer.current else {
            return DecoderRequest::default();
        };
        let p = &mut buffer.parents[parent];
        let mut demand = p.stack;
        while demand != ROOT && p.children[usize::from(demand)].phase != Phase::Demand {
            demand = p.children[usize::from(demand)].parent;
        }
        if demand == ROOT {
            return DecoderRequest::default();
        }
        if usize::from(p.decoder_request_count) == MAX_DECODER_REQUESTS {
            p.flags |= PARTIAL | METADATA_OVERFLOW;
            return DecoderRequest::default();
        }
        let request = p.decoder_request_count;
        let start_us = relative_us(p);
        p.decoder_requests[usize::from(request)] = DecoderRequestSnapshot {
            layer_id,
            project_render_epoch,
            input_id,
            position_ms,
            start_us,
            demand_span_id: demand,
            ..DecoderRequestSnapshot::default()
        };
        p.decoder_request_count += 1;
        DecoderRequest {
            generation: buffer.generation,
            parent,
            request,
            queued_us: p.start_us.saturating_add(u64::from(start_us)),
        }
    })
}

/// Preparation executes in request order on the caller, before parallel jobs.
impl DecoderRequest {
    pub fn is_active(self) -> bool {
        self.generation != 0
    }

    pub fn state(
        self,
        previous_request_ms: Option<u64>,
        cached_pts_ms: Option<u64>,
        next_pts_ms: Option<u64>,
        width: u32,
        height: u32,
    ) {
        update_parent(self.generation, self.parent, |p| {
            let request = &mut p.decoder_requests[usize::from(self.request)];
            request.previous_request_ms = previous_request_ms;
            request.cached_pts_ms = cached_pts_ms;
            request.next_pts_ms = next_pts_ms;
            request.width = width;
            request.height = height;
        });
    }
    pub fn mark(self, paths: u8) {
        update_parent(self.generation, self.parent, |p| {
            p.decoder_requests[usize::from(self.request)].paths |= paths;
        });
    }

    pub fn queued(mut self) -> Self {
        if !process_enabled() {
            return self;
        }
        if self.generation != 0 {
            self.queued_us = now_us();
        }
        self
    }

    pub fn finish(self, selected_pts_ms: Option<u64>, cancelled: bool) {
        update_parent(self.generation, self.parent, |p| {
            let end_us = relative_us(p);
            let request = &mut p.decoder_requests[usize::from(self.request)];
            if request.complete {
                p.flags |= PARTIAL | INVALID_TREE;
            }
            request.end_us = end_us;
            request.selected_pts_ms = selected_pts_ms;
            request.cancelled = cancelled;
            request.complete = true;
        });
    }

    pub fn record_work(self, sample: DecoderWorkSample) {
        update_parent(self.generation, self.parent, |p| {
            let request = &mut p.decoder_requests[usize::from(self.request)];
            if sample.invalid || sample.worker_id == 0 || request.complete {
                p.flags |= PARTIAL | INVALID_TREE;
            }
            if request.work_count == 0 {
                request.work = sample;
            } else {
                let work = &mut request.work;
                work.last_worker_id = sample.last_worker_id;
                work.queued_us = work.queued_us.min(sample.queued_us);
                work.start_us = work.start_us.min(sample.start_us);
                work.end_us = work.end_us.max(sample.end_us);
                let totals = [
                    (&mut work.decode_io_us, sample.decode_io_us),
                    (&mut work.hardware_download_us, sample.hardware_download_us),
                    (&mut work.swscale_us, sample.swscale_us),
                    (&mut work.alloc_copy_us, sample.alloc_copy_us),
                ];
                for (total, extra) in totals {
                    match total.checked_add(extra) {
                        Some(sum) => *total = sum,
                        None => p.flags |= PARTIAL | TIME_OVERFLOW,
                    }
                }
            }
            match request.work_count.checked_add(1) {
                Some(count) => request.work_count = count,
                None => p.flags |= PARTIAL | METADATA_OVERFLOW,
            }
        });
    }

    pub fn work(self) -> DecoderWorkGuard {
        if !process_enabled() || self.generation == 0 {
            return DecoderWorkGuard::default();
        }
        let start_us = now_us();
        let worker_id = DECODER_WORKER_ID.with(|id| *id);
        let previous = DECODER_WORK.with(|work| {
            work.replace(Some(DecoderWorkState {
                sample: DecoderWorkSample {
                    worker_id,
                    last_worker_id: worker_id,
                    queued_us: self.queued_us,
                    start_us,
                    ..DecoderWorkSample::default()
                },
                phase_active: false,
            }))
        });
        if previous.is_some() {
            DECODER_WORK.with(|work| work.borrow_mut().as_mut().unwrap().sample.invalid = true);
        }
        DecoderWorkGuard {
            active: true,
            previous,
        }
    }
}

/// A per-layer provider cache hit links to the batch record in this parent.
/// It is an access record: no worker costs are counted a second time.
pub fn link_decoder_access(layer: u64, epoch: u64, input_id: u64, position_ms: u64) {
    if !process_enabled() {
        return;
    }
    let token = begin_decoder_request(layer, epoch, input_id, position_ms);
    update_parent(token.generation, token.parent, |p| {
        let source = p.decoder_requests[..usize::from(token.request)]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, request)| {
                request.layer_id == layer
                    && request.project_render_epoch == epoch
                    && request.input_id == input_id
                    && request.position_ms == position_ms
                    && request.paths & DECODER_PROVIDER == 0
            })
            .map(|(index, request)| (index as u8, *request));
        let end_us = relative_us(p);
        let request = &mut p.decoder_requests[usize::from(token.request)];
        request.paths = DECODER_PROVIDER;
        request.end_us = end_us;
        request.complete = true;
        if let Some((source_id, source)) = source {
            request.source_request_id = Some(source_id);
            request.selected_pts_ms = source.selected_pts_ms;
            request.cancelled = source.cancelled;
        }
    });
}

#[derive(Clone, Copy)]
struct DecoderWorkState {
    sample: DecoderWorkSample,
    phase_active: bool,
}

static NEXT_DECODER_WORKER: AtomicU64 = AtomicU64::new(1);
thread_local! {
    static DECODER_WORK: RefCell<Option<DecoderWorkState>> = const { RefCell::new(None) };
    static DECODER_WORKER_ID: u64 = NEXT_DECODER_WORKER.fetch_add(1, Ordering::Relaxed);
}

#[derive(Default)]
pub struct DecoderWorkGuard {
    active: bool,
    previous: Option<DecoderWorkState>,
}

impl DecoderWorkGuard {
    pub fn finish(mut self) -> DecoderWorkSample {
        if !self.active {
            return DecoderWorkSample::default();
        }
        let end_us = now_us();
        let mut sample = DECODER_WORK.with(|work| {
            let state = work.replace(self.previous.take()).unwrap();
            let mut sample = state.sample;
            sample.invalid |= state.phase_active;
            sample
        });
        self.active = false;
        sample.end_us = end_us;
        let total = end_us.saturating_sub(sample.start_us);
        let projection = u64::from(sample.hardware_download_us)
            + u64::from(sample.swscale_us)
            + u64::from(sample.alloc_copy_us);
        sample.invalid |= projection > total || total > u64::from(u32::MAX);
        sample.decode_io_us = total.saturating_sub(projection).min(u64::from(u32::MAX)) as u32;
        sample
    }
}

impl Drop for DecoderWorkGuard {
    fn drop(&mut self) {
        if self.active {
            DECODER_WORK.with(|work| work.replace(self.previous.take()));
        }
    }
}

#[derive(Clone, Copy)]
pub enum DecoderWorkPhase {
    HardwareDownload,
    Swscale,
    AllocCopy,
}

pub struct DecoderWorkPhaseGuard {
    phase: DecoderWorkPhase,
    start_us: Option<u64>,
}

pub fn work_phase(phase: DecoderWorkPhase) -> DecoderWorkPhaseGuard {
    let inactive = || DecoderWorkPhaseGuard {
        phase,
        start_us: None,
    };
    if !process_enabled() {
        return inactive();
    }
    DECODER_WORK.with(|work| {
        let mut work = work.borrow_mut();
        let Some(state) = work.as_mut() else {
            return inactive();
        };
        if state.phase_active {
            state.sample.invalid = true;
            return inactive();
        }
        state.phase_active = true;
        DecoderWorkPhaseGuard {
            phase,
            start_us: Some(now_us()),
        }
    })
}

impl Drop for DecoderWorkPhaseGuard {
    fn drop(&mut self) {
        let Some(start_us) = self.start_us else {
            return;
        };
        let elapsed = now_us().saturating_sub(start_us);
        DECODER_WORK.with(|work| {
            if let Some(state) = work.borrow_mut().as_mut() {
                let target = match self.phase {
                    DecoderWorkPhase::HardwareDownload => &mut state.sample.hardware_download_us,
                    DecoderWorkPhase::Swscale => &mut state.sample.swscale_us,
                    DecoderWorkPhase::AllocCopy => &mut state.sample.alloc_copy_us,
                };
                match u32::try_from(elapsed)
                    .ok()
                    .and_then(|elapsed| target.checked_add(elapsed))
                {
                    Some(sum) => *target = sum,
                    None => state.sample.invalid = true,
                }
                state.phase_active = false;
            }
        });
    }
}
pub fn set_outcome(outcome: Outcome) {
    with_current(|p| p.outcome = outcome);
}
pub fn set_requested_layers(layers: &[u64]) {
    with_current(|p| {
        if layers.len() > p.requested_layers.len() {
            p.flags |= PARTIAL | METADATA_OVERFLOW;
        }
        p.requested_count = layers.len().min(p.requested_layers.len()) as u8;
        p.requested_layers[..usize::from(p.requested_count)]
            .copy_from_slice(&layers[..usize::from(p.requested_count)]);
    });
}
pub fn add_retired_owner_for_layer(layer: u64) {
    with_current(|p| {
        if p.requested_layers[..usize::from(p.requested_count)].contains(&layer) {
            p.retired_requested += 1;
        } else {
            p.retired_other += 1;
        }
    });
}
pub fn set_demand_counters(before: [u64; 4], after: [u64; 4]) {
    with_current(|p| {
        p.demand_before = before;
        p.demand_after = after;
    });
}
pub fn set_isf_metadata(path: IsfPath, stage_count: u32, frame_index: u64) {
    set_isf_metadata_for_layer(path, 0, stage_count, frame_index);
}
pub fn set_isf_metadata_for_layer(
    path: IsfPath,
    layer_id: u64,
    stage_count: u32,
    frame_index: u64,
) {
    with_current(|p| {
        let index = usize::from(p.attempt_count);
        if index == MAX_ATTEMPTS {
            p.flags |= PARTIAL | METADATA_OVERFLOW;
            return;
        }
        p.attempts[index] = IsfAttempt {
            path,
            layer_id,
            stage_count,
            frame_index,
            span_id: p.stack,
        };
        p.attempt_count += 1;
    });
}
pub fn mark_isf_failed(frame_index: u64, layer_id: u64) {
    with_current(|p| {
        if p.failed[..usize::from(p.failed_count)].contains(&(frame_index, layer_id)) {
            return;
        }
        let index = usize::from(p.failed_count);
        if index == p.failed.len() {
            p.flags |= PARTIAL | METADATA_OVERFLOW;
            return;
        }
        p.failed[index] = (frame_index, layer_id);
        p.failed_count += 1;
    });
}
pub fn isf_single_path(frame_index: u64, layer_id: u64) -> IsfPath {
    let mut path = IsfPath::Ordinary;
    with_current(|p| {
        if p.failed[..usize::from(p.failed_count)].contains(&(frame_index, layer_id)) {
            path = IsfPath::Recovery;
        }
    });
    path
}

pub struct SpanGuard {
    generation: u64,
    parent: usize,
    child: u8,
}
fn relative_us(parent: &mut Parent) -> u32 {
    let elapsed = now_us().saturating_sub(parent.start_us);
    if elapsed > u64::from(u32::MAX) {
        parent.flags |= PARTIAL | TIME_OVERFLOW;
    }
    elapsed.min(u64::from(u32::MAX)) as u32
}
pub fn span(phase: Phase) -> SpanGuard {
    let inactive = || SpanGuard {
        generation: 0,
        parent: 0,
        child: ROOT,
    };
    if !process_enabled() {
        return inactive();
    }
    BUFFER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(buffer) = slot.as_mut() else {
            return inactive();
        };
        let Some(index) = buffer.current else {
            return inactive();
        };
        let p = &mut buffer.parents[index];
        if usize::from(p.child_count) == MAX_CHILDREN {
            p.flags |= PARTIAL | CHILD_OVERFLOW;
            return inactive();
        }
        let child = p.child_count;
        let start_us = relative_us(p);
        p.children[usize::from(child)] = Child {
            start_us,
            end_us: 0,
            parent: p.stack,
            phase,
            complete: false,
        };
        p.child_count += 1;
        p.stack = child;
        SpanGuard {
            generation: buffer.generation,
            parent: index,
            child,
        }
    })
}
impl Drop for SpanGuard {
    fn drop(&mut self) {
        update_parent(self.generation, self.parent, |p| {
            let end_us = relative_us(p);
            let child = &mut p.children[usize::from(self.child)];
            child.end_us = end_us;
            child.complete = true;
            if p.stack != self.child {
                p.flags |= PARTIAL | INVALID_TREE;
            }
            p.stack = child.parent;
        });
    }
}

#[derive(Debug, Serialize)]
pub struct ChildSnapshot {
    pub id: u8,
    pub parent_id: Option<u8>,
    pub phase: Phase,
    pub start_us: u32,
    pub end_us: u32,
    pub exclusive_us: u64,
}
#[derive(Debug, Serialize)]
pub struct ParentSnapshot {
    pub id: u64,
    pub output: u64,
    pub epoch: u64,
    pub width: u32,
    pub height: u32,
    pub start_us: u64,
    pub end_us: u64,
    pub start_monotonic_us: u64,
    pub end_monotonic_us: u64,
    pub start_unix_us: u64,
    pub end_unix_us: u64,
    pub gap_before_us: u64,
    pub outcome: Outcome,
    pub flags: u32,
    pub complete: bool,
    pub residual_us: u64,
    pub retired_requested: u32,
    pub retired_other: u32,
    pub demand_before: [u64; 4],
    pub demand_after: [u64; 4],
    pub attempts: Vec<IsfAttempt>,
    pub decoder_requests: Vec<DecoderRequestSnapshot>,
    pub children: Vec<ChildSnapshot>,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub active_capture: bool,
    pub generation: u64,
    pub start_monotonic_us: u64,
    pub start_unix_us: u64,
    pub clock_calibration_uncertainty_us: u64,
    pub stop_us: u64,
    pub retained_bytes: usize,
    pub attempted_parents: u64,
    pub overflow: bool,
    pub unprepared_parents: u64,
    pub first_parent_id: Option<u64>,
    pub last_parent_id: Option<u64>,
    pub parents: Vec<ParentSnapshot>,
}

fn export_parent(p: &Parent, control: &Control) -> ParentSnapshot {
    let duration = p.end_us.saturating_sub(p.start_us);
    let mut flags = p.flags;
    let source = &p.children[..usize::from(p.child_count)];
    let mut direct = [0_u64; MAX_CHILDREN];
    let mut root_total = 0_u64;
    for (index, child) in source.iter().enumerate() {
        if !child.complete || child.end_us < child.start_us || u64::from(child.end_us) > duration {
            flags |= PARTIAL | INVALID_TREE;
        }
        let elapsed = u64::from(child.end_us.saturating_sub(child.start_us));
        if child.parent == ROOT {
            root_total += elapsed;
        } else if usize::from(child.parent) < index {
            let parent = &source[usize::from(child.parent)];
            if child.start_us < parent.start_us || child.end_us > parent.end_us {
                flags |= PARTIAL | INVALID_TREE;
            }
            direct[usize::from(child.parent)] += elapsed;
        } else {
            flags |= PARTIAL | INVALID_TREE;
        }
        // Siblings are ordered by creation; reject overlapping intervals.
        if source[..index]
            .iter()
            .any(|previous| previous.parent == child.parent && previous.end_us > child.start_us)
        {
            flags |= PARTIAL | INVALID_TREE;
        }
    }
    if root_total > duration {
        flags |= PARTIAL | INVALID_TREE;
    }
    for (index, request) in p.decoder_requests[..usize::from(p.decoder_request_count)]
        .iter()
        .enumerate()
    {
        let demand = source.get(usize::from(request.demand_span_id));
        if !request.complete
            || request.end_us < request.start_us
            || !demand.is_some_and(|demand| {
                demand.phase == Phase::Demand
                    && request.start_us >= demand.start_us
                    && request.end_us <= demand.end_us
            })
            || request
                .source_request_id
                .is_some_and(|id| usize::from(id) >= index)
        {
            flags |= PARTIAL | INVALID_TREE;
        }
        if request.work_count != 0 {
            let work = &request.work;
            if work.invalid
                || work.queued_us > work.start_us
                || work.start_us > work.end_us
                || work.queued_us < p.start_us.saturating_add(u64::from(request.start_us))
                || work.end_us > p.start_us.saturating_add(u64::from(request.end_us))
            {
                flags |= PARTIAL | INVALID_TREE;
            }
        }
    }
    let children = source
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let duration = u64::from(c.end_us.saturating_sub(c.start_us));
            if direct[index] > duration {
                flags |= PARTIAL | INVALID_TREE;
            }
            ChildSnapshot {
                id: index as u8,
                parent_id: (c.parent != ROOT).then_some(c.parent),
                phase: c.phase,
                start_us: c.start_us,
                end_us: c.end_us,
                exclusive_us: duration.saturating_sub(direct[index]),
            }
        })
        .collect();
    ParentSnapshot {
        id: p.id,
        output: p.output,
        epoch: p.epoch,
        width: p.width,
        height: p.height,
        start_us: p.start_us,
        end_us: p.end_us,
        start_monotonic_us: control.start_us.saturating_add(p.start_us),
        end_monotonic_us: control.start_us.saturating_add(p.end_us),
        start_unix_us: control.start_unix_us.saturating_add(p.start_us),
        end_unix_us: control.start_unix_us.saturating_add(p.end_us),
        gap_before_us: p.gap_before_us,
        outcome: p.outcome,
        flags,
        complete: p.complete,
        residual_us: duration.saturating_sub(root_total),
        retired_requested: p.retired_requested,
        retired_other: p.retired_other,
        demand_before: p.demand_before,
        demand_after: p.demand_after,
        attempts: p.attempts[..usize::from(p.attempt_count)].to_vec(),
        decoder_requests: p.decoder_requests[..usize::from(p.decoder_request_count)].to_vec(),
        children,
    }
}

/// Allocations/formatting/serialization occur only on this cold, post-ACK path.
pub fn snapshot() -> Result<Snapshot, &'static str> {
    let control = CONTROL.lock().map_err(|_| "trace control poisoned")?;
    if ACTIVE.load(Ordering::Acquire) != 0 {
        return Err("capture is still running");
    }
    if !ACK.load(Ordering::Acquire) {
        return Err("Stop has not been acknowledged");
    }
    if OWNER.load(Ordering::Acquire) {
        return Err("trace owner has not retired");
    }
    let parents = control
        .buffer
        .as_ref()
        .map(|b| {
            b.parents[..b.len]
                .iter()
                .map(|p| export_parent(p, &control))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if parents.iter().any(|p| !p.complete) {
        return Err("capture contains an unfinished parent");
    }
    Ok(Snapshot {
        active_capture: control.generation != 0,
        generation: control.generation,
        start_monotonic_us: control.start_us,
        start_unix_us: control.start_unix_us,
        clock_calibration_uncertainty_us: control.clock_calibration_uncertainty_us,
        stop_us: control.stop_us,
        retained_bytes: if control.buffer.is_some() {
            RETAINED_BYTES
        } else {
            0
        },
        attempted_parents: control.buffer.as_ref().map_or(0, |b| b.attempted),
        overflow: control.buffer.as_ref().is_some_and(|b| b.overflow),
        unprepared_parents: UNPREPARED.load(Ordering::Relaxed),
        first_parent_id: parents.first().map(|p| p.id),
        last_parent_id: parents.last().map(|p| p.id),
        parents,
    })
}

#[cfg(test)]
pub(crate) static TIMING_TEST_SERIAL: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::TIMING_TEST_SERIAL as SERIAL;
    use super::*;
    fn start() -> Result<u64, &'static str> {
        enable_for_tests();
        super::start()
    }

    #[test]
    fn native_timing_disabled_startup_never_touches_tls() {
        const CHILD: &str = "SYNDOCAL_QA_DISABLED_TIMING_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native_frame_timing::tests::native_timing_disabled_startup_never_touches_tls",
                    "--test-threads=1",
                ])
                .env(CHILD, "1")
                .env_remove(OUTPUT_ENV)
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        configure_from_environment().unwrap();
        assert!(!process_enabled());
        let exercise_hooks = || {
            assert!(!recording_active());
            prepare_thread().unwrap();
            let parent = begin_parent(1, 2, 3, 4);
            parent.set_dimensions(5, 6);
            let _span = span(Phase::Prepare);
            with_current(|_| panic!("disabled metadata hook ran"));
            set_outcome(Outcome::Presented);
            set_requested_layers(&[1]);
            add_retired_owner_for_layer(1);
            set_demand_counters([0; 4], [1; 4]);
            let request = begin_decoder_request(1, 2, 3, 4);
            request.mark(DECODER_CACHED);
            request.state(Some(0), Some(33), None, 48, 27);
            request.mark(DECODER_REOPEN);
            let work = request.queued().work();
            drop(work_phase(DecoderWorkPhase::HardwareDownload));
            drop(work_phase(DecoderWorkPhase::Swscale));
            drop(work_phase(DecoderWorkPhase::AllocCopy));
            request.record_work(work.finish());
            request.finish(Some(33), false);
            link_decoder_access(1, 2, 3, 4);
            set_isf_metadata(IsfPath::Ordinary, 3, 4);
            set_isf_metadata_for_layer(IsfPath::Batch3, 1, 3, 4);
            mark_isf_failed(4, 1);
            assert_eq!(isf_single_path(4, 1), IsfPath::Ordinary);
            retire_thread();
            drop(ThreadRetirementGuard::new());
        };
        assert_eq!(BUFFER_INITIALIZATIONS.load(Ordering::Relaxed), 0);
        exercise_hooks();
        assert_eq!(BUFFER_INITIALIZATIONS.load(Ordering::Relaxed), 0);
        // Also prove no access when TLS already exists: any RefCell borrow by
        // a hook would panic while this exclusive borrow remains held.
        BUFFER.with(|cell| {
            let _exclusive = cell.borrow_mut();
            DECODER_WORK.with(|work| {
                let _work_exclusive = work.borrow_mut();
                exercise_hooks();
            });
        });
        assert_eq!(GENERATION.load(Ordering::Relaxed), 0);
        assert!(CLOCK.get().is_none());
        assert!(configure_from_environment().is_err());
        assert!(!process_enabled());
    }
    fn finish() -> Snapshot {
        stop_capture().unwrap();
        retire_thread();
        acknowledge_stop().unwrap();
        snapshot().unwrap()
    }

    #[test]
    fn native_timing_nesting_coverage_and_stop_guard() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        assert!(snapshot().is_err());
        {
            let _parent = begin_parent(1, 2, 1920, 1080);
            let _prepare = span(Phase::Prepare);
            {
                let _demand = span(Phase::Demand);
            }
            set_outcome(Outcome::Presented);
            assert!(snapshot().is_err());
        }
        stop_capture().unwrap();
        assert!(acknowledge_stop().is_err());
        assert!(snapshot().is_err());
        retire_thread();
        acknowledge_stop().unwrap();
        let snapshot = snapshot().unwrap();
        let p = &snapshot.parents[0];
        assert_eq!(p.flags, 0);
        assert_eq!(p.children[1].parent_id, Some(0));
        assert_eq!(
            p.residual_us + p.children.iter().map(|c| c.exclusive_us).sum::<u64>(),
            p.end_us - p.start_us
        );
        assert_eq!(p.outcome, Outcome::Presented);
    }

    #[test]
    fn native_timing_overflow_and_partial_are_explicit() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        {
            let _parent = begin_parent(0, 0, 1, 1);
            for _ in 0..=MAX_CHILDREN {
                let _span = span(Phase::Queue);
            }
        }
        for _ in 1..=MAX_PARENTS {
            let _parent = begin_parent(0, 0, 1, 1);
        }
        let snapshot = finish();
        assert!(snapshot.overflow);
        assert_eq!(snapshot.attempted_parents, MAX_PARENTS as u64 + 1);
        assert_eq!(snapshot.parents.len(), MAX_PARENTS);
        assert_ne!(snapshot.parents[0].flags & CHILD_OVERFLOW, 0);
        assert!(snapshot.retained_bytes <= MAX_RETAINED_BYTES);
    }

    #[test]
    fn native_timing_decoder_requests_return_explicit_worker_samples_and_provider_links() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        {
            let _parent = begin_parent(10, 20, 48, 27);
            let demand = span(Phase::Demand);
            let first = begin_decoder_request(1, 20, 100, 50);
            first.mark(DECODER_SYNC | DECODER_RESET);
            first.state(Some(33), Some(33), Some(66), 48, 27);
            let second = begin_decoder_request(2, 20, 200, 66);
            second.mark(DECODER_SYNC | DECODER_REOPEN);
            let run = |token: DecoderRequest| {
                let token = token.queued();
                std::thread::spawn(move || {
                    // A worker must not touch the render caller's Parent TLS.
                    BUFFER.with(|buffer| {
                        let _exclusive = buffer.borrow_mut();
                        let guard = token.work();
                        drop(work_phase(DecoderWorkPhase::HardwareDownload));
                        drop(work_phase(DecoderWorkPhase::Swscale));
                        drop(work_phase(DecoderWorkPhase::AllocCopy));
                        guard.finish()
                    })
                })
            };
            let first_worker = run(first);
            let second_worker = run(second);
            let second_sample = second_worker.join().unwrap();
            let first_sample = first_worker.join().unwrap();
            assert_ne!(first_sample.worker_id, second_sample.worker_id);
            second.record_work(second_sample);
            second.finish(Some(67), false);
            first.record_work(first_sample);
            first.finish(Some(66), false);
            drop(demand);
            {
                let _access = span(Phase::Demand);
                link_decoder_access(1, 20, 100, 50);
            }
            {
                let _access = span(Phase::Demand);
                let cancelled = begin_decoder_request(3, 20, 300, 0);
                cancelled.finish(None, true);
            }
        }
        let snapshot = finish();
        let parent = &snapshot.parents[0];
        assert_eq!(parent.flags, 0);
        assert_eq!(parent.decoder_requests.len(), 4);
        let first = &parent.decoder_requests[0];
        let second = &parent.decoder_requests[1];
        assert_eq!(first.paths, DECODER_SYNC | DECODER_RESET);
        assert_eq!(second.paths, DECODER_SYNC | DECODER_REOPEN);
        assert_eq!(first.previous_request_ms, Some(33));
        assert_eq!(first.cached_pts_ms, Some(33));
        assert_eq!(first.next_pts_ms, Some(66));
        assert_eq!(first.selected_pts_ms, Some(66));
        for request in [first, second] {
            assert_eq!(request.demand_span_id, 0);
            assert_eq!(request.work_count, 1);
            let work = request.work;
            assert_eq!(
                u64::from(work.decode_io_us)
                    + u64::from(work.hardware_download_us)
                    + u64::from(work.swscale_us)
                    + u64::from(work.alloc_copy_us),
                work.end_us - work.start_us
            );
        }
        let access = &parent.decoder_requests[2];
        assert_eq!(access.paths, DECODER_PROVIDER);
        assert_eq!(access.source_request_id, Some(0));
        assert_eq!(access.demand_span_id, 1);
        assert_eq!(access.selected_pts_ms, Some(66));
        assert_eq!(access.work_count, 0);
        assert!(parent.decoder_requests[3].cancelled);
        assert_eq!(
            parent.residual_us + parent.children.iter().map(|c| c.exclusive_us).sum::<u64>(),
            parent.end_us - parent.start_us
        );
        assert_eq!(snapshot.retained_bytes, RETAINED_BYTES);
        assert!(RETAINED_BYTES <= MAX_RETAINED_BYTES);
    }

    #[test]
    fn native_timing_decoder_metadata_overflow_and_unfinished_requests_are_invalid() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        {
            let _parent = begin_parent(0, 0, 1, 1);
            let _demand = span(Phase::Demand);
            for index in 0..=MAX_DECODER_REQUESTS {
                let request = begin_decoder_request(index as u64, 0, index as u64, 0);
                request.mark(DECODER_CACHED);
                request.finish(Some(0), false);
            }
        }
        {
            let _parent = begin_parent(0, 0, 1, 1);
            let _demand = span(Phase::Demand);
            let _unfinished = begin_decoder_request(1, 0, 1, 0);
        }
        let snapshot = finish();
        assert_eq!(
            snapshot.parents[0].decoder_requests.len(),
            MAX_DECODER_REQUESTS
        );
        assert_ne!(snapshot.parents[0].flags & METADATA_OVERFLOW, 0);
        assert_ne!(snapshot.parents[0].flags & PARTIAL, 0);
        assert_ne!(snapshot.parents[1].flags & INVALID_TREE, 0);
    }

    #[test]
    fn native_timing_cancel_reject_recovery_and_owner_counts() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        for outcome in [
            Outcome::Cancelled,
            Outcome::Rejected,
            Outcome::ZeroPresent,
            Outcome::Revoked,
        ] {
            let _parent = begin_parent(3, 4, 1, 1);
            set_outcome(outcome);
            set_requested_layers(&[8]);
            add_retired_owner_for_layer(8);
            add_retired_owner_for_layer(9);
            mark_isf_failed(7, 8);
            for _ in 0..4 {
                mark_isf_failed(7, 8);
            }
            assert_eq!(isf_single_path(7, 8), IsfPath::Recovery);
            assert_eq!(isf_single_path(7, 9), IsfPath::Ordinary);
            set_isf_metadata_for_layer(IsfPath::Recovery, 8, 3, 7);
        }
        let snapshot = finish();
        assert_eq!(snapshot.parents.len(), 4);
        for p in snapshot.parents {
            assert_eq!(p.flags, 0);
            assert_eq!(p.start_unix_us, snapshot.start_unix_us + p.start_us);
            assert_eq!(p.end_monotonic_us, snapshot.start_monotonic_us + p.end_us);
            assert_eq!((p.retired_requested, p.retired_other), (1, 1));
            assert_eq!(p.attempts[0].frame_index, 7);
        }
    }

    #[test]
    fn native_timing_active_zero_and_cold_owner_handoff() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        let empty = finish();
        assert!(empty.active_capture);
        assert!(empty.parents.is_empty());
        start().unwrap();
        prepare_thread().unwrap();
        {
            let _parent = begin_parent(1, 1, 1, 1);
        }
        retire_thread();
        std::thread::spawn(|| {
            prepare_thread().unwrap();
            {
                let _parent = begin_parent(2, 1, 1, 1);
            }
            retire_thread();
        })
        .join()
        .unwrap();
        assert_eq!(finish().parents.len(), 2);
    }

    #[test]
    fn native_timing_misnested_parent_rejected_and_stop_finishes_inflight() {
        let _serial = SERIAL.lock().unwrap();
        start().unwrap();
        prepare_thread().unwrap();
        {
            let _parent = begin_parent(0, 0, 1, 1);
            let a = span(Phase::Prepare);
            let b = span(Phase::Demand);
            drop(a);
            drop(b);
        }
        let parent = begin_parent(0, 0, 1, 1);
        stop_capture().unwrap();
        assert!(snapshot().is_err());
        {
            let _late_present = span(Phase::PresenterPresent);
            set_outcome(Outcome::Presented);
        }
        drop(parent);
        // Capture stop closes admission, not the already admitted parent's spans.
        drop(begin_parent(0, 0, 1, 1));
        retire_thread();
        acknowledge_stop().unwrap();
        let snapshot = snapshot().unwrap();
        assert_ne!(snapshot.parents[0].flags & INVALID_TREE, 0);
        assert_eq!(snapshot.parents.len(), 2);
        assert_eq!(snapshot.parents[1].flags, 0);
        assert_eq!(snapshot.parents[1].outcome, Outcome::Presented);
        assert_eq!(
            snapshot.parents[1].children[0].phase,
            Phase::PresenterPresent
        );
        assert_eq!(
            snapshot.parents[1].start_unix_us,
            snapshot.start_unix_us + snapshot.parents[1].start_us
        );
    }

    #[test]
    fn native_timing_startup_absent_is_inactive_and_bad_paths_are_rejected() {
        let _serial = SERIAL.lock().unwrap();
        let generation = GENERATION.load(Ordering::Acquire);
        assert!(automatic_from_value(None).unwrap().is_none());
        assert_eq!(GENERATION.load(Ordering::Acquire), generation);
        assert!(automatic_from_value(Some(OsString::from("relative.json"))).is_err());
    }

    #[test]
    fn native_timing_automatic_export_is_post_ack_once_and_preserves_existing_file() {
        let _serial = SERIAL.lock().unwrap();
        enable_for_tests();
        let output = std::env::temp_dir().join(format!(
            "syndocal-frame-timing-{}-{}.json",
            std::process::id(),
            absolute_us()
        ));
        let automatic = AutomaticCapture::new(output.clone()).unwrap();
        assert!(!automatic.export_once().unwrap());
        automatic.start_once().unwrap();
        let generation = GENERATION.load(Ordering::Acquire);
        automatic.start_once().unwrap();
        assert_eq!(GENERATION.load(Ordering::Acquire), generation);
        prepare_thread().unwrap();
        let parent = begin_parent(44, 1, 65, 3);
        {
            let _demand = span(Phase::Demand);
            let request = begin_decoder_request(44, 1, 9, 33);
            request.mark(DECODER_CACHED);
            request.finish(Some(33), false);
        }
        assert!(automatic.export_once().is_err());
        assert!(!output.exists());
        stop_capture().unwrap();
        assert!(automatic.export_once().is_err());
        {
            let _late = span(Phase::PresenterPresent);
            set_outcome(Outcome::Presented);
        }
        drop(parent);
        drop(begin_parent(44, 1, 65, 3));
        retire_thread();
        assert!(automatic.export_once().is_err());
        acknowledge_stop().unwrap();
        assert!(automatic.export_once().unwrap());
        let bytes = std::fs::read(&output).unwrap();
        let exported: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(exported["parents"].as_array().unwrap().len(), 1);
        assert_eq!(exported["parents"][0]["flags"], 0);
        assert_eq!(
            exported["parents"][0]["decoder_requests"][0]["paths"],
            DECODER_CACHED
        );
        assert_eq!(
            exported["parents"][0]["decoder_requests"][0]["selected_pts_ms"],
            33
        );
        assert_eq!(
            exported["parents"][0]["start_unix_us"].as_u64().unwrap(),
            exported["start_unix_us"].as_u64().unwrap()
                + exported["parents"][0]["start_us"].as_u64().unwrap()
        );
        assert!(!automatic.export_once().unwrap());
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
        assert!(AutomaticCapture::new(output.clone()).is_err());
        std::fs::remove_file(output).unwrap();
    }
}
