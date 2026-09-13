//! Stable identity and observable lifecycle state for live video sources.
//!
//! The project snapshot owns the source kind and endpoint. This module owns
//! the runtime-facing identity and state vocabulary so a reconnect or source
//! replacement cannot be confused with an unrelated route.

use serde::{Deserialize, Serialize};

pub const LIVE_VIDEO_SOURCE_IDENTITY_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveVideoSourceIdentity {
    pub version: u8,
    pub backend_id: String,
    pub endpoint_name: String,
}

impl LiveVideoSourceIdentity {
    pub fn new(backend_id: &str, endpoint_name: &str) -> Result<Self, String> {
        let backend_id = backend_id.trim().to_ascii_lowercase();
        let endpoint_name = endpoint_name.trim().to_string();
        if backend_id.is_empty() {
            return Err("Live video source backend identity is required".to_string());
        }
        if endpoint_name.is_empty() {
            return Err("Live video source endpoint identity is required".to_string());
        }
        if backend_id.chars().any(char::is_control)
            || endpoint_name.chars().any(char::is_control)
        {
            return Err("Live video source identity cannot contain control characters".to_string());
        }
        Ok(Self {
            version: LIVE_VIDEO_SOURCE_IDENTITY_VERSION,
            backend_id,
            endpoint_name,
        })
    }

    /// A length-delimited key avoids collisions from separators in endpoint
    /// names while remaining stable across process restarts and route moves.
    pub fn stable_key(&self) -> String {
        format!(
            "v{}:{}:{}:{}:{}",
            self.version,
            self.backend_id.len(),
            self.backend_id,
            self.endpoint_name.len(),
            self.endpoint_name
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiveVideoSourceState {
    Disabled,
    Unavailable,
    Ready,
    Starting,
    Live,
    Fault,
    Retiring,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveVideoSourceRuntimeStatus {
    pub route_id: u64,
    pub identity: String,
    pub generation: u64,
    pub state: LiveVideoSourceState,
    pub issue: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_trimmed_case_stable_and_length_delimited() {
        let identity = LiveVideoSourceIdentity::new(" NDI ", " Stage:Main ").unwrap();
        assert_eq!(identity.backend_id, "ndi");
        assert_eq!(identity.endpoint_name, "Stage:Main");
        assert_eq!(identity.stable_key(), "v1:3:ndi:10:Stage:Main");
        assert_eq!(
            identity.stable_key(),
            LiveVideoSourceIdentity::new("ndi", "Stage:Main")
                .unwrap()
                .stable_key()
        );
    }

    #[test]
    fn identity_rejects_empty_and_controlled_values() {
        assert!(LiveVideoSourceIdentity::new("", "Stage").is_err());
        assert!(LiveVideoSourceIdentity::new("ndi", " ").is_err());
        assert!(LiveVideoSourceIdentity::new("ndi", "Stage\nMain").is_err());
    }
}
