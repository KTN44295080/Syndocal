use std::path::PathBuf;

use protocol::{VideoLayerId, VideoSourceKind};

#[cfg(feature = "libav")]
use super::VideoPixelFormat;
use super::{
    render_input_cache_owner_matches, StillImageSignature, VideoDecodeError, VideoFrame,
    VideoFrameDecoder, VideoFrameRequest, VideoRenderInput,
};
#[cfg(feature = "libav")]
use ffmpeg_next as ffmpeg;

#[cfg(feature = "libav")]
#[path = "libav_hardware.rs"]
mod hardware;

#[cfg(all(feature = "libav", test))]
#[path = "libav_seek_projection_tests.rs"]
mod seek_projection_tests;

#[cfg(all(feature = "libav", test))]
#[path = "libav_catch_up_tests.rs"]
mod catch_up_tests;

#[cfg(all(feature = "libav", test))]
use std::cell::Cell;
#[cfg(feature = "libav")]
use std::cell::RefCell;

#[cfg(feature = "libav")]
// Two serialized output decodes can briefly exceed 250 ms during cold GPU
// startup. Reopening both sessions at that cadence sustains the slow path.
// Bound forward catch-up to one second; larger jumps and reversals still seek.
const LIBAV_SEQUENTIAL_REQUEST_GAP_MS: u64 = 1_000;
#[cfg(any(feature = "libav", test))]
const LIBAV_WORKING_SET_CAPACITY: usize = 8;

#[cfg(feature = "libav")]
const LIBAV_SCALER_CACHE_CAPACITY: usize = 8;

// `ffmpeg_next::software::scaling::context::Context` owns a native pointer and
// intentionally is not `Send`. Decoders, however, are moved into the native
// output worker, so the scaler cannot be stored in `LibavDecodeSession`
// without weakening that ownership boundary. Keep a bounded cache on the
// decoding thread instead. A scaler is keyed by the complete source/target
// geometry and pixel formats; it is therefore safe to reuse for any render
// input that has the same conversion contract while leaving the input/session
// ownership in `LibavDecodeSession` unchanged.
#[cfg(feature = "libav")]
thread_local! {
    static LIBAV_SCALER_CACHE: RefCell<Vec<LibavScalerCacheEntry>> = const {
        RefCell::new(Vec::new())
    };
}

