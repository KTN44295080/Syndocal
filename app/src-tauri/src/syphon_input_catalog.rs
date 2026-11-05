//! Read-only discovery; no client, layer, or output is created here.
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct DiscoveredSyphonServer {
    pub name: String,
    pub app_name: String,
    pub uuid: String,
}

#[tauri::command]
pub(crate) async fn discover_syphon_sources() -> Result<Vec<DiscoveredSyphonServer>, String> {
    tauri::async_runtime::spawn_blocking(discover)
        .await
        .map_err(|error| format!("Syphon server discovery failed: {error}"))?
}

fn discover() -> Result<Vec<DiscoveredSyphonServer>, String> {
    #[cfg(all(target_os = "macos", feature = "syphon"))]
    return crate::syphon_native::list_servers().map(|servers| {
        servers
            .into_iter()
            .map(|server| DiscoveredSyphonServer {
                name: server.name,
                app_name: server.app_name,
                uuid: server.uuid,
            })
            .collect()
    });
    #[cfg(not(all(target_os = "macos", feature = "syphon")))]
    Err("Syphon input discovery requires a macOS build with the syphon feature".to_owned())
}

#[cfg(test)]
mod tests {
    #[cfg(not(all(target_os = "macos", feature = "syphon")))]
    #[test]
    fn syphon_discovery_is_fail_closed_without_native_backend() {
        assert!(super::discover()
            .unwrap_err()
            .contains("macOS build with the syphon feature"));
    }
}
