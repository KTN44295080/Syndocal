//! Identity-keyed media-derived cache and bounded work admission.
//!
//! Project-authored media identity stays outside this module.  Every derived
//! record is keyed by the immutable content identity plus the algorithm and
//! settings that produced it, so a pathname change cannot reuse a stale
//! thumbnail, waveform, proxy, or analysis result.  The cache is deliberately
//! machine-local policy: callers supply the current free-space observation and
//! workers own filesystem/process I/O at the boundary.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

const MAX_SETTINGS_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaContentIdentity {
    pub algorithm: String,
    pub digest: String,
    pub byte_size: u64,
}

impl MediaContentIdentity {
    pub fn new(algorithm: impl Into<String>, digest: impl Into<String>, byte_size: u64) -> Self {
        Self {
            algorithm: algorithm.into(),
            digest: digest.into(),
            byte_size,
        }
    }

    fn validate(&self) -> Result<(), CacheError> {
        if self.algorithm.trim().is_empty() || self.digest.trim().is_empty() {
            return Err(CacheError::InvalidIdentity);
        }
        Ok(())
    }
}

/// Derived products share one cache identity contract even when their payload
/// is produced by different decoders or analyzers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum MediaDerivedKind {
    Thumbnail,
    ContactSheet,
    CodecDetails,
    Waveform,
    Proxy,
    Analysis,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaDerivedKey {
    pub content: MediaContentIdentity,
    pub kind: MediaDerivedKind,
    pub algorithm_version: u32,
    /// Canonical serialized settings, for example dimensions, seek position,
    /// proxy profile, or analysis resolution.  It is not a pathname.
    pub settings: String,
}

impl MediaDerivedKey {
    pub fn new(
        content: MediaContentIdentity,
        kind: MediaDerivedKind,
        algorithm_version: u32,
        settings: impl Into<String>,
    ) -> Self {
        Self {
            content,
            kind,
            algorithm_version,
            settings: settings.into(),
        }
    }