#[cfg(all(feature = "libav", test))]
thread_local! {
    static LIBAV_SCALER_CREATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

pub struct LibavFrameDecoder {
    entries: Vec<LibavFrameCacheEntry>,
    working_set_lru: Vec<LibavRenderInputOwner>,
    #[cfg(feature = "libav")]
    sessions: Vec<LibavDecodeSession>,
    #[cfg(feature = "libav")]
    session_counters: LibavSessionCounters,
    #[cfg(feature = "libav")]
    hardware_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LibavRenderInputOwner {
    key: protocol::VideoRenderInputKey,
    legacy_layer_id: Option<VideoLayerId>,
}

impl LibavRenderInputOwner {
    #[cfg_attr(not(any(feature = "libav", test)), allow(dead_code))]
    fn new(key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) -> Self {
        Self {
            key,
            legacy_layer_id: (key == protocol::VideoRenderInputKey::LEGACY).then_some(layer_id),
        }
    }

    fn matches(self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) -> bool {
        render_input_cache_owner_matches(
            self.key,
            self.legacy_layer_id.unwrap_or(layer_id),
            key,
            layer_id,
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LibavSessionDiagnostics {
    pub active_sessions: usize,
    pub hardware_sessions: usize,
    pub hardware_frames: u64,
    pub hardware_errors: u64,
    pub opens: u64,
    pub resets: u64,
    pub sequential_continues: u64,
    pub frame_reuses: u64,
    pub evictions: u64,
    pub errors: u64,
}

#[derive(Clone)]
#[cfg_attr(not(feature = "libav"), allow(dead_code))]
struct LibavFrameCacheEntry {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    position_ms: u64,
    signature: StillImageSignature,
    frame: VideoFrame,
}

#[cfg(feature = "libav")]
struct LibavDecodeSession {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    signature: StillImageSignature,
    input: ffmpeg::format::context::Input,
    decoder: ffmpeg::decoder::Video,
    hardware: bool,
    stream_index: usize,
    time_base: ffmpeg::Rational,
    timestamp_origin: i64,
    last_request_ms: Option<u64>,
    eof_sent: bool,
    decoder_drained: bool,
}

#[cfg(feature = "libav")]
struct LibavScalerCacheEntry {
    source_format: ffmpeg::format::Pixel,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
    scaler: ffmpeg::software::scaling::context::Context,
}

#[cfg(feature = "libav")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LibavSessionCounters {
    hardware_frames: u64,
    hardware_errors: u64,
    opens: u64,
    resets: u64,
    sequential_continues: u64,
    frame_reuses: u64,
    evictions: u64,
    errors: u64,
}

#[cfg(feature = "libav")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibavSessionDecision {
    ReuseFrame,
    Continue,
    Reopen,
}

#[cfg(feature = "libav")]
struct LibavReceiveResult {
    frame: Option<VideoFrame>,
    decoder_drained: bool,
}

impl Default for LibavFrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LibavFrameDecoder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            working_set_lru: Vec::new(),
            #[cfg(feature = "libav")]
            sessions: Vec::new(),
            #[cfg(feature = "libav")]
            session_counters: LibavSessionCounters::default(),
            #[cfg(feature = "libav")]
            hardware_enabled: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn software_for_tests() -> Self {
        let decoder = Self::new();
        #[cfg(feature = "libav")]
        let decoder = Self {
            hardware_enabled: false,
            ..decoder
        };
        decoder
    }

    #[cfg(feature = "libav")]
    fn record_hardware_error(&mut self, error: &VideoDecodeError) {
        if matches!(error, VideoDecodeError::HardwareDecode { .. }) {
            self.session_counters.hardware_errors =
                self.session_counters.hardware_errors.saturating_add(1);
        }
    }

    pub const fn is_built() -> bool {
        cfg!(feature = "libav")
    }

    pub fn cache_len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn session_diagnostics(&self) -> LibavSessionDiagnostics {
        #[cfg(feature = "libav")]
        {
            LibavSessionDiagnostics {
                active_sessions: self.sessions.len(),
                hardware_sessions: self
                    .sessions
                    .iter()
                    .filter(|session| session.hardware)
                    .count(),
                hardware_frames: self.session_counters.hardware_frames,
                hardware_errors: self.session_counters.hardware_errors,
                opens: self.session_counters.opens,
                resets: self.session_counters.resets,
                sequential_continues: self.session_counters.sequential_continues,
                frame_reuses: self.session_counters.frame_reuses,
                evictions: self.session_counters.evictions,
                errors: self.session_counters.errors,
            }
        }
        #[cfg(not(feature = "libav"))]
        {
            LibavSessionDiagnostics::default()
        }
    }

    #[cfg(any(feature = "libav", test))]
    fn cached_frame(
        &self,
        key: protocol::VideoRenderInputKey,
        request: &VideoFrameRequest,
        path: &std::path::Path,
        signature: &StillImageSignature,
    ) -> Option<VideoFrame> {
        self.entries
            .iter()
            .find(|entry| {
                entry.key == key
                    && entry.layer_id == request.layer_id
                    && entry.path == path
                    && entry.width == request.width
                    && entry.height == request.height
                    && entry.position_ms == request.position_ms
                    && entry.signature == *signature
            })
            .map(|entry| entry.frame.clone())
    }

    #[cfg(any(feature = "libav", test))]
    fn replace_layer_cache_entry(&mut self, entry: LibavFrameCacheEntry) {
        self.touch_input(entry.key, entry.layer_id);
        self.entries.retain(|cached| {
            !render_input_cache_owner_matches(
                entry.key,
                entry.layer_id,
                cached.key,
                cached.layer_id,
            )
        });
        self.entries.push(entry);
    }

    fn evict_input_state(&mut self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) {
        self.entries.retain(|entry| {
            !render_input_cache_owner_matches(key, layer_id, entry.key, entry.layer_id)
        });
        #[cfg(feature = "libav")]
        self.sessions.retain(|session| {
            !render_input_cache_owner_matches(key, layer_id, session.key, session.layer_id)
        });
    }

    pub(crate) fn release_layer(&mut self, layer_id: VideoLayerId) {
        self.entries.retain(|entry| entry.layer_id != layer_id);
        #[cfg(feature = "libav")]
        self.sessions.retain(|session| session.layer_id != layer_id);
        let active_owners = self
            .entries
            .iter()
            .map(|entry| LibavRenderInputOwner::new(entry.key, entry.layer_id))
            .collect::<Vec<_>>();
        self.working_set_lru
            .retain(|owner| active_owners.contains(owner));
    }

    fn release_input_key(&mut self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) {
        self.evict_input_state(key, layer_id);
        self.working_set_lru
            .retain(|cached| !cached.matches(key, layer_id));
    }

    #[cfg(any(feature = "libav", test))]
    fn touch_input(&mut self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) {
        let owner = LibavRenderInputOwner::new(key, layer_id);
        self.working_set_lru.retain(|cached| *cached != owner);
        self.working_set_lru.push(owner);
        while self.working_set_lru.len() > LIBAV_WORKING_SET_CAPACITY {
            let evicted = self.working_set_lru.remove(0);
            self.evict_input_state(evicted.key, evicted.legacy_layer_id.unwrap_or(0));
            #[cfg(feature = "libav")]
            {
                self.session_counters.evictions = self.session_counters.evictions.saturating_add(1);
            }
        }
    }

    #[cfg(feature = "libav")]
    fn session_index(
        &self,
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
    ) -> Option<usize> {
        self.sessions.iter().position(|session| {
            render_input_cache_owner_matches(key, layer_id, session.key, session.layer_id)
        })
    }

    #[cfg(feature = "libav")]
    fn decode_enabled(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let key = input.key;
        let request = &input.request;
        let Some(path) = request
            .source
            .path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
        else {
            self.release_input_key(key, request.layer_id);
            self.session_counters.errors = self.session_counters.errors.saturating_add(1);
            return Err(VideoDecodeError::MissingSourcePath {
                layer_id: request.layer_id,
                label: request.label.clone(),
            });
        };
        let path = PathBuf::from(path);
        let signature = StillImageSignature::from_path(&path);
        if let Some(frame) = self.cached_frame(key, request, &path, &signature) {
            self.touch_input(key, request.layer_id);
            self.session_counters.frame_reuses =
                self.session_counters.frame_reuses.saturating_add(1);
            return Ok(Some(frame));
        }

        let cached_entry = self
            .entries
            .iter()
            .find(|entry| {
                render_input_cache_owner_matches(key, request.layer_id, entry.key, entry.layer_id)
            })
            .cloned();
        let session_index = self.session_index(key, request.layer_id);
        let session_matches = session_index
            .is_some_and(|index| self.sessions[index].matches(request, &path, &signature));
        let decision = if session_matches {
            session_decision(
                self.sessions[session_index.expect("matching session index")].last_request_ms,
                cached_entry.as_ref().map(|entry| entry.frame.pts_ms),
                cached_entry.is_some(),
                self.sessions[session_index.expect("matching session index")].decoder_drained,
                request.position_ms,
            )
        } else {
            LibavSessionDecision::Reopen
        };

        if decision == LibavSessionDecision::ReuseFrame {
            let mut entry = cached_entry.expect("frame reuse requires a cached frame");
            entry.position_ms = request.position_ms;
            let frame = entry.frame.clone();
            self.replace_layer_cache_entry(entry);
            let index = session_index.expect("frame reuse requires a matching session");
            self.sessions[index].last_request_ms = Some(request.position_ms);
            self.session_counters.frame_reuses =
                self.session_counters.frame_reuses.saturating_add(1);
            return Ok(Some(frame));
        }

        let session_index = if decision == LibavSessionDecision::Continue {
            self.session_counters.sequential_continues =
                self.session_counters.sequential_continues.saturating_add(1);
            session_index.expect("continuation requires a matching session")
        } else {
            if session_index.is_some() {
                self.session_counters.resets = self.session_counters.resets.saturating_add(1);
            }
            self.release_input_key(key, request.layer_id);
            self.touch_input(key, request.layer_id);
            let session = match open_libav_session(
                key,
                request,
                path.clone(),
                signature.clone(),
                self.hardware_enabled,
            ) {
                Ok(session) => session,
                Err(error) => {
                    self.session_counters.errors = self.session_counters.errors.saturating_add(1);
                    self.release_input_key(key, request.layer_id);
                    self.record_hardware_error(&error);
                    return Err(error);
                }
            };
            self.sessions.push(session);
            self.session_counters.opens = self.session_counters.opens.saturating_add(1);
            self.sessions.len() - 1
        };

        let seek_before_decode = decision == LibavSessionDecision::Reopen;
        let terminal_frame = (decision == LibavSessionDecision::Continue)
            .then(|| cached_entry.as_ref().map(|entry| &entry.frame))
            .flatten();
        let primary_decode = decode_libav_session_frame(
            request,
            &mut self.sessions[session_index],
            seek_before_decode,
            terminal_frame,
            &mut self.session_counters.hardware_frames,
        );
        let frame = match primary_decode {
            Ok(frame) => frame,
            Err(ref error)
                if !matches!(error, VideoDecodeError::HardwareDecode { .. })
                    && seek_before_decode
                    && request.position_ms > 0 =>
            {
                let mut fallback_session = match open_libav_session(
                    key,
                    request,
                    path.clone(),
                    signature.clone(),
                    self.hardware_enabled,
                ) {
                    Ok(session) => session,
                    Err(error) => {
                        self.session_counters.errors =
                            self.session_counters.errors.saturating_add(1);
                        self.release_input_key(key, request.layer_id);
                        self.record_hardware_error(&error);
                        return Err(error);
                    }
                };
                self.session_counters.opens = self.session_counters.opens.saturating_add(1);
                match decode_libav_session_frame(
                    request,
                    &mut fallback_session,
                    false,
                    None,
                    &mut self.session_counters.hardware_frames,
                ) {
                    Ok(frame) => {
                        self.sessions[session_index] = fallback_session;
                        frame
                    }
                    Err(error) => {
                        self.session_counters.errors =
                            self.session_counters.errors.saturating_add(1);
                        self.release_input_key(key, request.layer_id);
                        self.record_hardware_error(&error);
                        return Err(error);
                    }
                }
            }
            Err(error) => {
                self.session_counters.errors = self.session_counters.errors.saturating_add(1);
                self.release_input_key(key, request.layer_id);
                self.record_hardware_error(&error);
                return Err(error);
            }
        };
        self.replace_layer_cache_entry(LibavFrameCacheEntry {
            key,
            layer_id: request.layer_id,
            path,
            width: request.width,
            height: request.height,
            position_ms: request.position_ms,
            signature,
            frame: frame.clone(),
        });
        Ok(Some(frame))
    }
}

impl VideoFrameDecoder for LibavFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.entries
            .retain(|entry| layer_ids.contains(&entry.layer_id));
        let active_owners = self
            .entries
            .iter()
            .map(|entry| LibavRenderInputOwner::new(entry.key, entry.layer_id))
            .collect::<Vec<_>>();
        self.working_set_lru
            .retain(|owner| active_owners.contains(owner));
        #[cfg(feature = "libav")]
        self.sessions
            .retain(|session| layer_ids.contains(&session.layer_id));
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.entries.retain(|entry| {
            inputs.iter().any(|input| {
                render_input_cache_owner_matches(
                    input.key,
                    input.layer_id(),
                    entry.key,
                    entry.layer_id,
                )
            })
        });
        self.working_set_lru.retain(|owner| {
            inputs
                .iter()
                .any(|input| owner.matches(input.key, input.layer_id()))
        });
        #[cfg(feature = "libav")]
        self.sessions.retain(|session| {
            inputs.iter().any(|input| {
                render_input_cache_owner_matches(
                    input.key,
                    input.layer_id(),
                    session.key,
                    session.layer_id,
                )
            })
        });
    }

    fn release_layer(&mut self, layer_id: VideoLayerId) {
        LibavFrameDecoder::release_layer(self, layer_id);
    }

    fn release_input(&mut self, input: &VideoRenderInput) {
        self.release_input_key(input.key, input.layer_id());
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input_frame(&VideoRenderInput::legacy(request.clone()))
    }

    fn decode_input_frame(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        if input.request.source.kind != VideoSourceKind::File {
            self.release_input_key(input.key, input.layer_id());
            return Ok(None);
        }
        #[cfg(feature = "libav")]
        {
            self.decode_enabled(input)
        }
        #[cfg(not(feature = "libav"))]
        {
            Ok(None)
        }
    }
}

