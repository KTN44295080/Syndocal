//! Machine-local E3 recovery acceptance pause/trace support.
//!
//! Production is completely unaffected unless an operator explicitly sets
//! [`E3_ACCEPTANCE_CONTROL_DIR_ENV`].  When enabled, the registered recovery
//! paths publish an atomic ready trace at one of two durable linearization
//! points, then wait for an exact, incarnation-bound resume trace. A timeout
//! is recorded durably as an acceptance failure, but the already-committed
//! production publication/acknowledgement continues truthfully.

use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

pub(crate) const E3_ACCEPTANCE_CONTROL_DIR_ENV: &str = "SYNDOCAL_E3_ACCEPTANCE_CONTROL_DIR";
pub(crate) const E3_ACCEPTANCE_TIMEOUT_MS_ENV: &str = "SYNDOCAL_E3_ACCEPTANCE_TIMEOUT_MS";

const DEFAULT_TIMEOUT_MS: u64 = 300_000;
const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 3_600_000;
const POLL_INTERVAL: Duration = Duration::from_millis(20);
const TRACE_SCHEMA_VERSION: u16 = 1;
const MAX_CONTROL_FILE_BYTES: u64 = 64 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub(crate) struct E3NativeAcceptanceConfig {
    control_dir: PathBuf,
    timeout: Duration,
}

impl E3NativeAcceptanceConfig {
    fn new(control_dir: PathBuf, timeout: Duration) -> Result<Self, String> {
        let current_exe = env::current_exe()
            .map_err(|error| format!("Unable to resolve the E3 acceptance executable: {error}"))?;
        let executable_parent = current_exe.parent().ok_or_else(|| {
            format!(
                "E3 acceptance executable has no parent: {}",
                current_exe.display()
            )
        })?;
        Self::new_with_executable_parent(control_dir, timeout, executable_parent)
    }

    fn new_with_executable_parent(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
    ) -> Result<Self, String> {
        Self::new_with_executable_parent_and_probe(
            control_dir,
            timeout,
            executable_parent,
            |path| {
                fs::symlink_metadata(path).map_err(|error| {
                    format!(
                        "Unable to inspect E3 acceptance control path {}: {error}",
                        path.display()
                    )
                })
            },
        )
    }

    fn new_with_executable_parent_and_probe<Probe>(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
        probe: Probe,
    ) -> Result<Self, String>
    where
        Probe: FnMut(&Path) -> Result<fs::Metadata, String>,
    {
        Self::new_with_executable_parent_and_probes(
            control_dir,
            timeout,
            executable_parent,
            probe,
            validate_fixed_local_drive_paths,
        )
    }

