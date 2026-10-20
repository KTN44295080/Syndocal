//! Read-only, bounded NDI source discovery for the layer picker.
//! The SDK stays behind the existing optional NDI feature and runtime loader.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct NdiDiscoveredSource {
    pub name: String,
    pub endpoint_name: String,
}

#[derive(Debug, Serialize)]
pub struct NdiSourceCatalog {
    pub state: &'static str,
    pub sources: Vec<NdiDiscoveredSource>,
    pub message: Option<String>,
}

fn unavailable(state: &'static str, message: impl Into<String>) -> NdiSourceCatalog {
    NdiSourceCatalog {
        state,
        sources: Vec::new(),
        message: Some(message.into()),
    }
}

/// Discovery never creates a layer, changes the project, or starts an output.
/// Blocking SDK discovery stays off Tauri's async executor.
#[tauri::command]
pub async fn list_ndi_sources_v1() -> NdiSourceCatalog {
    match tauri::async_runtime::spawn_blocking(discover_catalog).await {
        Ok(catalog) => catalog,
        Err(error) => unavailable("error", format!("NDI source discovery failed: {error}")),
    }
}

#[cfg(not(feature = "ndi"))]
fn discover_catalog() -> NdiSourceCatalog {
    unavailable("not_built", "NDI input is not enabled in this build")
}

#[cfg(feature = "ndi")]
fn discover_catalog() -> NdiSourceCatalog {
    use std::{sync::Mutex, time::Duration};

    // Multiple windows may request a refresh. Keep at most one SDK finder alive
    // for this command, and do not queue stale refresh requests behind it.
    static DISCOVERY: Mutex<()> = Mutex::new(());
    let _discovery = match DISCOVERY.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => {
            return unavailable("error", "NDI source discovery is already running");
        }
        Err(std::sync::TryLockError::Poisoned(_)) => {
            return unavailable("error", "NDI source discovery state is unavailable");
        }
    };
    match io::ndi::discover_sources(Duration::from_millis(1500)) {
        Ok(sources) => NdiSourceCatalog {
            state: "available",
            sources: sources
                .into_iter()
                .map(|source| NdiDiscoveredSource {
                    name: source.name,
                    endpoint_name: source.endpoint_name,
                })
                .collect(),
            message: None,
        },
        Err(io::ndi::NdiError::RuntimeMissing(error)) => unavailable(
            "runtime_missing",
            format!("NDI runtime not installed: {error}"),
        ),
        Err(error) => unavailable("error", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_failure_cannot_retain_selectable_sources() {
        let result = unavailable("runtime_missing", "NDI runtime not installed");
        let wire = serde_json::to_value(result).unwrap();
        assert_eq!(wire["state"], "runtime_missing");
        assert_eq!(wire["sources"], serde_json::json!([]));
        assert_eq!(wire["message"], "NDI runtime not installed");
    }

    #[cfg(not(feature = "ndi"))]
    #[test]
    fn disabled_feature_reports_not_built_without_loading_the_sdk() {
        let result = discover_catalog();
        assert_eq!(result.state, "not_built");
        assert!(result.sources.is_empty());
        assert!(result.message.unwrap().contains("not enabled"));
    }
}