#[cfg(feature = "libav")]
impl LibavDecodeSession {
    fn matches(
        &self,
        request: &VideoFrameRequest,
        path: &std::path::Path,
        signature: &StillImageSignature,
    ) -> bool {
        self.layer_id == request.layer_id
            && self.path == path
            && self.width == request.width
            && self.height == request.height
            && self.signature == *signature
    }
}

#[cfg(feature = "libav")]
fn session_decision(
    last_request_ms: Option<u64>,
    cached_frame_pts_ms: Option<u64>,
    has_cached_frame: bool,
    decoder_drained: bool,
    target_ms: u64,
) -> LibavSessionDecision {
    let Some(last_request_ms) = last_request_ms else {
        return LibavSessionDecision::Reopen;
    };
    if !has_cached_frame {
        return LibavSessionDecision::Reopen;
    }
    if target_ms == last_request_ms
        || (target_ms > last_request_ms
            && cached_frame_pts_ms.is_some_and(|pts_ms| target_ms <= pts_ms))
        || (target_ms > last_request_ms && decoder_drained)
    {
        return LibavSessionDecision::ReuseFrame;
    }
    if target_ms > last_request_ms
        && target_ms.saturating_sub(last_request_ms) <= LIBAV_SEQUENTIAL_REQUEST_GAP_MS
    {
        return LibavSessionDecision::Continue;
    }
    LibavSessionDecision::Reopen
}

#[cfg(feature = "libav")]
fn open_libav_session(
    key: protocol::VideoRenderInputKey,
    request: &VideoFrameRequest,
    path: PathBuf,
    signature: StillImageSignature,
    hardware_enabled: bool,
) -> Result<LibavDecodeSession, VideoDecodeError> {
    use ffmpeg::media::Type;
    ffmpeg::init().map_err(|error| decode_error(request, error))?;
    let input = ffmpeg::format::input(&path).map_err(|error| decode_error(request, error))?;
    let stream = input
        .streams()
        .best(Type::Video)
        .ok_or_else(|| decode_message(request, "video stream not found"))?;
    let stream_index = stream.index();
    let time_base = stream.time_base();
    let timestamp_origin = match stream.start_time() {
        ffmpeg::ffi::AV_NOPTS_VALUE => 0,
        timestamp => timestamp,
    };
    let mut context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|error| decode_error(request, error))?;
    let hardware = hardware::configure(&mut context, request, hardware_enabled)?;
    let mut decoder = context
        .decoder()
        .video()
        .map_err(|error| hardware::classify(request, hardware, decode_error(request, error)))?;
    decoder.flush();
    Ok(LibavDecodeSession {
        key,
        layer_id: request.layer_id,
        path,
        width: request.width,
        height: request.height,
        signature,
        input,
        decoder,
        hardware,
        stream_index,
        time_base,
        timestamp_origin,
        last_request_ms: None,
        eof_sent: false,
        decoder_drained: false,
    })
}

#[cfg(feature = "libav")]
struct LibavScalerLease {
    source_format: ffmpeg::format::Pixel,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
    scaler: Option<ffmpeg::software::scaling::context::Context>,
}

#[cfg(feature = "libav")]
impl LibavScalerLease {
    /// Run the conversion while retaining ownership of the cache lease.
    ///
    /// A failed native conversion can leave the underlying scaler in an
    /// unusable state. Do not return that context to the thread-local cache;
    /// the next request for this geometry must create a fresh conversion
    /// context instead.
    fn run(
        &mut self,
        input: &ffmpeg_next::util::frame::video::Video,
        output: &mut ffmpeg_next::util::frame::video::Video,
    ) -> Result<(), ffmpeg_next::Error> {
        let result = self
            .scaler
            .as_mut()
            .expect("scaler lease must own a scaler until drop")
            .run(input, output);
        if result.is_err() {
            self.scaler = None;
        }
        result
    }
}

#[cfg(feature = "libav")]
impl Drop for LibavScalerLease {
    fn drop(&mut self) {
        let Some(scaler) = self.scaler.take() else {
            return;
        };
        LIBAV_SCALER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.retain(|entry| {
                entry.source_format != self.source_format
                    || entry.source_width != self.source_width
                    || entry.source_height != self.source_height
                    || entry.target_width != self.target_width
                    || entry.target_height != self.target_height
            });
            if cache.len() >= LIBAV_SCALER_CACHE_CAPACITY {
                cache.remove(0);
            }
            cache.push(LibavScalerCacheEntry {
                source_format: self.source_format,
                source_width: self.source_width,
                source_height: self.source_height,
                target_width: self.target_width,
                target_height: self.target_height,
                scaler,
            });
        });
    }
}

#[cfg(feature = "libav")]
fn acquire_cached_libav_scaler(
    request: &VideoFrameRequest,
    source_format: ffmpeg::format::Pixel,
    source_width: u32,
    source_height: u32,
) -> Result<LibavScalerLease, VideoDecodeError> {
    use ffmpeg::{
        format::Pixel,
        software::scaling::{context::Context, flag::Flags},
    };

    LIBAV_SCALER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let target_width = request.width;
        let target_height = request.height;
        let scaler = if let Some(cache_index) = cache.iter().position(|entry| {
            entry.source_format == source_format
                && entry.source_width == source_width
                && entry.source_height == source_height
                && entry.target_width == target_width
                && entry.target_height == target_height
        }) {
            cache.remove(cache_index).scaler
        } else {
            #[cfg(all(feature = "libav", test))]
            LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(count.get() + 1));
            Context::get(
                source_format,
                source_width,
                source_height,
                Pixel::RGBA,
                target_width,
                target_height,
                Flags::BILINEAR,
            )
            .map_err(|error| decode_error(request, error))?
        };
        Ok(LibavScalerLease {
            source_format,
            source_width,
            source_height,
            target_width,
            target_height,
            scaler: Some(scaler),
        })
    })
}

#[cfg(feature = "libav")]
fn decode_libav_session_frame(
    request: &VideoFrameRequest,
    session: &mut LibavDecodeSession,
    seek_before_decode: bool,
    terminal_frame: Option<&VideoFrame>,
    hardware_frames: &mut u64,
) -> Result<VideoFrame, VideoDecodeError> {
    let hardware = session.hardware;
    decode_libav_session_frame_inner(
        request,
        session,
        seek_before_decode,
        terminal_frame,
        hardware_frames,
    )
    .map_err(|error| hardware::classify(request, hardware, error))
}

#[cfg(feature = "libav")]
fn decode_libav_session_frame_inner(
    request: &VideoFrameRequest,
    session: &mut LibavDecodeSession,
    seek_before_decode: bool,
    terminal_frame: Option<&VideoFrame>,
    hardware_frames: &mut u64,
) -> Result<VideoFrame, VideoDecodeError> {
    let target_offset_us = request
        .position_ms
        .saturating_mul(1_000)
        .min(i64::MAX as u64) as i64;
    let target_us = target_offset_us;
    if seek_before_decode && request.position_ms > 0 && session.input.seek(target_us, ..).is_ok() {
        session.decoder.flush();
    }
    let mut candidate = None;

    let received = receive_target_frame(
        request,
        &mut session.decoder,
        session.hardware,
        hardware_frames,
        session.time_base,
        session.timestamp_origin,
        &mut candidate,
    )?;
    session.decoder_drained = received.decoder_drained;
    if let Some(frame) = received.frame {
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }
    if session.decoder_drained {
        let frame = finish_decoded_candidate(
            request,
            candidate,
            terminal_frame,
            session.hardware,
            hardware_frames,
        )?;
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }

    for (packet_stream, packet) in session.input.packets() {
        if packet_stream.index() != session.stream_index {
            continue;
        }
        session
            .decoder
            .send_packet(&packet)
            .map_err(|error| decode_error(request, error))?;
        let received = receive_target_frame(
            request,
            &mut session.decoder,
            session.hardware,
            hardware_frames,
            session.time_base,
            session.timestamp_origin,
            &mut candidate,
        )?;
        session.decoder_drained = received.decoder_drained;
        if let Some(frame) = received.frame {
            session.last_request_ms = Some(request.position_ms);
            return Ok(frame);
        }
    }
    if !session.eof_sent {
        session
            .decoder
            .send_eof()
            .map_err(|error| decode_error(request, error))?;
        session.eof_sent = true;
    }
    let received = receive_target_frame(
        request,
        &mut session.decoder,
        session.hardware,
        hardware_frames,
        session.time_base,
        session.timestamp_origin,
        &mut candidate,
    )?;
    session.decoder_drained = received.decoder_drained;
    if let Some(frame) = received.frame {
        session.last_request_ms = Some(request.position_ms);
        return Ok(frame);
    }
    let frame = finish_decoded_candidate(
        request,
        candidate,
        terminal_frame,
        session.hardware,
        hardware_frames,
    )?;
    session.last_request_ms = Some(request.position_ms);
    Ok(frame)
}

