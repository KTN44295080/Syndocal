//! SDK-independent NDI discovery identity and receiver signal policy.
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdiSourceDescriptor {
    pub name: String,
    /// Complete SDK advertisement, including host qualification.
    pub endpoint_name: String,
}

pub(crate) fn validate_source_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.trim() != name
        || name.len() > 4096
        || name.chars().any(char::is_control)
    {
        return Err(
            "NDI source name is empty, non-canonical, too long, or contains control characters"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn parse_discovered_sources(
    names: impl IntoIterator<Item = String>,
) -> Result<Vec<NdiSourceDescriptor>, String> {
    let mut unique = BTreeSet::new();
    for name in names {
        validate_source_name(&name)?;
        if !unique.insert(name.clone()) {
            return Err(format!(
                "NDI source identity '{name}' is ambiguous; duplicate advertisements were rejected"
            ));
        }
    }
    Ok(unique
        .into_iter()
        .map(|name| NdiSourceDescriptor {
            endpoint_name: name.clone(),
            name,
        })
        .collect())
}

pub(crate) fn exact_source_index<'a>(
    endpoint_name: &str,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<Option<usize>, String> {
    validate_source_name(endpoint_name)?;
    let mut selected = None;
    for (index, name) in names.into_iter().enumerate() {
        if name == endpoint_name {
            if selected.is_some() {
                return Err(format!("NDI source identity '{endpoint_name}' is ambiguous; duplicate advertisements were rejected"));
            }
            selected = Some(index);
        }
    }
    Ok(selected)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NdiInputSignal {
    NoSignal,
    Connected,
    Lost,
}

pub(crate) struct NdiInputState<T> {
    latest: Option<Arc<T>>,
    last_frame_at: Option<Instant>,
    ever_connected: bool,
    detail: String,
}

impl<T> NdiInputState<T> {
    pub(crate) fn new() -> Self {
        Self {
            latest: None,
            last_frame_at: None,
            ever_connected: false,
            detail: "waiting for the selected source".to_string(),
        }
    }
    fn signal(&self) -> NdiInputSignal {
        if self.latest.is_some() {
            NdiInputSignal::Connected
        } else if self.ever_connected {
            NdiInputSignal::Lost
        } else {
            NdiInputSignal::NoSignal
        }
    }
    pub(crate) fn publish(&mut self, frame: T, now: Instant) {
        self.latest = Some(Arc::new(frame));
        self.last_frame_at = Some(now);
        self.ever_connected = true;
        self.detail.clear();
    }
    pub(crate) fn invalidate(&mut self, detail: impl Into<String>) {
        self.latest = None;
        self.last_frame_at = None;
        self.detail = detail.into();
    }
    pub(crate) fn expire(&mut self, now: Instant, timeout: Duration) -> bool {
        if self
            .last_frame_at
            .is_some_and(|last| now.saturating_duration_since(last) >= timeout)
        {
            self.invalidate("no video frames received; reconnecting to the selected source");
            return true;
        }
        false
    }
    pub(crate) fn latest(&self) -> Option<Arc<T>> {
        self.latest.clone()
    }
    pub(crate) fn current_error(&self, endpoint_name: &str) -> Option<String> {
        let prefix = match self.signal() {
            NdiInputSignal::Connected => return None,
            NdiInputSignal::NoSignal => "NDI input has no signal",
            NdiInputSignal::Lost => "NDI input signal lost",
        };
        Some(format!("{prefix} for '{endpoint_name}': {}", self.detail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_preserves_full_identity_and_sorts_deterministically() {
        let sources = parse_discovered_sources([
            "STAGE-B (Program)".to_string(),
            "STAGE-A (Program)".to_string(),
            "配信PC (映像)".to_string(),
        ])
        .unwrap();
        assert_eq!(sources[0].name, "STAGE-A (Program)");
        assert_eq!(sources[1].name, "STAGE-B (Program)");
        assert_eq!(sources[2].name, "配信PC (映像)");
        assert!(sources
            .iter()
            .all(|source| source.name == source.endpoint_name));
    }
    #[test]
    fn discovery_rejects_ambiguous_or_invalid_advertisements() {
        assert!(parse_discovered_sources(["PC (A)".to_string(), "PC (A)".to_string()]).is_err());
        for name in ["", "   ", " PC (A)", "PC (A) ", "PC\0(A)", "PC\n(A)"] {
            assert!(parse_discovered_sources([name.to_string()]).is_err());
        }
        assert!(parse_discovered_sources(Vec::<String>::new())
            .unwrap()
            .is_empty());
    }
    #[test]
    fn selection_is_exact_and_never_uses_suffix_case_or_first_source() {
        let names = ["STAGE-A (Program)", "STAGE-B (Program)"];
        assert_eq!(
            exact_source_index("STAGE-B (Program)", names).unwrap(),
            Some(1)
        );
        for selected in ["Program", "stage-a (Program)", "STAGE-C (Program)"] {
            assert_eq!(exact_source_index(selected, names).unwrap(), None);
        }
        assert!(exact_source_index("PC (A)", ["PC (A)", "PC (A)"]).is_err());
    }
    #[test]
    fn receiver_has_no_signal_until_first_valid_frame() {
        let mut state = NdiInputState::<u8>::new();
        assert_eq!(state.signal(), NdiInputSignal::NoSignal);
        state.invalidate("source not found; retrying discovery");
        assert!(state
            .current_error("PC (A)")
            .unwrap()
            .starts_with("NDI input has no signal"));
        state.publish(7, Instant::now());
        assert_eq!(state.signal(), NdiInputSignal::Connected);
        assert!(state.current_error("PC (A)").is_none());
    }
    #[test]
    fn receiver_broadcasts_latest_snapshot_to_independent_consumers() {
        let mut state = NdiInputState::new();
        let now = Instant::now();
        state.publish(1_u8, now);
        let preview = state.latest().unwrap();
        let output = state.latest().unwrap();
        assert!(Arc::ptr_eq(&preview, &output));
        state.publish(2, now);
        assert_eq!(*state.latest().unwrap(), 2);
        assert_eq!(*preview, 1);
    }
    #[test]
    fn receiver_timeout_discards_frame_then_reconnect_recovers() {
        let mut state = NdiInputState::new();
        let started = Instant::now();
        let timeout = Duration::from_secs(2);
        state.publish(1_u8, started);
        assert!(!state.expire(started + timeout - Duration::from_millis(1), timeout));
        assert!(state.expire(started + timeout, timeout));
        assert_eq!(state.signal(), NdiInputSignal::Lost);
        assert!(state.latest().is_none());
        assert!(state
            .current_error("PC (A)")
            .unwrap()
            .starts_with("NDI input signal lost"));
        state.invalidate("source disappeared; retrying discovery");
        assert_eq!(state.signal(), NdiInputSignal::Lost);
        state.publish(2, started + timeout);
        assert_eq!(*state.latest().unwrap(), 2);
        assert!(state.current_error("PC (A)").is_none());
    }
    #[test]
    fn invalid_frame_or_capture_error_clears_published_pixels() {
        let mut state = NdiInputState::new();
        state.publish(1_u8, Instant::now());
        state.invalidate("unsupported pixel format; reconnecting");
        assert!(state.latest().is_none());
        assert_eq!(state.signal(), NdiInputSignal::Lost);
    }
}