    fn validate(&self) -> Result<(), CacheError> {
        self.content.validate()?;
        if self.algorithm_version == 0 || self.settings.len() > MAX_SETTINGS_BYTES {
            return Err(CacheError::InvalidKey);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaDerivedCachePolicy {
    pub max_bytes: usize,
    pub max_entries: usize,
    pub min_free_space_bytes: u64,
}

impl MediaDerivedCachePolicy {
    pub fn validate(self) -> Result<Self, CacheError> {
        if self.max_bytes == 0 || self.max_entries == 0 {
            return Err(CacheError::InvalidPolicy);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaDerivedReadiness {
    Ready,
    Queued,
    Running,
    Degraded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaDerivedCacheEntry {
    pub key: MediaDerivedKey,
    pub byte_len: usize,
    pub last_used: u64,
    pub pinned: bool,
    pub in_use: u32,
    pub readiness: MediaDerivedReadiness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheInsertOutcome {
    Stored { evicted: Vec<MediaDerivedKey> },
    Deduplicated,
    Degraded { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheError {
    InvalidIdentity,
    InvalidKey,
    InvalidPolicy,
    EntryNotFound,
    EntryInUse,
    OperationNotFound,
    InvalidOperationState,
    QueueFull,
}

/// A bounded LRU cache for machine-local derived records.
pub struct MediaDerivedCache {
    policy: MediaDerivedCachePolicy,
    entries: HashMap<MediaDerivedKey, MediaDerivedCacheEntry>,
    quarantined: HashSet<MediaDerivedKey>,
    used_bytes: usize,
    clock: u64,
}

impl MediaDerivedCache {
    pub fn new(policy: MediaDerivedCachePolicy) -> Result<Self, CacheError> {
        Ok(Self {
            policy: policy.validate()?,
            entries: HashMap::new(),
            quarantined: HashSet::new(),
            used_bytes: 0,
            clock: 0,
        })
    }

    pub fn policy(&self) -> MediaDerivedCachePolicy {
        self.policy
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn quarantined_len(&self) -> usize {
        self.quarantined.len()
    }

    pub fn get(&mut self, key: &MediaDerivedKey) -> Option<&MediaDerivedCacheEntry> {
        if self.entries.contains_key(key) {
            self.touch(key);
        }
        self.entries.get(key)
    }

    pub fn entry(&self, key: &MediaDerivedKey) -> Option<&MediaDerivedCacheEntry> {
        self.entries.get(key)
    }

    pub fn insert_ready(
        &mut self,
        key: MediaDerivedKey,
        byte_len: usize,
        available_free_space_bytes: u64,
    ) -> Result<CacheInsertOutcome, CacheError> {
        key.validate()?;
        if byte_len == 0 {
            return Ok(CacheInsertOutcome::Degraded {
                reason: "empty derived artifact".to_string(),
            });
        }
        if self.quarantined.contains(&key) {
            return Ok(CacheInsertOutcome::Degraded {
                reason: "derived artifact is quarantined until rebuilt".to_string(),
            });
        }
        if available_free_space_bytes < self.policy.min_free_space_bytes {
            return Ok(CacheInsertOutcome::Degraded {
                reason: "minimum free-space reserve is not available".to_string(),
            });
        }
        if byte_len > self.policy.max_bytes {
            return Ok(CacheInsertOutcome::Degraded {
                reason: "derived artifact exceeds the configured cache limit".to_string(),
            });
        }
        self.clock = self.clock.saturating_add(1);
        if let Some(existing) = self.entries.get_mut(&key) {
            let prospective_used_bytes = self
                .used_bytes
                .saturating_sub(existing.byte_len)
                .saturating_add(byte_len);
            if prospective_used_bytes > self.policy.max_bytes {
                return Ok(CacheInsertOutcome::Degraded {
                    reason: "updated derived artifact exceeds the configured cache limit"
                        .to_string(),
                });
            }
            self.used_bytes = prospective_used_bytes;
            existing.byte_len = byte_len;
            existing.last_used = self.clock;
            existing.readiness = MediaDerivedReadiness::Ready;
            return Ok(CacheInsertOutcome::Deduplicated);
        }

        let mut evicted = Vec::new();
        while self.used_bytes.saturating_add(byte_len) > self.policy.max_bytes
            || self.entries.len() >= self.policy.max_entries
        {
            let Some(candidate) = self
                .entries
                .values()
                .filter(|entry| !entry.pinned && entry.in_use == 0)
                .min_by_key(|entry| entry.last_used)
                .map(|entry| entry.key.clone())
            else {
                return Ok(CacheInsertOutcome::Degraded {
                    reason: "all cache entries are pinned or in use".to_string(),
                });
            };
            let removed = self.entries.remove(&candidate).expect("candidate exists");
            self.used_bytes -= removed.byte_len;
            evicted.push(candidate);
        }
        self.used_bytes += byte_len;
        self.entries.insert(
            key.clone(),
            MediaDerivedCacheEntry {
                key,
                byte_len,
                last_used: self.clock,
                pinned: false,
                in_use: 0,
                readiness: MediaDerivedReadiness::Ready,
            },
        );
        Ok(CacheInsertOutcome::Stored { evicted })
    }

    pub fn mark_in_use(&mut self, key: &MediaDerivedKey) -> Result<(), CacheError> {
        let entry = self.entries.get_mut(key).ok_or(CacheError::EntryNotFound)?;
        entry.in_use = entry.in_use.saturating_add(1);
        Ok(())
    }

    pub fn release(&mut self, key: &MediaDerivedKey) -> Result<(), CacheError> {
        let entry = self.entries.get_mut(key).ok_or(CacheError::EntryNotFound)?;
        if entry.in_use == 0 {
            return Err(CacheError::InvalidOperationState);
        }
        entry.in_use -= 1;
        Ok(())
    }

    pub fn set_pinned(&mut self, key: &MediaDerivedKey, pinned: bool) -> Result<(), CacheError> {
        self.entries
            .get_mut(key)
            .ok_or(CacheError::EntryNotFound)
            .map(|entry| entry.pinned = pinned)
    }

    pub fn quarantine(&mut self, key: &MediaDerivedKey) -> Result<(), CacheError> {
        let entry = self.entries.remove(key).ok_or(CacheError::EntryNotFound)?;
        if entry.in_use > 0 {
            self.entries.insert(key.clone(), entry);
            return Err(CacheError::EntryInUse);
        }
        self.used_bytes -= entry.byte_len;
        self.quarantined.insert(key.clone());
        Ok(())
    }

    pub fn allow_rebuild(&mut self, key: &MediaDerivedKey) {
        self.quarantined.remove(key);
    }

    pub fn clear(&mut self) -> usize {
        let removable: Vec<_> = self
            .entries
            .values()
            .filter(|entry| !entry.pinned && entry.in_use == 0)
            .map(|entry| entry.key.clone())
            .collect();
        let count = removable.len();
        for key in removable {
            if let Some(entry) = self.entries.remove(&key) {
                self.used_bytes -= entry.byte_len;
            }
        }
        count
    }

    fn touch(&mut self, key: &MediaDerivedKey) {
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used = self.clock;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum MediaDerivedPriority {
    Background = 1,
    Next = 2,
    Playback = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaDerivedOperationState {
    Queued,
    Running,
    CancelRequested,
    Ready,
    Degraded(String),
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaDerivedOperationSnapshot {
    pub operation_id: u64,
    pub key: MediaDerivedKey,
    pub priority: MediaDerivedPriority,
    pub progress_millis: u16,
    pub state: MediaDerivedOperationState,
}

#[derive(Clone, Debug)]
struct QueuedOperation {
    priority: MediaDerivedPriority,
    sequence: u64,
    operation_id: u64,
}

impl PartialEq for QueuedOperation {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}

impl Eq for QueuedOperation {}

impl Ord for QueuedOperation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for QueuedOperation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct OperationRecord {
    key: MediaDerivedKey,
    priority: MediaDerivedPriority,
    progress_millis: u16,
    state: MediaDerivedOperationState,
}

/// Bounded admission for hash/probe/thumbnail/proxy/analysis work.  It does
/// not own filesystem or decoder calls; the caller owns each admitted worker
/// and reports progress/terminal truth back through this state machine.
pub struct MediaDerivedWorkQueue {
    max_workers: usize,
    queue_capacity: usize,
    next_operation_id: u64,
    next_sequence: u64,
    running: usize,
    queue: BinaryHeap<QueuedOperation>,
    operations: HashMap<u64, OperationRecord>,
}

impl MediaDerivedWorkQueue {
    pub fn new(max_workers: usize, queue_capacity: usize) -> Result<Self, CacheError> {
        if max_workers == 0 || queue_capacity == 0 {
            return Err(CacheError::InvalidPolicy);
        }
        Ok(Self {
            max_workers,
            queue_capacity,
            next_operation_id: 0,
            next_sequence: 0,
            running: 0,
            queue: BinaryHeap::new(),
            operations: HashMap::new(),
        })
    }

    pub fn queued_len(&self) -> usize {
        self.queue
            .iter()
            .filter(|item| {
                self.operations
                    .get(&item.operation_id)
                    .is_some_and(|record| {
                        matches!(record.state, MediaDerivedOperationState::Queued)
                    })
            })
            .count()
    }

    pub fn running_len(&self) -> usize {
        self.running
    }

    pub fn submit(
        &mut self,
        key: MediaDerivedKey,
        priority: MediaDerivedPriority,
    ) -> Result<u64, CacheError> {
        key.validate()?;
        if let Some(existing) = self.operations.iter().find_map(|(id, record)| {
            (record.key == key
                && matches!(
                    record.state,
                    MediaDerivedOperationState::Queued
                        | MediaDerivedOperationState::Running
                        | MediaDerivedOperationState::CancelRequested
                ))
            .then_some(*id)
        }) {
            return Ok(existing);
        }
        if self.queued_len() >= self.queue_capacity {
            return Err(CacheError::QueueFull);
        }
        self.next_operation_id = self.next_operation_id.saturating_add(1);
        self.next_sequence = self.next_sequence.saturating_add(1);
        let operation_id = self.next_operation_id;
        self.operations.insert(
            operation_id,
            OperationRecord {
                key,
                priority,
                progress_millis: 0,
                state: MediaDerivedOperationState::Queued,
            },
        );
        self.queue.push(QueuedOperation {
            priority,
            sequence: self.next_sequence,
            operation_id,
        });
        Ok(operation_id)
    }

    pub fn start_next(&mut self) -> Option<MediaDerivedOperationSnapshot> {
        while self.running < self.max_workers {
            let item = self.queue.pop()?;
            let record = self.operations.get_mut(&item.operation_id)?;
            if !matches!(record.state, MediaDerivedOperationState::Queued) {
                continue;
            }
            record.state = MediaDerivedOperationState::Running;
            self.running += 1;
            return Some(self.snapshot(item.operation_id));
        }
        None
    }

    pub fn set_progress(
        &mut self,
        operation_id: u64,
        progress_millis: u16,
    ) -> Result<(), CacheError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(CacheError::OperationNotFound)?;
        if !matches!(
            record.state,
            MediaDerivedOperationState::Running | MediaDerivedOperationState::CancelRequested
        ) {
            return Err(CacheError::InvalidOperationState);
        }
        record.progress_millis = progress_millis.min(1000);
        Ok(())
    }

    pub fn request_cancel(&mut self, operation_id: u64) -> Result<bool, CacheError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(CacheError::OperationNotFound)?;
        match record.state {
            MediaDerivedOperationState::Queued => {
                record.state = MediaDerivedOperationState::Cancelled;
                Ok(true)
            }
            MediaDerivedOperationState::Running => {
                record.state = MediaDerivedOperationState::CancelRequested;
                Ok(true)
            }
            MediaDerivedOperationState::CancelRequested => Ok(false),
            _ => Ok(false),
        }
    }

    pub fn is_cancel_requested(&self, operation_id: u64) -> Result<bool, CacheError> {
        let record = self
            .operations
            .get(&operation_id)
            .ok_or(CacheError::OperationNotFound)?;
        Ok(matches!(
            record.state,
            MediaDerivedOperationState::CancelRequested | MediaDerivedOperationState::Cancelled
        ))
    }

    pub fn finish(
        &mut self,
        operation_id: u64,
        state: MediaDerivedOperationState,
    ) -> Result<MediaDerivedOperationSnapshot, CacheError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(CacheError::OperationNotFound)?;
        let was_running = matches!(
            record.state,
            MediaDerivedOperationState::Running | MediaDerivedOperationState::CancelRequested
        );
        if !was_running {
            return Err(CacheError::InvalidOperationState);
        }
        if matches!(record.state, MediaDerivedOperationState::CancelRequested) {
            record.state = MediaDerivedOperationState::Cancelled;
        } else {
            record.state = state;
        }
        self.running -= 1;
        Ok(self.snapshot(operation_id))
    }

    pub fn restart(&mut self, operation_id: u64) -> Result<u64, CacheError> {
        let record = self
            .operations
            .get(&operation_id)
            .ok_or(CacheError::OperationNotFound)?
            .clone();
        if matches!(
            record.state,
            MediaDerivedOperationState::Queued
                | MediaDerivedOperationState::Running
                | MediaDerivedOperationState::CancelRequested
        ) {
            return Err(CacheError::InvalidOperationState);
        }
        self.submit(record.key, record.priority)
    }

    pub fn snapshot(&self, operation_id: u64) -> MediaDerivedOperationSnapshot {
        let record = self
            .operations
            .get(&operation_id)
            .expect("operation id must exist");
        MediaDerivedOperationSnapshot {
            operation_id,
            key: record.key.clone(),
            priority: record.priority,
            progress_millis: record.progress_millis,
            state: record.state.clone(),
        }
    }

    fn snapshot_result(
        &self,
        operation_id: u64,
    ) -> Result<MediaDerivedOperationSnapshot, CacheError> {
        self.operations
            .contains_key(&operation_id)
            .then(|| self.snapshot(operation_id))
            .ok_or(CacheError::OperationNotFound)
    }
}

impl MediaDerivedWorkQueue {
    pub fn status(&self, operation_id: u64) -> Result<MediaDerivedOperationSnapshot, CacheError> {
        self.snapshot_result(operation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(digest: &str) -> MediaContentIdentity {
        MediaContentIdentity::new("sha256", digest, 100)
    }

    fn key(digest: &str, kind: MediaDerivedKind, settings: &str) -> MediaDerivedKey {
        MediaDerivedKey::new(content(digest), kind, 1, settings)
    }

    fn cache() -> MediaDerivedCache {
        MediaDerivedCache::new(MediaDerivedCachePolicy {
            max_bytes: 10,
            max_entries: 3,
            min_free_space_bytes: 4,
        })
        .unwrap()
    }

    #[test]
    fn key_identity_is_content_kind_version_and_settings_not_path() {
        let first = key("a", MediaDerivedKind::Thumbnail, "96x54");
        let same = key("a", MediaDerivedKind::Thumbnail, "96x54");
        let different_content = key("b", MediaDerivedKind::Thumbnail, "96x54");
        let different_settings = key("a", MediaDerivedKind::Thumbnail, "192x108");
        assert_eq!(first, same);
        assert_ne!(first, different_content);
        assert_ne!(first, different_settings);
    }

    #[test]
    fn cache_deduplicates_identical_content_and_settings() {
        let mut cache = cache();
        let item = key("a", MediaDerivedKind::Waveform, "1000");
        assert_eq!(
            cache.insert_ready(item.clone(), 4, 100).unwrap(),
            CacheInsertOutcome::Stored { evicted: vec![] }
        );
        assert_eq!(
            cache.insert_ready(item.clone(), 4, 100).unwrap(),
            CacheInsertOutcome::Deduplicated
        );
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.used_bytes(), 4);
        assert_eq!(
            cache.get(&item).unwrap().readiness,
            MediaDerivedReadiness::Ready
        );
    }

    #[test]
    fn cache_eviction_is_lru_but_never_removes_pinned_or_in_use_entries() {
        let mut cache = cache();
        let old = key("old", MediaDerivedKind::Proxy, "preview");
        let pinned = key("pinned", MediaDerivedKind::Thumbnail, "96x54");
        let in_use = key("in-use", MediaDerivedKind::Analysis, "fft");
        cache.insert_ready(old.clone(), 4, 100).unwrap();
        cache.insert_ready(pinned.clone(), 3, 100).unwrap();
        cache.insert_ready(in_use.clone(), 2, 100).unwrap();
        cache.set_pinned(&pinned, true).unwrap();
        cache.mark_in_use(&in_use).unwrap();
        let newest = key("new", MediaDerivedKind::ContactSheet, "4x2");
        let result = cache.insert_ready(newest.clone(), 4, 100).unwrap();
        assert_eq!(result, CacheInsertOutcome::Stored { evicted: vec![old] });
        assert!(cache.entry(&pinned).is_some());
        assert!(cache.entry(&in_use).is_some());
        assert!(cache.entry(&newest).is_some());
    }

    #[test]
    fn cache_reports_degraded_when_free_space_or_capacity_is_unsafe() {
        let mut cache = cache();
        let item = key("a", MediaDerivedKind::CodecDetails, "v1");
        assert!(matches!(
            cache.insert_ready(item.clone(), 2, 3).unwrap(),
            CacheInsertOutcome::Degraded { .. }
        ));
        assert!(matches!(
            cache.insert_ready(item, 11, 100).unwrap(),
            CacheInsertOutcome::Degraded { .. }
        ));
    }

    #[test]
    fn cache_rejects_a_same_key_resize_that_would_exceed_total_capacity() {
        let mut cache = cache();
        let first = key("a", MediaDerivedKind::Thumbnail, "96x54");
        let second = key("b", MediaDerivedKind::Waveform, "1000");
        cache.insert_ready(first.clone(), 6, 100).unwrap();
        cache.insert_ready(second, 4, 100).unwrap();
        assert!(matches!(
            cache.insert_ready(first.clone(), 7, 100).unwrap(),
            CacheInsertOutcome::Degraded { .. }
        ));
        assert_eq!(cache.entry(&first).unwrap().byte_len, 6);
        assert_eq!(cache.used_bytes(), 10);
    }

    #[test]
    fn corrupt_entries_are_quarantined_until_explicit_rebuild() {
        let mut cache = cache();
        let item = key("a", MediaDerivedKind::Analysis, "v1");
        cache.insert_ready(item.clone(), 2, 100).unwrap();
        cache.quarantine(&item).unwrap();
        assert_eq!(cache.quarantined_len(), 1);
        assert!(matches!(
            cache.insert_ready(item.clone(), 2, 100).unwrap(),
            CacheInsertOutcome::Degraded { .. }
        ));
        cache.allow_rebuild(&item);
        assert!(matches!(
            cache.insert_ready(item, 2, 100).unwrap(),
            CacheInsertOutcome::Stored { .. }
        ));
    }

    #[test]
    fn clear_preserves_pinned_and_in_use_entries() {
        let mut cache = cache();
        let pinned = key("p", MediaDerivedKind::Thumbnail, "x");
        let in_use = key("u", MediaDerivedKind::Proxy, "x");
        let clearable = key("c", MediaDerivedKind::Waveform, "x");
        cache.insert_ready(pinned.clone(), 2, 100).unwrap();
        cache.insert_ready(in_use.clone(), 2, 100).unwrap();
        cache.insert_ready(clearable, 2, 100).unwrap();
        cache.set_pinned(&pinned, true).unwrap();
        cache.mark_in_use(&in_use).unwrap();
        assert_eq!(cache.clear(), 1);
        assert!(cache.entry(&pinned).is_some());
        assert!(cache.entry(&in_use).is_some());
    }

    #[test]
    fn work_queue_is_bounded_and_deduplicates_active_identity() {
        let mut queue = MediaDerivedWorkQueue::new(1, 2).unwrap();
        let playback = key("p", MediaDerivedKind::Proxy, "playback");
        let background = key("b", MediaDerivedKind::Thumbnail, "background");
        let first = queue
            .submit(playback.clone(), MediaDerivedPriority::Playback)
            .unwrap();
        assert_eq!(
            queue
                .submit(playback, MediaDerivedPriority::Background)
                .unwrap(),
            first
        );
        queue
            .submit(background.clone(), MediaDerivedPriority::Background)
            .unwrap();
        assert_eq!(queue.queued_len(), 2);
        assert_eq!(
            queue.submit(
                key("third", MediaDerivedKind::Analysis, "x"),
                MediaDerivedPriority::Background
            ),
            Err(CacheError::QueueFull)
        );
        let started = queue.start_next().unwrap();
        assert_eq!(started.key, key("p", MediaDerivedKind::Proxy, "playback"));
        assert_eq!(queue.running_len(), 1);
        assert!(queue.start_next().is_none());
    }

    #[test]
    fn priority_queue_prefers_next_over_background_and_keeps_fifo_for_ties() {
        let mut queue = MediaDerivedWorkQueue::new(1, 4).unwrap();
        let background = queue
            .submit(
                key("b", MediaDerivedKind::Thumbnail, "b"),
                MediaDerivedPriority::Background,
            )
            .unwrap();
        let next = queue
            .submit(
                key("n", MediaDerivedKind::Proxy, "n"),
                MediaDerivedPriority::Next,
            )
            .unwrap();
        let next_two = queue
            .submit(
                key("n2", MediaDerivedKind::Waveform, "n2"),
                MediaDerivedPriority::Next,
            )
            .unwrap();
        assert_eq!(queue.start_next().unwrap().operation_id, next);
        queue
            .finish(next, MediaDerivedOperationState::Ready)
            .unwrap();
        assert_eq!(queue.start_next().unwrap().operation_id, next_two);
        queue
            .finish(next_two, MediaDerivedOperationState::Ready)
            .unwrap();
        assert_eq!(queue.start_next().unwrap().operation_id, background);
    }

    #[test]
    fn cancellation_distinguishes_queued_cancel_from_running_cancel_request() {
        let mut queue = MediaDerivedWorkQueue::new(1, 4).unwrap();
        let queued = queue
            .submit(
                key("q", MediaDerivedKind::Thumbnail, "q"),
                MediaDerivedPriority::Background,
            )
            .unwrap();
        assert!(queue.request_cancel(queued).unwrap());
        assert_eq!(
            queue.status(queued).unwrap().state,
            MediaDerivedOperationState::Cancelled
        );
        assert!(queue.start_next().is_none());

        let running = queue
            .submit(
                key("r", MediaDerivedKind::Analysis, "r"),
                MediaDerivedPriority::Playback,
            )
            .unwrap();
        queue.start_next().unwrap();
        assert!(queue.request_cancel(running).unwrap());
        assert_eq!(
            queue.status(running).unwrap().state,
            MediaDerivedOperationState::CancelRequested
        );
        assert!(queue.is_cancel_requested(running).unwrap());
        assert_eq!(
            queue
                .finish(running, MediaDerivedOperationState::Ready)
                .unwrap()
                .state,
            MediaDerivedOperationState::Cancelled
        );
    }

    #[test]
    fn progress_is_bounded_and_terminal_result_cannot_be_overwritten() {
        let mut queue = MediaDerivedWorkQueue::new(1, 2).unwrap();
        let operation = queue
            .submit(
                key("a", MediaDerivedKind::Analysis, "a"),
                MediaDerivedPriority::Playback,
            )
            .unwrap();
        queue.start_next().unwrap();
        queue.set_progress(operation, 5_000).unwrap();
        assert_eq!(queue.status(operation).unwrap().progress_millis, 1000);
        assert_eq!(
            queue
                .finish(
                    operation,
                    MediaDerivedOperationState::Degraded("slow disk".into())
                )
                .unwrap()
                .state,
            MediaDerivedOperationState::Degraded("slow disk".into())
        );
        assert_eq!(
            queue.set_progress(operation, 1),
            Err(CacheError::InvalidOperationState)
        );
        assert_eq!(
            queue.finish(operation, MediaDerivedOperationState::Ready),
            Err(CacheError::InvalidOperationState)
        );
    }

    #[test]
    fn terminal_operation_can_restart_with_new_identity_and_progress() {
        let mut queue = MediaDerivedWorkQueue::new(1, 2).unwrap();
        let operation = queue
            .submit(
                key("a", MediaDerivedKind::Proxy, "a"),
                MediaDerivedPriority::Next,
            )
            .unwrap();
        queue.start_next().unwrap();
        queue
            .finish(
                operation,
                MediaDerivedOperationState::Failed("decoder".into()),
            )
            .unwrap();
        let restarted = queue.restart(operation).unwrap();
        assert_ne!(operation, restarted);
        let snapshot = queue.status(restarted).unwrap();
        assert_eq!(snapshot.state, MediaDerivedOperationState::Queued);
        assert_eq!(snapshot.progress_millis, 0);
    }

    #[test]
    fn unsupported_or_pressure_states_are_truthful_terminal_values() {
        let mut queue = MediaDerivedWorkQueue::new(1, 2).unwrap();
        for (kind, state) in [
            (
                MediaDerivedKind::Proxy,
                MediaDerivedOperationState::Degraded("unsupported codec".into()),
            ),
            (
                MediaDerivedKind::Waveform,
                MediaDerivedOperationState::Failed("read error".into()),
            ),
        ] {
            let operation = queue
                .submit(key("same", kind, "v1"), MediaDerivedPriority::Background)
                .unwrap();
            queue.start_next().unwrap();
            let result = queue.finish(operation, state.clone()).unwrap();
            assert_eq!(result.state, state);
        }
    }
}