#[cfg(feature = "libav")]
struct DecodedCandidate {
    frame: ffmpeg::util::frame::video::Video,
    pts_ms: u64,
}

#[cfg(feature = "libav")]
fn project_decoded_frame(
    request: &VideoFrameRequest,
    decoded: &ffmpeg::util::frame::video::Video,
    pts_ms: u64,
    hardware: bool,
    hardware_frames: &mut u64,
) -> Result<VideoFrame, VideoDecodeError> {
    let transferred = hardware::transfer(request, hardware, decoded)?;
    if hardware {
        *hardware_frames = hardware_frames.saturating_add(1);
    }
    let source = transferred.as_ref().unwrap_or(decoded);
    let mut scaler =
        acquire_cached_libav_scaler(request, source.format(), source.width(), source.height())?;
    let mut rgba = ffmpeg::util::frame::video::Video::empty();
    scaler
        .run(source, &mut rgba)
        .map_err(|error| decode_error(request, error))?;
    copy_rgba_frame(request, &rgba, pts_ms)
}

#[cfg(feature = "libav")]
fn finish_decoded_candidate(
    request: &VideoFrameRequest,
    candidate: Option<DecodedCandidate>,
    terminal_frame: Option<&VideoFrame>,
    hardware: bool,
    hardware_frames: &mut u64,
) -> Result<VideoFrame, VideoDecodeError> {
    match candidate {
        Some(candidate) => project_decoded_frame(
            request,
            &candidate.frame,
            candidate.pts_ms,
            hardware,
            hardware_frames,
        ),
        None => terminal_frame
            .cloned()
            .ok_or_else(|| decode_message(request, "decoder produced no video frame")),
    }
}

#[cfg(feature = "libav")]
fn receive_target_frame(
    request: &VideoFrameRequest,
    decoder: &mut ffmpeg_next::decoder::Video,
    hardware: bool,
    hardware_frames: &mut u64,
    time_base: ffmpeg_next::Rational,
    timestamp_origin: i64,
    candidate: &mut Option<DecodedCandidate>,
) -> Result<LibavReceiveResult, VideoDecodeError> {
    use ffmpeg_next::error::EAGAIN;
    use ffmpeg_next::util::frame::video::Video;

    let mut decoded = Video::empty();
    loop {
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                let pts_ms = match decoded.timestamp().and_then(|timestamp| {
                    normalized_timestamp_ms(timestamp, timestamp_origin, time_base)
                }) {
                    Some(pts_ms) if pts_ms >= 0 => pts_ms as u64,
                    Some(_) => continue,
                    None => request.position_ms,
                };
                // Even discarded frames must honor the selected HW format.
                if hardware && decoded.format() != ffmpeg::format::Pixel::D3D11 {
                    return Err(hardware::classify(
                        request,
                        true,
                        decode_message(request, "decoder returned a non-D3D11 frame"),
                    ));
                }
                if pts_ms >= request.position_ms {
                    let frame = project_decoded_frame(
                        request,
                        &decoded,
                        pts_ms,
                        hardware,
                        hardware_frames,
                    )?;
                    return Ok(LibavReceiveResult {
                        frame: Some(frame),
                        decoder_drained: false,
                    });
                }
                // Keep only the latest decoder-owned frame as the EOF fallback.
                // Moving its AVFrame retains one surface, not a CPU pixel copy;
                // replaced candidates are released on each iteration.
                *candidate = Some(DecodedCandidate {
                    frame: std::mem::replace(&mut decoded, Video::empty()),
                    pts_ms,
                });
            }
            Err(ffmpeg_next::Error::Eof) => {
                return Ok(LibavReceiveResult {
                    frame: None,
                    decoder_drained: true,
                });
            }
            Err(ffmpeg_next::Error::Other { errno }) if errno == EAGAIN => {
                return Ok(LibavReceiveResult {
                    frame: None,
                    decoder_drained: false,
                });
            }
            Err(error) => return Err(decode_error(request, error)),
        }
    }
}

#[cfg(feature = "libav")]
fn copy_rgba_frame(
    request: &VideoFrameRequest,
    frame: &ffmpeg_next::util::frame::video::Video,
    pts_ms: u64,
) -> Result<VideoFrame, VideoDecodeError> {
    let row_bytes = (request.width as usize)
        .checked_mul(4)
        .ok_or_else(|| decode_message(request, "RGBA row size overflow"))?;
    let data_len = row_bytes
        .checked_mul(request.height as usize)
        .ok_or_else(|| decode_message(request, "RGBA frame size overflow"))?;
    let source = frame.data(0);
    let stride = frame.stride(0);
    let mut data = vec![0; data_len];
    for row in 0..request.height as usize {
        let source_start = row
            .checked_mul(stride)
            .ok_or_else(|| decode_message(request, "RGBA source offset overflow"))?;
        let source_end = source_start
            .checked_add(row_bytes)
            .ok_or_else(|| decode_message(request, "RGBA source row overflow"))?;
        let destination_start = row * row_bytes;
        let source_row = source
            .get(source_start..source_end)
            .ok_or_else(|| decode_message(request, "RGBA source stride is too small"))?;
        data[destination_start..destination_start + row_bytes].copy_from_slice(source_row);
    }
    let duration_ms = request
        .source
        .metadata
        .and_then(|metadata| metadata.frame_rate)
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .map(|rate| (1_000.0 / rate).round().max(1.0) as u64)
        .unwrap_or(33);
    Ok(VideoFrame {
        layer_id: request.layer_id,
        width: request.width,
        height: request.height,
        pts_ms,
        duration_ms,
        format: VideoPixelFormat::Rgba8,
        data,
    })
}

#[cfg(feature = "libav")]
fn normalized_timestamp_ms(
    timestamp: i64,
    timestamp_origin: i64,
    time_base: ffmpeg_next::Rational,
) -> Option<i64> {
    let normalized = timestamp.checked_sub(timestamp_origin)?;
    rescale_timestamp(normalized, time_base, 1_000)
}

#[cfg(feature = "libav")]
fn rescale_timestamp(
    timestamp: i64,
    time_base: ffmpeg_next::Rational,
    units_per_second: i128,
) -> Option<i64> {
    let numerator = i128::from(time_base.numerator());
    let denominator = i128::from(time_base.denominator());
    if numerator <= 0 || denominator <= 0 {
        return None;
    }
    let value = i128::from(timestamp)
        .checked_mul(numerator)?
        .checked_mul(units_per_second)?
        .checked_div_euclid(denominator)?;
    i64::try_from(value).ok()
}

#[cfg(feature = "libav")]
fn decode_error(request: &VideoFrameRequest, error: impl std::fmt::Display) -> VideoDecodeError {
    decode_message(request, error.to_string())
}

