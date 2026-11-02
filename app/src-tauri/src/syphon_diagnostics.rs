//! Bounded, best-effort Syphon evidence. No output admission or lifecycle policy.
//! Only the native sender supplies metadata; fake senders never touch a profile.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_ROUTES: usize = 64;
const MAX_REPORT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NativeMetadata {
    pub macos_version: String,
    pub metal_device_name: Option<String>,
    pub has_unified_memory: Option<bool>,
}

#[derive(Clone, Serialize)]
struct RouteReport {
    output_id: u64,
    server_name: String,
    native: NativeMetadata,
    frames_published: u64,
    last_publish_latency_ms: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    route_state: &'static str,
    native_retired: bool,
    last_error: Option<String>,
    output_ownership: protocol::OutputOwnershipStatus,
    observed_unix_ms: u64,
}

#[derive(Default, Serialize)]
struct Report {
    schema_version: u32,
    process_id: u32,
    app_version: &'static str,
    build_commit: Option<&'static str>,
    input_enabled: bool,
    // Publishing/newFrameImage is not an external-client loopback acceptance.
    external_client_verified: bool,
    routes: BTreeMap<u64, RouteReport>,
}

static REPORT: Mutex<Option<Report>> = Mutex::new(None);

pub(super) struct Recorder {
    route: RouteReport,
    destination: Option<PathBuf>,
    write_error_reported: bool,
}

impl Recorder {
    pub(super) fn new(
        output_id: u64,
        name: &str,
        native: NativeMetadata,
        ownership: protocol::OutputOwnershipStatus,
    ) -> Self {
        let destination = if cfg!(all(target_os = "macos", feature = "syphon")) && !cfg!(test) {
            std::env::var_os("HOME").map(PathBuf::from).map(|home| {
                home.join(
                    "Library/Application Support/jp.seraf.ktn.syndocal/logs/syphon-acceptance.json",
                )
            })
        } else {
            None
        };
        Self {
            route: RouteReport {
                output_id,
                server_name: name.to_owned(),
                native,
                frames_published: 0,
                last_publish_latency_ms: None,
                width: None,
                height: None,
                route_state: "constructed",
                native_retired: false,
                last_error: None,
                output_ownership: ownership,
                observed_unix_ms: 0,
            },
            destination,
            write_error_reported: false,
        }
    }

    pub(super) fn published(&mut self, width: u32, height: u32, elapsed: Duration) {
        self.route.frames_published = self.route.frames_published.saturating_add(1);
        self.route.width = Some(width);
        self.route.height = Some(height);
        self.route.last_publish_latency_ms = Some(elapsed.as_secs_f64() * 1000.0);
        self.route.route_state = "publishing";
    }

    pub(super) fn error(&mut self, error: &str) {
        // Bound even framework/driver-provided error strings at a UTF-8 boundary.
        self.route.last_error = Some(error.chars().take(2048).collect());
        self.route.route_state = "failed";
    }

    pub(super) fn native_closed(&mut self) {
        self.route.native_retired = true;
        self.route.route_state = if self.route.last_error.is_some() {
            "native_closed_after_error"
        } else {
            "native_closed"
        };
    }

    pub(super) fn snapshot(&mut self, ownership: protocol::OutputOwnershipStatus) {
        self.route.output_ownership = ownership;
        self.route.observed_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let Some(destination) = self.destination.as_ref() else {
            return;
        };
        if let Err(error) = write_snapshot(destination, &self.route) {
            if !self.write_error_reported {
                eprintln!("Syphon diagnostics could not be written: {error}");
                self.write_error_reported = true;
            }
        }
    }
}