    fn new_with_executable_parent_and_probes<Probe, DriveValidate>(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
        mut probe: Probe,
        mut validate_drives: DriveValidate,
    ) -> Result<Self, String>
    where
        Probe: FnMut(&Path) -> Result<fs::Metadata, String>,
        DriveValidate: FnMut(&Path, &Path) -> Result<(), String>,
    {
        if !control_dir.is_absolute() {
            return Err(format!(
                "E3 acceptance control directory must be absolute: {}",
                control_dir.display()
            ));
        }
        validate_control_directory_lexically(&control_dir, executable_parent)?;
        let timeout_ms = u64::try_from(timeout.as_millis())
            .map_err(|_| "E3 acceptance timeout exceeds the supported integer range".to_string())?;
        if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
            return Err(format!(
                "E3 acceptance timeout must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS} milliseconds"
            ));
        }
        // Drive classification consumes only the raw root prefix. It must run
        // before metadata or canonicalization so a mapped executable/control
        // path cannot trigger remote descendant resolution before rejection.
        validate_drives(executable_parent, &control_dir)?;
        let canonical_executable_parent = executable_parent.canonicalize().map_err(|error| {
            format!(
                "Unable to canonicalize the E3 acceptance executable parent {}: {error}",
                executable_parent.display()
            )
        })?;
        let relative = control_dir.strip_prefix(executable_parent).map_err(|_| {
            "E3 acceptance control directory escaped its executable parent".to_string()
        })?;
        let mut current = executable_parent.to_path_buf();
        let mut final_metadata = None;
        for component in relative.components() {
            match component {
                Component::Normal(component) => current.push(component),
                Component::CurDir => continue,
                _ => {
                    return Err(
                        "E3 acceptance control directory contains an unsafe path component"
                            .to_string(),
                    )
                }
            }
            let metadata = probe(&current)?;
            if metadata_is_reparse_point(&metadata) {
                return Err(format!(
                    "E3 acceptance control directory contains a reparse or symbolic-link component: {}",
                    current.display()
                ));
            }
            final_metadata = Some(metadata);
        }
        if !final_metadata.is_some_and(|metadata| metadata.is_dir()) {
            return Err(format!(
                "E3 acceptance control directory does not exist or is not a directory: {}",
                control_dir.display()
            ));
        }
        let canonical_control_dir = control_dir.canonicalize().map_err(|error| {
            format!(
                "Unable to canonicalize the E3 acceptance control directory {}: {error}",
                control_dir.display()
            )
        })?;
        validate_canonical_control_directory(&canonical_control_dir, &canonical_executable_parent)?;
        validate_drives(&canonical_executable_parent, &canonical_control_dir)?;
        Ok(Self {
            control_dir: canonical_control_dir,
            timeout,
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
    ) -> Result<Self, String> {
        Self::new_with_executable_parent(control_dir, timeout, executable_parent)
    }

    #[cfg(test)]
    fn for_test_with_probe<Probe>(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
        probe: Probe,
    ) -> Result<Self, String>
    where
        Probe: FnMut(&Path) -> Result<fs::Metadata, String>,
    {
        Self::new_with_executable_parent_and_probe(control_dir, timeout, executable_parent, probe)
    }

    #[cfg(test)]
    fn for_test_with_probes<Probe, DriveValidate>(
        control_dir: PathBuf,
        timeout: Duration,
        executable_parent: &Path,
        probe: Probe,
        validate_drives: DriveValidate,
    ) -> Result<Self, String>
    where
        Probe: FnMut(&Path) -> Result<fs::Metadata, String>,
        DriveValidate: FnMut(&Path, &Path) -> Result<(), String>,
    {
        Self::new_with_executable_parent_and_probes(
            control_dir,
            timeout,
            executable_parent,
            probe,
            validate_drives,
        )
    }
}

fn validate_control_directory_lexically(
    control_dir: &Path,
    executable_parent: &Path,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::Prefix;

        if let Some(Component::Prefix(prefix)) = control_dir.components().next() {
            if matches!(
                prefix.kind(),
                Prefix::UNC(_, _)
                    | Prefix::VerbatimUNC(_, _)
                    | Prefix::DeviceNS(_)
                    | Prefix::Verbatim(_)
            ) {
                return Err(
                    "E3 acceptance control directory must not use a remote or device path prefix"
                        .to_string(),
                );
            }
        }
    }
    if control_dir == executable_parent || !control_dir.starts_with(executable_parent) {
        return Err(format!(
            "E3 acceptance control directory must be lexically below the current executable directory: {}",
            control_dir.display()
        ));
    }
    let relative = control_dir
        .strip_prefix(executable_parent)
        .map_err(|_| "E3 acceptance control directory escaped its executable parent".to_string())?;
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(
            "E3 acceptance control directory contains an unsafe parent or root component"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn validate_canonical_control_directory(
    canonical_control_dir: &Path,
    canonical_executable_parent: &Path,
) -> Result<(), String> {
    if canonical_control_dir == canonical_executable_parent
        || !canonical_control_dir.starts_with(canonical_executable_parent)
    {
        return Err(format!(
            "E3 acceptance control directory must resolve below the current executable directory: {}",
            canonical_control_dir.display()
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_fixed_local_drive_paths(
    canonical_executable_parent: &Path,
    canonical_control_dir: &Path,
) -> Result<(), String> {
    let executable_drive_type = windows_drive_type(canonical_executable_parent)?;
    let control_drive_type = windows_drive_type(canonical_control_dir)?;
    validate_fixed_drive_types(executable_drive_type, control_drive_type)
}

#[cfg(not(windows))]
fn validate_fixed_local_drive_paths(
    _canonical_executable_parent: &Path,
    _canonical_control_dir: &Path,
) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn windows_drive_type(path: &Path) -> Result<u32, String> {
    use std::{os::windows::ffi::OsStrExt, path::Prefix};
    use windows::{core::PCWSTR, Win32::Storage::FileSystem::GetDriveTypeW};

    let drive = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => {
                return Err(format!(
                    "E3 acceptance path does not resolve to a local drive: {}",
                    path.display()
                ))
            }
        },
        _ => {
            return Err(format!(
                "E3 acceptance path has no local drive prefix: {}",
                path.display()
            ))
        }
    };
    let root = format!("{}:\\", char::from(drive));
    let mut wide = std::ffi::OsStr::new(&root)
        .encode_wide()
        .collect::<Vec<_>>();
    wide.push(0);
    // SAFETY: `wide` is a live, NUL-terminated UTF-16 drive-root string for
    // the duration of the call.
    Ok(unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) })
}

#[cfg(windows)]
fn validate_fixed_drive_types(
    executable_drive_type: u32,
    control_drive_type: u32,
) -> Result<(), String> {
    // Win32 DRIVE_FIXED. The `windows` crate exposes this constant through a
    // broader optional module than this crate enables; keep the documented
    // ABI value local while calling GetDriveTypeW from the enabled API.
    const DRIVE_FIXED: u32 = 3;
    if executable_drive_type != DRIVE_FIXED || control_drive_type != DRIVE_FIXED {
        return Err(
            "E3 acceptance requires both the executable and control directory on a fixed local drive"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn config_from_environment() -> Result<Option<E3NativeAcceptanceConfig>, String> {
    let Some(control_dir) = env::var_os(E3_ACCEPTANCE_CONTROL_DIR_ENV) else {
        return Ok(None);
    };
    if control_dir.is_empty() {
        return Err(format!(
            "{E3_ACCEPTANCE_CONTROL_DIR_ENV} is present but empty"
        ));
    }
    let timeout_ms = match env::var(E3_ACCEPTANCE_TIMEOUT_MS_ENV) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| format!("{E3_ACCEPTANCE_TIMEOUT_MS_ENV} must be an unsigned integer"))?,
        Err(env::VarError::NotPresent) => DEFAULT_TIMEOUT_MS,
        Err(env::VarError::NotUnicode(_)) => {
            return Err(format!(
                "{E3_ACCEPTANCE_TIMEOUT_MS_ENV} is not valid Unicode"
            ))
        }
    };
    E3NativeAcceptanceConfig::new(
        PathBuf::from(control_dir),
        Duration::from_millis(timeout_ms),
    )
    .map(Some)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum E3NativeAcceptancePhase {
    RecoveryPublicationCommittedBeforeEvent,
    RecoveryAcknowledgedBeforeReply,
}

impl E3NativeAcceptancePhase {
    fn file_stem(self) -> &'static str {
        match self {
            Self::RecoveryPublicationCommittedBeforeEvent => "recovery-publication",
            Self::RecoveryAcknowledgedBeforeReply => "recovery-acknowledged",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct E3NativeAcceptanceTrace {
    pub(crate) schema_version: u16,
    pub(crate) phase: E3NativeAcceptancePhase,
    pub(crate) pid: u32,
    pub(crate) incarnation: u64,
    pub(crate) request_id: String,
    pub(crate) source_serial: u64,
    pub(crate) target_serial: u64,
    pub(crate) target_checkpoint_hash: String,
    pub(crate) journal_serial: u64,
}

pub(crate) struct E3NativeAcceptanceTraceInput {
    pub(crate) request_id: String,
    pub(crate) source_serial: u64,
    pub(crate) target_serial: u64,
    pub(crate) target_checkpoint_hash: String,
    pub(crate) journal_serial: u64,
}

impl E3NativeAcceptanceTrace {
    pub(crate) fn new(
        phase: E3NativeAcceptancePhase,
        pid: u32,
        incarnation: u64,
        input: E3NativeAcceptanceTraceInput,
    ) -> Result<Self, String> {
        let E3NativeAcceptanceTraceInput {
            request_id,
            source_serial,
            target_serial,
            target_checkpoint_hash,
            journal_serial,
        } = input;
        if incarnation == 0 {
            return Err("E3 acceptance process incarnation must be nonzero".to_string());
        }
        let request_id = request_id.trim().to_string();
        if request_id.is_empty() || request_id.len() > 160 {
            return Err("E3 acceptance recovery request ID is missing or too long".to_string());
        }
        let target_checkpoint_hash = target_checkpoint_hash.trim().to_string();
        if target_checkpoint_hash.is_empty() || target_checkpoint_hash.len() > 256 {
            return Err("E3 acceptance target checkpoint hash is missing or too long".to_string());
        }
        if target_serial == 0 || journal_serial != target_serial {
            return Err(
                "E3 acceptance target and durable journal serials do not match".to_string(),
            );
        }
        Ok(Self {
            schema_version: TRACE_SCHEMA_VERSION,
            phase,
            pid,
            incarnation,
            request_id,
            source_serial,
            target_serial,
            target_checkpoint_hash,
            journal_serial,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct E3NativeAcceptanceFault<'a> {
    trace: &'a E3NativeAcceptanceTrace,
    fault: &'static str,
}

fn control_path(
    config: &E3NativeAcceptanceConfig,
    phase: E3NativeAcceptancePhase,
    suffix: &str,
) -> PathBuf {
    config
        .control_dir
        .join(format!("e3-{}-{suffix}.json", phase.file_stem()))
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Unable to encode E3 acceptance trace: {error}"))?;
    if bytes.len() as u64 > MAX_CONTROL_FILE_BYTES {
        return Err(format!(
            "E3 acceptance control payload exceeds the {MAX_CONTROL_FILE_BYTES} byte limit"
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "E3 acceptance control path has no parent: {}",
            path.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("e3-acceptance.json");
    let temp = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| {
                format!(
                    "Unable to create E3 acceptance trace {}: {error}",
                    temp.display()
                )
            })?;
        file.write_all(&bytes).map_err(|error| {
            format!(
                "Unable to write E3 acceptance trace {}: {error}",
                temp.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "Unable to flush E3 acceptance trace {}: {error}",
                temp.display()
            )
        })?;
        drop(file);
        super::replace_file_atomically(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn read_resume(path: &Path) -> Result<Option<E3NativeAcceptanceTrace>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Unable to inspect E3 acceptance resume {}: {error}",
                path.display()
            ))
        }
    };
    if metadata.len() > MAX_CONTROL_FILE_BYTES {
        return Ok(None);
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Unable to read E3 acceptance resume {}: {error}",
                path.display()
            ))
        }
    };
    // An external acceptance driver may be between its own create/write/
    // replace steps. A malformed or stale resume is never authority to
    // continue, so treat it as nonmatching and keep waiting until timeout.
    Ok(serde_json::from_slice(&bytes).ok())
}

pub(crate) fn pause_with_config(
    config: Option<&E3NativeAcceptanceConfig>,
    trace: E3NativeAcceptanceTrace,
) -> Result<(), String> {
    let Some(config) = config else {
        return Ok(());
    };
    let ready_path = control_path(config, trace.phase, "ready");
    let resume_path = control_path(config, trace.phase, "resume");
    let fault_path = control_path(config, trace.phase, "fault");
    write_json_atomically(&ready_path, &trace)?;

    let deadline = Instant::now()
        .checked_add(config.timeout)
        .ok_or_else(|| "E3 acceptance timeout deadline overflowed".to_string())?;
    loop {
        if read_resume(&resume_path)?.as_ref() == Some(&trace) {
            fs::remove_file(&resume_path).map_err(|error| {
                format!(
                    "Unable to consume E3 acceptance resume {}: {error}",
                    resume_path.display()
                )
            })?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            let fault = E3NativeAcceptanceFault {
                trace: &trace,
                fault: "timeout_waiting_for_exact_resume",
            };
            write_json_atomically(&fault_path, &fault)?;
            return Ok(());
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "syndocal-e3-native-acceptance-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn configured_test_directory(
        label: &str,
        timeout: Duration,
    ) -> (PathBuf, E3NativeAcceptanceConfig) {
        let root = test_directory(label);
        let executable_parent = root.join("bin");
        let control_dir = executable_parent.join("e3-acceptance-control");
        fs::create_dir_all(&control_dir).unwrap();
        let config =
            E3NativeAcceptanceConfig::for_test(control_dir, timeout, &executable_parent).unwrap();
        (root, config)
    }

    fn trace(phase: E3NativeAcceptancePhase, request_id: &str) -> E3NativeAcceptanceTrace {
        E3NativeAcceptanceTrace::new(
            phase,
            std::process::id(),
            77,
            E3NativeAcceptanceTraceInput {
                request_id: request_id.to_string(),
                source_serial: 41,
                target_serial: 42,
                target_checkpoint_hash: "checkpoint-b".to_string(),
                journal_serial: 42,
            },
        )
        .unwrap()
    }

    fn wait_for_path(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !path.exists() {
            assert!(
                Instant::now() < deadline,
                "{} was not created",
                path.display()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn no_config_is_an_exact_noop_and_invalid_config_fails_closed() {
        let directory = test_directory("disabled");
        pause_with_config(
            None,
            trace(
                E3NativeAcceptancePhase::RecoveryPublicationCommittedBeforeEvent,
                "disabled-request",
            ),
        )
        .unwrap();
        assert!(!directory.exists());
        let anchor = test_directory("invalid-anchor");
        fs::create_dir_all(&anchor).unwrap();
        assert!(E3NativeAcceptanceConfig::for_test(
            PathBuf::from("relative-control-directory"),
            Duration::from_millis(100),
            &anchor,
        )
        .unwrap_err()
        .contains("must be absolute"));
        assert!(E3NativeAcceptanceConfig::for_test(
            anchor.join("missing-control-directory"),
            Duration::from_millis(100),
            &anchor,
        )
        .unwrap_err()
        .contains("Unable to inspect"));
        let _ = fs::remove_dir_all(anchor);
    }

    #[test]
    fn control_directory_must_canonically_resolve_below_the_executable_parent() {
        let root = test_directory("control-root");
        let executable_parent = root.join("bin");
        let valid = executable_parent.join("e3-acceptance-valid");
        let outside = root.join("outside");
        fs::create_dir_all(&valid).unwrap();
        fs::create_dir_all(executable_parent.join("child")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        E3NativeAcceptanceConfig::for_test(valid, Duration::from_millis(100), &executable_parent)
            .unwrap();
        let mut outside_probes = 0;
        assert!(E3NativeAcceptanceConfig::for_test_with_probe(
            outside.clone(),
            Duration::from_millis(100),
            &executable_parent,
            |_| {
                outside_probes += 1;
                Err("outside path must not be probed".to_string())
            },
        )
        .unwrap_err()
        .contains("lexically below"));
        assert_eq!(outside_probes, 0);
        let traversal = executable_parent
            .join("child")
            .join("..")
            .join("..")
            .join("outside");
        assert!(E3NativeAcceptanceConfig::for_test(
            traversal,
            Duration::from_millis(100),
            &executable_parent,
        )
        .unwrap_err()
        .contains("unsafe parent"));
        let mut unc_probes = 0;
        assert!(E3NativeAcceptanceConfig::for_test_with_probe(
            PathBuf::from(r"\\server\share\e3-acceptance"),
            Duration::from_millis(100),
            &executable_parent,
            |_| {
                unc_probes += 1;
                Err("UNC path must not be probed".to_string())
            },
        )
        .unwrap_err()
        .contains("remote or device"));
        assert_eq!(unc_probes, 0);

        #[cfg(windows)]
        {
            assert!(validate_fixed_drive_types(3, 3).is_ok());
            assert!(validate_fixed_drive_types(4, 3)
                .unwrap_err()
                .contains("fixed local drive"));
            assert!(validate_fixed_drive_types(3, 2)
                .unwrap_err()
                .contains("fixed local drive"));

            let mapped_parent = PathBuf::from(r"Z:\syndocal\bin");
            let mapped_control = mapped_parent.join("e3-acceptance-control");
            let mut mapped_drive_checks = 0;
            let mut mapped_metadata_probes = 0;
            let mapped_error = E3NativeAcceptanceConfig::for_test_with_probes(
                mapped_control,
                Duration::from_millis(100),
                &mapped_parent,
                |_| {
                    mapped_metadata_probes += 1;
                    Err("mapped path must not reach metadata".to_string())
                },
                |_, _| {
                    mapped_drive_checks += 1;
                    Err("injected mapped drive rejection".to_string())
                },
            )
            .unwrap_err();
            assert!(mapped_error.contains("injected mapped drive rejection"));
            assert_eq!(mapped_drive_checks, 1);
            assert_eq!(mapped_metadata_probes, 0);
        }

        #[cfg(windows)]
        {
            let linked = executable_parent.join("e3-acceptance-linked-outside");
            if std::os::windows::fs::symlink_dir(&outside, &linked).is_ok() {
                assert!(E3NativeAcceptanceConfig::for_test(
                    linked,
                    Duration::from_millis(100),
                    &executable_parent,
                )
                .unwrap_err()
                .contains("reparse or symbolic-link"));
            }
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn both_boundaries_require_an_exact_incarnation_and_request_bound_resume() {
        for (index, phase) in [
            E3NativeAcceptancePhase::RecoveryPublicationCommittedBeforeEvent,
            E3NativeAcceptancePhase::RecoveryAcknowledgedBeforeReply,
        ]
        .into_iter()
        .enumerate()
        {
            let (directory, config) =
                configured_test_directory(&format!("resume-{index}"), Duration::from_secs(2));
            let expected = trace(phase, &format!("exact-request-{index}"));
            let ready_path = control_path(&config, phase, "ready");
            let resume_path = control_path(&config, phase, "resume");
            let (sent, received) = mpsc::channel();
            let worker_config = config.clone();
            let worker_trace = expected.clone();
            let worker = thread::spawn(move || {
                let result = pause_with_config(Some(&worker_config), worker_trace);
                sent.send(result).unwrap();
            });
            wait_for_path(&ready_path);
            let ready: E3NativeAcceptanceTrace =
                serde_json::from_slice(&fs::read(&ready_path).unwrap()).unwrap();
            assert_eq!(ready, expected, "ready trace contains every exact fence");

            let mut wrong = expected.clone();
            wrong.request_id.push_str("-stale");
            fs::write(&resume_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
            assert!(received.recv_timeout(Duration::from_millis(80)).is_err());
            wrong = expected.clone();
            wrong.incarnation = wrong.incarnation.checked_add(1).unwrap();
            fs::write(&resume_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
            assert!(received.recv_timeout(Duration::from_millis(80)).is_err());

            fs::write(&resume_path, serde_json::to_vec(&expected).unwrap()).unwrap();
            received
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            worker.join().unwrap();
            assert!(!resume_path.exists());
            assert!(!control_path(&config, phase, "fault").exists());
            let _ = fs::remove_dir_all(directory);
        }
    }

    #[test]
    fn missing_resume_records_a_fault_then_truthfully_continues_the_committed_boundary() {
        let (directory, config) = configured_test_directory("timeout", Duration::from_millis(100));
        let expected = trace(
            E3NativeAcceptancePhase::RecoveryAcknowledgedBeforeReply,
            "timeout-request",
        );
        let mut continuation_count = 0;
        pause_with_config(Some(&config), expected.clone()).unwrap();
        continuation_count += 1;
        assert_eq!(continuation_count, 1);
        let fault_path = control_path(&config, expected.phase, "fault");
        let fault: serde_json::Value =
            serde_json::from_slice(&fs::read(&fault_path).unwrap()).unwrap();
        assert_eq!(
            fault.get("fault").and_then(serde_json::Value::as_str),
            Some("timeout_waiting_for_exact_resume")
        );
        assert_eq!(
            fault
                .get("trace")
                .and_then(|value| value.get("requestId"))
                .and_then(serde_json::Value::as_str),
            Some("timeout-request")
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn every_malformed_or_mismatched_resume_times_out_without_consumption() {
        let expected = trace(
            E3NativeAcceptancePhase::RecoveryPublicationCommittedBeforeEvent,
            "mismatch-matrix-request",
        );
        let mut payloads = vec![
            ("malformed", b"{".to_vec()),
            ("oversize", vec![b'x'; MAX_CONTROL_FILE_BYTES as usize + 1]),
        ];
        let mut push_mismatch = |label: &'static str, mutate: fn(&mut E3NativeAcceptanceTrace)| {
            let mut mismatch = expected.clone();
            mutate(&mut mismatch);
            payloads.push((label, serde_json::to_vec(&mismatch).unwrap()));
        };
        push_mismatch("schema", |trace| trace.schema_version += 1);
        push_mismatch("phase", |trace| {
            trace.phase = E3NativeAcceptancePhase::RecoveryAcknowledgedBeforeReply
        });
        push_mismatch("pid", |trace| trace.pid += 1);
        push_mismatch("incarnation", |trace| trace.incarnation += 1);
        push_mismatch("request", |trace| trace.request_id.push_str("-wrong"));
        push_mismatch("source", |trace| trace.source_serial += 1);
        push_mismatch("target", |trace| trace.target_serial += 1);
        push_mismatch("hash", |trace| {
            trace.target_checkpoint_hash.push_str("-wrong")
        });
        push_mismatch("journal", |trace| trace.journal_serial += 1);

        for (label, payload) in payloads {
            let (directory, config) =
                configured_test_directory(&format!("mismatch-{label}"), Duration::from_millis(100));
            let ready_path = control_path(&config, expected.phase, "ready");
            let resume_path = control_path(&config, expected.phase, "resume");
            let fault_path = control_path(&config, expected.phase, "fault");
            let continuation_count = std::sync::Arc::new(AtomicU64::new(0));
            let worker_count = continuation_count.clone();
            let worker_config = config.clone();
            let worker_trace = expected.clone();
            let (sent, received) = mpsc::channel();
            let worker = thread::spawn(move || {
                let result = pause_with_config(Some(&worker_config), worker_trace);
                if result.is_ok() {
                    worker_count.fetch_add(1, Ordering::SeqCst);
                }
                sent.send(result).unwrap();
            });
            wait_for_path(&ready_path);
            fs::write(&resume_path, &payload).unwrap();
            assert!(
                received.recv_timeout(Duration::from_millis(40)).is_err(),
                "{label} resume released the boundary before timeout"
            );
            assert_eq!(continuation_count.load(Ordering::SeqCst), 0);
            received
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            worker.join().unwrap();
            assert_eq!(continuation_count.load(Ordering::SeqCst), 1);
            assert!(fault_path.exists(), "{label} resume did not record a fault");
            assert_eq!(
                fs::read(&resume_path).unwrap(),
                payload,
                "{label} resume was incorrectly consumed"
            );
            let _ = fs::remove_dir_all(directory);
        }
    }
}