#[cfg(feature = "libav")]
fn decode_message(request: &VideoFrameRequest, message: impl Into<String>) -> VideoDecodeError {
    VideoDecodeError::Decode {
        layer_id: request.layer_id,
        label: request.label.clone(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use protocol::{VideoSourceKind, VideoSourceSummary};

    use super::*;
    #[cfg(feature = "libav")]
    use crate::{FfmpegCliFrameDecoder, PreferredVideoFrameDecoder, VideoDecoderDiagnostics};

    fn request() -> VideoFrameRequest {
        VideoFrameRequest {
            layer_id: 1,
            label: "General codec".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("clip.mp4".to_string()),
                name: None,
                codec: Some("h264".to_string()),
                metadata: None,
            },
            position_ms: 0,
            width: 16,
            height: 16,
        }
    }

    fn signature(token: u64) -> StillImageSignature {
        StillImageSignature {
            len: Some(token),
            modified: None,
        }
    }

    fn cache_entry(
        request: &VideoFrameRequest,
        path: impl Into<PathBuf>,
        signature: StillImageSignature,
        token: u8,
    ) -> LibavFrameCacheEntry {
        LibavFrameCacheEntry {
            key: protocol::VideoRenderInputKey::LEGACY,
            layer_id: request.layer_id,
            path: path.into(),
            width: request.width,
            height: request.height,
            position_ms: request.position_ms,
            signature,
            frame: VideoFrame {
                layer_id: request.layer_id,
                width: request.width,
                height: request.height,
                pts_ms: request.position_ms,
                duration_ms: 33,
                format: crate::VideoPixelFormat::Rgba8,
                data: vec![token],
            },
        }
    }

    #[cfg(feature = "libav")]
    fn assert_seek_parity(actual: &VideoFrame, expected: &VideoFrame, context: &str) {
        assert_eq!(actual.layer_id, expected.layer_id, "{context}: layer");
        assert_eq!(actual.width, expected.width, "{context}: width");
        assert_eq!(actual.height, expected.height, "{context}: height");
        assert_eq!(actual.pts_ms, expected.pts_ms, "{context}: PTS");
        assert_eq!(
            actual.duration_ms, expected.duration_ms,
            "{context}: duration"
        );
        assert_eq!(actual.format, expected.format, "{context}: format");
        assert_eq!(
            actual.data.len(),
            expected.data.len(),
            "{context}: data length"
        );
        let mut max_rgb_delta = 0_u8;
        let mut rgb_delta_total = 0_u64;
        let mut changed_rgb_bytes = 0_usize;
        for (index, (actual, expected)) in actual.data.iter().zip(&expected.data).enumerate() {
            let delta = actual.abs_diff(*expected);
            if index % 4 == 3 {
                assert_eq!(delta, 0, "{context}: alpha differs at byte {index}");
            } else {
                max_rgb_delta = max_rgb_delta.max(delta);
                rgb_delta_total += u64::from(delta);
                changed_rgb_bytes += usize::from(delta > 0);
            }
        }
        let rgb_bytes = actual.data.len() / 4 * 3;
        let mean_rgb_delta = rgb_delta_total as f64 / rgb_bytes as f64;
        assert!(
            max_rgb_delta <= 2 && mean_rgb_delta <= 0.1 && changed_rgb_bytes * 20 <= rgb_bytes,
            "{context}: max RGB delta {max_rgb_delta}, mean {mean_rgb_delta:.4}, changed {changed_rgb_bytes}/{rgb_bytes}"
        );
    }

    #[cfg(feature = "libav")]
    fn assert_rgba_reference_parity(actual: &VideoFrame, expected: &[u8], context: &str) {
        assert_eq!(actual.data.len(), expected.len(), "{context}: data length");
        let mut max_rgb_delta = 0_u8;
        let mut rgb_delta_total = 0_u64;
        let mut changed_rgb_bytes = 0_usize;
        for (index, (actual, expected)) in actual.data.iter().zip(expected).enumerate() {
            let delta = actual.abs_diff(*expected);
            if index % 4 == 3 {
                assert_eq!(delta, 0, "{context}: alpha differs at byte {index}");
            } else {
                max_rgb_delta = max_rgb_delta.max(delta);
                rgb_delta_total += u64::from(delta);
                changed_rgb_bytes += usize::from(delta > 0);
            }
        }
        let rgb_bytes = actual.data.len() / 4 * 3;
        let mean_rgb_delta = rgb_delta_total as f64 / rgb_bytes as f64;
        assert!(
            max_rgb_delta <= 2 && mean_rgb_delta <= 0.1 && changed_rgb_bytes * 20 <= rgb_bytes,
            "{context}: reference max RGB delta {max_rgb_delta}, mean {mean_rgb_delta:.4}, changed {changed_rgb_bytes}/{rgb_bytes}"
        );
    }

    #[test]
    fn cache_replaces_a_layers_entry_when_request_identity_changes() {
        let mut decoder = LibavFrameDecoder::software_for_tests();
        let mut request = request();
        let mut path = PathBuf::from("first.mp4");
        let mut file_signature = signature(1);

        decoder.replace_layer_cache_entry(cache_entry(&request, &path, file_signature.clone(), 1));
        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(
            decoder
                .cached_frame(
                    protocol::VideoRenderInputKey::LEGACY,
                    &request,
                    &path,
                    &file_signature
                )
                .unwrap()
                .data,
            vec![1]
        );

        request.position_ms = 33;
        assert!(decoder
            .cached_frame(
                protocol::VideoRenderInputKey::LEGACY,
                &request,
                &path,
                &file_signature
            )
            .is_none());
        decoder.replace_layer_cache_entry(cache_entry(&request, &path, file_signature.clone(), 2));
        assert_eq!(decoder.cache_len(), 1);

        request.width = 32;
        assert!(decoder
            .cached_frame(
                protocol::VideoRenderInputKey::LEGACY,
                &request,
                &path,
                &file_signature
            )
            .is_none());
        decoder.replace_layer_cache_entry(cache_entry(&request, &path, file_signature.clone(), 3));
        assert_eq!(decoder.cache_len(), 1);

        path = PathBuf::from("second.mp4");
        assert!(decoder
            .cached_frame(
                protocol::VideoRenderInputKey::LEGACY,
                &request,
                &path,
                &file_signature
            )
            .is_none());
        decoder.replace_layer_cache_entry(cache_entry(&request, &path, file_signature.clone(), 4));
        assert_eq!(decoder.cache_len(), 1);

        file_signature = signature(2);
        assert!(decoder
            .cached_frame(
                protocol::VideoRenderInputKey::LEGACY,
                &request,
                &path,
                &file_signature
            )
            .is_none());
        decoder.replace_layer_cache_entry(cache_entry(&request, &path, file_signature.clone(), 5));
        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(decoder.entries[0].position_ms, 33);
        assert_eq!(decoder.entries[0].width, 32);
        assert_eq!(decoder.entries[0].path, path);
        assert_eq!(decoder.entries[0].signature, file_signature);
        assert_eq!(decoder.entries[0].frame.data, vec![5]);
        assert_eq!(
            decoder
                .cached_frame(
                    protocol::VideoRenderInputKey::LEGACY,
                    &request,
                    &path,
                    &file_signature
                )
                .unwrap()
                .data,
            vec![5]
        );
    }

    #[test]
    fn retain_layers_prunes_the_bounded_layer_cache() {
        let mut decoder = LibavFrameDecoder::software_for_tests();
        let first = request();
        let mut second = request();
        second.layer_id = 2;
        decoder.replace_layer_cache_entry(cache_entry(&first, "first.mp4", signature(1), 1));
        decoder.replace_layer_cache_entry(cache_entry(&second, "second.mp4", signature(2), 2));
        assert_eq!(decoder.cache_len(), 2);

        decoder.retain_layers(&[second.layer_id]);

        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(decoder.entries[0].layer_id, second.layer_id);
        assert_eq!(decoder.entries[0].frame.data, vec![2]);
    }

    #[cfg(feature = "libav")]
    #[test]
    fn session_decision_reuses_only_cached_or_fully_drained_frames() {
        assert_eq!(
            session_decision(None, None, false, false, 0),
            LibavSessionDecision::Reopen
        );
        assert_eq!(
            session_decision(Some(100), Some(133), true, false, 100),
            LibavSessionDecision::ReuseFrame
        );
        assert_eq!(
            session_decision(Some(100), Some(133), true, false, 120),
            LibavSessionDecision::ReuseFrame
        );
        assert_eq!(
            session_decision(Some(100), Some(100), true, false, 133),
            LibavSessionDecision::Continue
        );
        assert_eq!(
            session_decision(Some(100), Some(100), true, true, 133),
            LibavSessionDecision::ReuseFrame
        );
        assert_eq!(
            session_decision(Some(100), Some(100), true, false, 1_101),
            LibavSessionDecision::Reopen
        );
        assert_eq!(
            session_decision(Some(100), Some(100), true, false, 99),
            LibavSessionDecision::Reopen
        );
        assert_eq!(
            session_decision(Some(100), Some(133), false, true, 120),
            LibavSessionDecision::Reopen
        );
    }

    #[cfg(feature = "libav")]
    #[test]
    fn persistent_decoder_remains_send_without_storing_a_scaler() {
        fn assert_send<T: Send>() {}
        assert_send::<LibavFrameDecoder>();
    }

    #[cfg(feature = "libav")]
    #[test]
    fn fixed_video_decode_reuses_one_bounded_scaler_at_frame_cadence() {
        use std::process::Command;

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let path = std::env::temp_dir().join(format!(
            "syndocal-libav-scaler-reuse-{}.mp4",
            std::process::id()
        ));
        let generated = Command::new(&ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=32x18:rate=30:duration=1",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
                "-y",
            ])
            .arg(&path)
            .output()
            .expect("FFmpeg fixed-video fixture generator must be available");
        assert!(
            generated.status.success(),
            "fixed-video fixture generation failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );

        let mut request = request();
        request.source.path = Some(path.to_string_lossy().into_owned());
        request.source.codec = Some("mpeg4".to_string());
        request.source.metadata = Some(protocol::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: Some(32),
            height: Some(18),
            frame_rate: Some(30.0),
            has_audio: false,
        });
        request.width = 64;
        request.height = 36;

        let mut decoder = LibavFrameDecoder::software_for_tests();
        for target_ms in [0, 33, 66, 99, 0, 33] {
            request.position_ms = target_ms;
            let frame = decoder
                .decode_frame(&request)
                .unwrap()
                .expect("fixed-video fixture must decode");
            assert_eq!((frame.width, frame.height), (64, 36));
            assert_eq!(frame.format, VideoPixelFormat::Rgba8);
            assert_eq!(
                decoder.decode_frame(&request).unwrap().unwrap(),
                frame,
                "same request should reuse the cached frame at {target_ms}ms"
            );
        }

        let scaler_entries = LIBAV_SCALER_CACHE.with(|cache| cache.borrow().len());
        let scaler_creations = LIBAV_SCALER_CREATION_COUNT.with(Cell::get);
        assert_eq!(
            scaler_entries, 1,
            "one source/target conversion should keep one reusable scaler"
        );
        assert_eq!(
            scaler_creations, 1,
            "repeated frame requests should create the scaler only once"
        );
        assert_eq!(decoder.sessions.len(), 1);
        assert_eq!(decoder.session_counters.opens, 2);
        assert_eq!(decoder.session_counters.resets, 1);
        assert!(decoder.session_counters.sequential_continues > 0);
        assert!(decoder.session_counters.frame_reuses >= 6);

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
        let _ = std::fs::remove_file(path);
    }

    #[cfg(feature = "libav")]
    #[test]
    fn failed_scaler_run_discards_context_before_recovery() {
        use ffmpeg::format::Pixel;
        use ffmpeg::util::frame::video::Video;

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));

        let request = request();
        let mut failed_lease =
            acquire_cached_libav_scaler(&request, Pixel::YUV420P, 16, 16).unwrap();
        let invalid_source = Video::new(Pixel::RGBA, 16, 16);
        let mut invalid_target = Video::empty();
        assert!(
            failed_lease
                .run(&invalid_source, &mut invalid_target)
                .is_err(),
            "a source-format change must fail the scaler run"
        );
        drop(failed_lease);
        assert_eq!(
            LIBAV_SCALER_CACHE.with(|cache| cache.borrow().len()),
            0,
            "a failed native run must not poison the reusable cache"
        );
        assert_eq!(LIBAV_SCALER_CREATION_COUNT.with(Cell::get), 1);

        let mut recovered_lease =
            acquire_cached_libav_scaler(&request, Pixel::YUV420P, 16, 16).unwrap();
        assert_eq!(
            LIBAV_SCALER_CREATION_COUNT.with(Cell::get),
            2,
            "recovery must create a fresh context rather than reuse the failed one"
        );
        let valid_source = Video::new(Pixel::YUV420P, 16, 16);
        let mut valid_target = Video::empty();
        assert!(recovered_lease
            .run(&valid_source, &mut valid_target)
            .is_ok());
        drop(recovered_lease);
        assert_eq!(
            LIBAV_SCALER_CACHE.with(|cache| cache.borrow().len()),
            1,
            "a successful replacement context remains reusable"
        );

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
    }

    #[cfg(feature = "libav")]
    #[test]
    fn scaler_cache_evicts_old_geometry_with_bounded_capacity() {
        use ffmpeg::format::Pixel;

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));

        let base_request = request();
        for width in 16..(16 + LIBAV_SCALER_CACHE_CAPACITY as u32 + 1) {
            let mut request = base_request.clone();
            request.width = width;
            let lease = acquire_cached_libav_scaler(&request, Pixel::YUV420P, 16, 16).unwrap();
            drop(lease);
        }
        assert_eq!(
            LIBAV_SCALER_CACHE.with(|cache| cache.borrow().len()),
            LIBAV_SCALER_CACHE_CAPACITY
        );
        assert_eq!(
            LIBAV_SCALER_CREATION_COUNT.with(Cell::get),
            LIBAV_SCALER_CACHE_CAPACITY + 1
        );

        let mut retained_request = base_request.clone();
        retained_request.width = 16 + LIBAV_SCALER_CACHE_CAPACITY as u32;
        let retained =
            acquire_cached_libav_scaler(&retained_request, Pixel::YUV420P, 16, 16).unwrap();
        drop(retained);
        assert_eq!(
            LIBAV_SCALER_CREATION_COUNT.with(Cell::get),
            LIBAV_SCALER_CACHE_CAPACITY + 1,
            "the newest geometry should remain cached"
        );

        let evicted = acquire_cached_libav_scaler(&base_request, Pixel::YUV420P, 16, 16).unwrap();
        drop(evicted);
        assert_eq!(
            LIBAV_SCALER_CREATION_COUNT.with(Cell::get),
            LIBAV_SCALER_CACHE_CAPACITY + 2,
            "the oldest geometry should be recreated after eviction"
        );

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
    }

    #[cfg(feature = "libav")]
    #[test]
    fn ten_bit_yuv444_decode_reuses_scaler_across_seek_and_loop() {
        use std::process::Command;

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let path = std::env::temp_dir().join(format!(
            "syndocal-libav-yuv444p10-{}-{}.mkv",
            std::process::id(),
            request().layer_id
        ));
        let generated = Command::new(&ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=32x18:rate=30:duration=1",
                "-vf",
                "format=yuv444p10le",
                "-c:v",
                "ffv1",
                "-level",
                "3",
                "-g",
                "1",
                "-pix_fmt",
                "yuv444p10le",
                "-y",
            ])
            .arg(&path)
            .output()
            .expect("FFmpeg 10-bit yuv444 fixture generator must be available");
        assert!(
            generated.status.success(),
            "10-bit yuv444 fixture generation failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        let reference = Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args([
                "-vf",
                "scale=64:36:flags=bilinear,format=rgba",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .output()
            .expect("FFmpeg 10-bit yuv444 reference decoder must be available");
        assert!(
            reference.status.success(),
            "10-bit yuv444 reference decode failed: {}",
            String::from_utf8_lossy(&reference.stderr)
        );
        const FRAME_BYTES: usize = 64 * 36 * 4;
        assert_eq!(reference.stdout.len(), 30 * FRAME_BYTES);

        let mut request = request();
        request.source.path = Some(path.to_string_lossy().into_owned());
        request.source.codec = Some("ffv1".to_string());
        request.source.metadata = Some(protocol::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: Some(32),
            height: Some(18),
            frame_rate: Some(30.0),
            has_audio: false,
        });
        request.width = 64;
        request.height = 36;

        let mut decoder = LibavFrameDecoder::software_for_tests();
        for target_ms in [0, 0, 33, 33, 66, 500, 0, 33] {
            request.position_ms = target_ms;
            let frame = decoder
                .decode_frame(&request)
                .unwrap()
                .expect("10-bit yuv444 fixture must decode");
            let frame_index = ((frame.pts_ms * 30 + 500) / 1_000) as usize;
            assert_rgba_reference_parity(
                &frame,
                &reference.stdout[frame_index * FRAME_BYTES..(frame_index + 1) * FRAME_BYTES],
                &format!("10-bit yuv444 at {target_ms}ms / pts {}ms", frame.pts_ms),
            );
            assert_eq!((frame.width, frame.height), (64, 36));
            assert_eq!(frame.format, VideoPixelFormat::Rgba8);
        }

        assert_eq!(
            LIBAV_SCALER_CACHE.with(|cache| cache.borrow().len()),
            1,
            "one 10-bit source/target conversion should remain reusable"
        );
        assert_eq!(LIBAV_SCALER_CREATION_COUNT.with(Cell::get), 1);
        assert_eq!(decoder.sessions.len(), 1);
        assert!(decoder.session_counters.sequential_continues > 0);
        assert!(decoder.session_counters.frame_reuses >= 2);

        LIBAV_SCALER_CACHE.with(|cache| cache.borrow_mut().clear());
        LIBAV_SCALER_CREATION_COUNT.with(|count| count.set(0));
        let _ = std::fs::remove_file(path);
    }

    #[cfg(not(feature = "libav"))]
    #[test]
    fn disabled_backend_defers_to_compatibility_decoder() {
        let mut decoder = LibavFrameDecoder::software_for_tests();
        assert!(!LibavFrameDecoder::is_built());
        assert_eq!(decoder.decode_frame(&request()), Ok(None));
    }

    #[cfg(feature = "libav")]
    #[test]
    fn decodes_h264_h265_and_prores_in_process() {
        use std::process::Command;

        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let encoder_output = Command::new(&ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .output()
            .expect("FFmpeg encoder inventory must be available");
        assert!(encoder_output.status.success());
        let encoder_inventory = String::from_utf8_lossy(&encoder_output.stdout);
        let available_encoders = encoder_inventory
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect::<std::collections::HashSet<_>>();
        assert!(LibavFrameDecoder::is_built());
        for (codec, encoder_candidates, extension, pixel_format) in [
            (
                "h264",
                &["libx264", "libopenh264"] as &[&str],
                "mp4",
                "yuv420p",
            ),
            (
                "hevc",
                &["libx265", "libkvazaar"] as &[&str],
                "mp4",
                "yuv420p",
            ),
            ("prores", &["prores_ks"] as &[&str], "mov", "yuv422p10le"),
        ] {
            let encoder = encoder_candidates
                .iter()
                .find(|candidate| available_encoders.contains(**candidate))
                .unwrap_or_else(|| panic!("no QA encoder is available for {codec}"));
            let path = std::env::temp_dir().join(format!(
                "syndocal-libav-{codec}-{}-{}.{}",
                std::process::id(),
                request().layer_id,
                extension
            ));
            let mut command = Command::new(&ffmpeg);
            command.args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=16x16:d=1:r=2",
                "-c:v",
                encoder,
                "-pix_fmt",
                pixel_format,
            ]);
            if *encoder == "libx265" {
                command.args(["-x265-params", "pools=1:frame-threads=1"]);
            }
            command.arg("-y").arg(&path);
            let output = command
                .output()
                .expect("FFmpeg QA fixture generator must be available");
            assert!(
                output.status.success(),
                "{codec} fixture generation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut request = request();
            request.source.path = Some(path.to_string_lossy().into_owned());
            request.source.codec = Some(codec.to_string());
            request.position_ms = 500;
            request.source.metadata = Some(protocol::VideoMediaMetadata {
                duration_ms: Some(1_000),
                width: Some(16),
                height: Some(16),
                frame_rate: Some(2.0),
                has_audio: false,
            });
            let mut decoder = LibavFrameDecoder::software_for_tests();

            let frame = decoder.decode_frame(&request).unwrap().unwrap();

            assert_eq!(frame.format, VideoPixelFormat::Rgba8, "{codec}");
            assert_eq!((frame.width, frame.height), (16, 16), "{codec}");
            assert_eq!(frame.data.len(), 16 * 16 * 4, "{codec}");
            assert!(frame.pts_ms >= 500, "{codec}: pts {}", frame.pts_ms);
            assert!(frame.data[0] > 200, "{codec}");
            assert!(frame.data[1] < 40, "{codec}");
            assert!(frame.data[2] < 40, "{codec}");
            assert_eq!(decoder.cache_len(), 1, "{codec}");
            assert_eq!(decoder.decode_frame(&request).unwrap().unwrap(), frame);
            assert_eq!(decoder.session_counters.frame_reuses, 1, "{codec}");

            let missing_ffmpeg = std::env::temp_dir().join(format!(
                "syndocal-missing-ffmpeg-{codec}-{}",
                std::process::id()
            ));
            let mut preferred = PreferredVideoFrameDecoder::with_software_libav_for_tests(
                FfmpegCliFrameDecoder::new(missing_ffmpeg),
            );
            assert_eq!(preferred.decode_frame(&request).unwrap().unwrap(), frame);
            assert_eq!(
                preferred.diagnostics(),
                VideoDecoderDiagnostics {
                    total_requests: 1,
                    libav_requests: 1,
                    libav_successes: 1,
                    libav_cache_len: 1,
                    libav_session_count: 1,
                    libav_session_open_count: 1,
                    ..VideoDecoderDiagnostics::default()
                },
                "{codec}"
            );
            let _ = std::fs::remove_file(path);
        }
    }

    #[cfg(feature = "libav")]
    #[test]
    fn sequential_b_frame_session_matches_reference_and_drains_eof() {
        use std::process::Command;

        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let path = std::env::temp_dir().join(format!(
            "syndocal-libav-sequential-bframes-{}.mp4",
            std::process::id()
        ));
        let output = Command::new(&ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=32x18:rate=30:duration=3",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
                "-bf",
                "2",
                "-g",
                "30",
                "-q:v",
                "2",
                "-y",
            ])
            .arg(&path)
            .output()
            .expect("FFmpeg B-frame fixture generator must be available");
        assert!(
            output.status.success(),
            "B-frame fixture generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reference = Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args([
                "-vf",
                "format=rgba",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .output()
            .expect("FFmpeg sequential reference decoder must be available");
        assert!(
            reference.status.success(),
            "B-frame reference decode failed: {}",
            String::from_utf8_lossy(&reference.stderr)
        );
        const FRAME_BYTES: usize = 32 * 18 * 4;
        assert_eq!(reference.stdout.len(), 90 * FRAME_BYTES);

        let mut sequential = LibavFrameDecoder::software_for_tests();
        let mut request = request();
        request.source.path = Some(path.to_string_lossy().into_owned());
        request.source.codec = Some("mpeg4".to_string());
        request.source.metadata = Some(protocol::VideoMediaMetadata {
            duration_ms: Some(3_000),
            width: Some(32),
            height: Some(18),
            frame_rate: Some(30.0),
            has_audio: false,
        });
        request.width = 32;
        request.height = 18;

        for target_ms in (0..=600).step_by(16) {
            request.position_ms = target_ms;
            let persistent_frame = sequential.decode_frame(&request).unwrap().unwrap();
            let frame_index = ((persistent_frame.pts_ms * 30 + 500) / 1_000) as usize;
            assert_rgba_reference_parity(
                &persistent_frame,
                &reference.stdout[frame_index * FRAME_BYTES..(frame_index + 1) * FRAME_BYTES],
                &format!(
                    "persistent decode at {target_ms}ms / pts {}ms / continues {} / reuses {}",
                    persistent_frame.pts_ms,
                    sequential.session_counters.sequential_continues,
                    sequential.session_counters.frame_reuses
                ),
            );
        }

        assert_eq!(sequential.cache_len(), 1);
        assert_eq!(sequential.sessions.len(), 1);
        assert_eq!(sequential.session_counters.opens, 1);
        assert_eq!(sequential.session_counters.resets, 0);
        assert!(sequential.session_counters.sequential_continues > 0);
        assert!(sequential.session_counters.frame_reuses > 0);
        assert_eq!(sequential.session_counters.errors, 0);
        assert!(!sequential.sessions[0].eof_sent);
        assert!(!sequential.sessions[0].decoder_drained);

        request.position_ms = 1_500;
        let jumped = sequential.decode_frame(&request).unwrap().unwrap();
        let mut fresh = LibavFrameDecoder::software_for_tests();
        assert_seek_parity(
            &jumped,
            &fresh.decode_frame(&request).unwrap().unwrap(),
            "large forward jump",
        );
        assert_eq!(sequential.session_counters.opens, 2);
        assert_eq!(sequential.session_counters.resets, 1);

        request.position_ms = 100;
        let reversed = sequential.decode_frame(&request).unwrap().unwrap();
        let mut fresh = LibavFrameDecoder::software_for_tests();
        assert_seek_parity(
            &reversed,
            &fresh.decode_frame(&request).unwrap().unwrap(),
            "backward reset",
        );
        assert_eq!(sequential.session_counters.opens, 3);
        assert_eq!(sequential.session_counters.resets, 2);

        let mut through_eof = LibavFrameDecoder::software_for_tests();
        let mut final_frame = None;
        for target_ms in (0..=3_500).step_by(33) {
            request.position_ms = target_ms;
            let persistent_frame = through_eof.decode_frame(&request).unwrap().unwrap();
            let frame_index = ((persistent_frame.pts_ms * 30 + 500) / 1_000) as usize;
            assert_rgba_reference_parity(
                &persistent_frame,
                &reference.stdout[frame_index * FRAME_BYTES..(frame_index + 1) * FRAME_BYTES],
                &format!("persistent EOF drain at {target_ms}ms"),
            );
            final_frame = Some(persistent_frame);
        }
        assert_eq!(through_eof.session_counters.opens, 1);
        assert_eq!(through_eof.session_counters.resets, 0);
        assert!(through_eof.sessions[0].eof_sent);
        assert!(through_eof.sessions[0].decoder_drained);
        let reuse_count = through_eof.session_counters.frame_reuses;
        request.position_ms = 4_000;
        assert_eq!(
            through_eof.decode_frame(&request).unwrap().unwrap(),
            final_frame.unwrap()
        );
        assert_eq!(through_eof.session_counters.opens, 1);
        assert_eq!(through_eof.session_counters.frame_reuses, reuse_count + 1);

        request.source.path = Some(
            path.with_extension("missing.mp4")
                .to_string_lossy()
                .into_owned(),
        );
        assert!(sequential.decode_frame(&request).is_err());
        assert_eq!(sequential.cache_len(), 0);
        assert!(sequential.sessions.is_empty());
        assert_eq!(sequential.session_counters.errors, 1);

        request.source.path = None;
        assert!(matches!(
            sequential.decode_frame(&request),
            Err(VideoDecodeError::MissingSourcePath { .. })
        ));
        assert_eq!(sequential.session_counters.errors, 2);

        let _ = std::fs::remove_file(path);
    }

    #[cfg(feature = "libav")]
    #[test]
    fn normalizes_positive_and_negative_stream_start_times_for_decode_and_seek() {
        use std::process::Command;

        const FRAME_BYTES: usize = 32 * 18 * 4;
        const FRAME_COUNT: usize = 20;
        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());

        for (label, timestamp_offset, expected_origin_sign) in
            [("positive", "1", 1_i8), ("negative", "-1", -1_i8)]
        {
            let path = std::env::temp_dir().join(format!(
                "syndocal-libav-{label}-start-time-{}.ts",
                std::process::id()
            ));
            let output = Command::new(&ffmpeg)
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=32x18:rate=10:duration=2",
                    "-c:v",
                    "mpeg2video",
                    "-bf",
                    "2",
                    "-g",
                    "10",
                    "-output_ts_offset",
                    timestamp_offset,
                    "-avoid_negative_ts",
                    "disabled",
                    "-muxdelay",
                    "0",
                    "-muxpreload",
                    "0",
                    "-y",
                ])
                .arg(&path)
                .output()
                .expect("FFmpeg timestamp-offset fixture generator must be available");
            assert!(
                output.status.success(),
                "{label} timestamp fixture generation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let reference = Command::new(&ffmpeg)
                .args(["-v", "error", "-i"])
                .arg(&path)
                .args([
                    "-vf",
                    "format=rgba",
                    "-pix_fmt",
                    "rgba",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ])
                .output()
                .expect("FFmpeg timestamp-offset reference decoder must be available");
            assert!(
                reference.status.success(),
                "{label} timestamp reference decode failed: {}",
                String::from_utf8_lossy(&reference.stderr)
            );
            assert_eq!(
                reference.stdout.len(),
                FRAME_COUNT * FRAME_BYTES,
                "{label} reference frame count"
            );

            let mut decoder = LibavFrameDecoder::software_for_tests();
            let mut request = request();
            request.source.path = Some(path.to_string_lossy().into_owned());
            request.source.codec = Some("mpeg2video".to_string());
            request.source.metadata = Some(protocol::VideoMediaMetadata {
                duration_ms: Some(2_000),
                width: Some(32),
                height: Some(18),
                frame_rate: Some(10.0),
                has_audio: false,
            });
            request.width = 32;
            request.height = 18;

            for target_ms in (0..=500).step_by(50) {
                request.position_ms = target_ms;
                let frame = decoder.decode_frame(&request).unwrap().unwrap();
                let expected_frame_index = target_ms.div_ceil(100) as usize;
                assert_eq!(
                    frame.pts_ms,
                    expected_frame_index as u64 * 100,
                    "{label} sequential target {target_ms}ms"
                );
                assert_rgba_reference_parity(
                    &frame,
                    &reference.stdout[expected_frame_index * FRAME_BYTES
                        ..(expected_frame_index + 1) * FRAME_BYTES],
                    &format!("{label} sequential target {target_ms}ms"),
                );
            }
            assert_eq!(decoder.sessions.len(), 1, "{label}");
            assert_eq!(
                decoder.sessions[0].timestamp_origin.signum() as i8,
                expected_origin_sign,
                "{label} stream start-time sign"
            );

            for target_ms in [1_500_u64, 100] {
                request.position_ms = target_ms;
                let frame = decoder.decode_frame(&request).unwrap().unwrap();
                let expected_frame_index = target_ms.div_ceil(100) as usize;
                assert_eq!(
                    frame.pts_ms,
                    expected_frame_index as u64 * 100,
                    "{label} seek target {target_ms}ms"
                );
                assert_rgba_reference_parity(
                    &frame,
                    &reference.stdout[expected_frame_index * FRAME_BYTES
                        ..(expected_frame_index + 1) * FRAME_BYTES],
                    &format!("{label} seek target {target_ms}ms"),
                );
            }
            assert_eq!(decoder.session_counters.resets, 2, "{label}");

            if label == "positive" {
                let mut bounded = LibavFrameDecoder::software_for_tests();
                request.position_ms = 0;
                for layer_id in 1..=9 {
                    request.layer_id = layer_id;
                    bounded.decode_frame(&request).unwrap().unwrap();
                }
                assert_eq!(bounded.entries.len(), LIBAV_WORKING_SET_CAPACITY);
                assert_eq!(bounded.sessions.len(), LIBAV_WORKING_SET_CAPACITY);
                assert_eq!(
                    bounded.working_set_lru,
                    (2..=9)
                        .map(|layer_id| LibavRenderInputOwner::new(
                            protocol::VideoRenderInputKey::LEGACY,
                            layer_id,
                        ))
                        .collect::<Vec<_>>()
                );
                assert_eq!(bounded.session_counters.opens, 9);
                assert_eq!(bounded.session_counters.evictions, 1);
                assert!(bounded.entries.iter().all(|entry| entry.layer_id != 1));
                assert!(bounded.sessions.iter().all(|session| session.layer_id != 1));

                request.source.kind = VideoSourceKind::Camera;
                assert_eq!(bounded.decode_frame(&request).unwrap(), None);
                assert_eq!(bounded.entries.len(), LIBAV_WORKING_SET_CAPACITY - 1);
                assert_eq!(bounded.sessions.len(), LIBAV_WORKING_SET_CAPACITY - 1);
                assert!(!bounded
                    .working_set_lru
                    .contains(&LibavRenderInputOwner::new(
                        protocol::VideoRenderInputKey::LEGACY,
                        request.layer_id,
                    ),));
                request.source.kind = VideoSourceKind::File;
            }

            let _ = std::fs::remove_file(path);
        }
    }
}
#[test]
fn legacy_render_input_owners_remain_layer_scoped() {
    let first = LibavRenderInputOwner::new(protocol::VideoRenderInputKey::LEGACY, 1);
    let second = LibavRenderInputOwner::new(protocol::VideoRenderInputKey::LEGACY, 2);
    assert_ne!(first, second);
    assert!(first.matches(protocol::VideoRenderInputKey::LEGACY, 1));
    assert!(!first.matches(protocol::VideoRenderInputKey::LEGACY, 2));

    let explicit = protocol::VideoRenderInputKey {
        project_render_epoch: 4,
        input_id: protocol::VideoRenderInputId(8),
    };
    assert!(LibavRenderInputOwner::new(explicit, 1).matches(explicit, 999));
}