fn write_snapshot(destination: &std::path::Path, route: &RouteReport) -> Result<(), String> {
    let mut guard = REPORT
        .lock()
        .map_err(|_| "Syphon diagnostic report lock was poisoned".to_owned())?;
    let report = guard.get_or_insert_with(|| Report {
        schema_version: 1,
        process_id: std::process::id(),
        app_version: env!("CARGO_PKG_VERSION"),
        build_commit: option_env!("GITHUB_SHA"),
        input_enabled: false,
        external_client_verified: false,
        routes: BTreeMap::new(),
    });
    if !report.routes.contains_key(&route.output_id) && report.routes.len() >= MAX_ROUTES {
        let oldest = report
            .routes
            .iter()
            .filter(|(_, route)| route.native_retired)
            .min_by_key(|(_, route)| route.observed_unix_ms)
            .map(|(id, _)| *id)
            .ok_or_else(|| "Syphon diagnostic route limit reached".to_owned())?;
        report.routes.remove(&oldest);
    }
    report.routes.insert(route.output_id, route.clone());
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("Syphon diagnostic report exceeds bounded size".to_owned());
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "Syphon diagnostic directory is missing".to_owned())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let staging = destination.with_extension("json.tmp");
    std::fs::write(&staging, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(staging, destination).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorder() -> Recorder {
        Recorder::new(
            92,
            "Syndocal-Syphon-Test",
            NativeMetadata {
                macos_version: "test".into(),
                metal_device_name: Some("test GPU".into()),
                has_unified_memory: Some(true),
            },
            protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Video),
        )
    }

    #[test]
    fn syphon_diagnostic_publication_and_retirement_preserve_error_and_counters() {
        let mut recorder = recorder();
        assert!(
            recorder.destination.is_none(),
            "tests never write the owner profile"
        );
        recorder.published(1920, 1080, Duration::from_micros(1500));
        recorder.error("Metal publication failed");
        recorder.native_closed();
        recorder.snapshot(protocol::OutputOwnershipStatus::for_role(
            protocol::MachineOutputRole::Standby,
        ));
        let json = serde_json::to_value(&recorder.route).unwrap();
        assert_eq!(json["frames_published"], 1);
        assert_eq!(json["last_publish_latency_ms"], 1.5);
        assert_eq!(json["native_retired"], true);
        assert_eq!(json["route_state"], "native_closed_after_error");
        assert_eq!(json["last_error"], "Metal publication failed");
        assert_eq!(json["output_ownership"]["video_allowed"], false);
    }

    #[test]
    fn syphon_diagnostic_failed_close_never_claims_native_retirement() {
        let mut recorder = recorder();
        recorder.error(&"映".repeat(3000));
        recorder.snapshot(protocol::OutputOwnershipStatus::for_role(
            protocol::MachineOutputRole::Standby,
        ));
        assert!(!recorder.route.native_retired);
        assert_eq!(recorder.route.route_state, "failed");
        assert_eq!(
            recorder.route.last_error.as_ref().unwrap().chars().count(),
            2048
        );
        assert_eq!(recorder.route.frames_published, 0);
    }

    #[test]
    fn syphon_diagnostic_atomic_report_keeps_multiple_routes_and_is_not_physical_acceptance() {
        let directory = std::env::temp_dir().join(format!(
            "syndocal-syphon-diagnostics-{}",
            std::process::id()
        ));
        let destination = directory.join("syphon-acceptance.json");
        let mut first = recorder();
        first.destination = Some(destination.clone());
        first.published(2, 2, Duration::from_millis(2));
        first.native_closed();
        first.snapshot(protocol::OutputOwnershipStatus::for_role(
            protocol::MachineOutputRole::Standby,
        ));
        let mut second = recorder();
        second.route.output_id = 93;
        second.destination = Some(destination.clone());
        second.error("injected constructor failure");
        second.snapshot(protocol::OutputOwnershipStatus::for_role(
            protocol::MachineOutputRole::Standby,
        ));
        let bytes = std::fs::read(&destination).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["input_enabled"], false);
        assert_eq!(json["external_client_verified"], false);
        assert_eq!(json["routes"]["92"]["frames_published"], 1);
        assert_eq!(json["routes"]["92"]["native_retired"], true);
        assert_eq!(json["routes"]["93"]["frames_published"], 0);
        assert_eq!(
            json["routes"]["93"]["last_error"],
            "injected constructor failure"
        );
        assert!(!destination.with_extension("json.tmp").exists());
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
