//! Runtime preflight for the existing grafton NDI SDK binding.
//! Windows binaries delay-load the SDK import DLL (see both build scripts).
//! The guard runs before grafton's first FFI call and retains a verified module.

#[cfg(feature = "ndi")]
pub(crate) fn initialize_ndi() -> Result<grafton_ndi::NDI, crate::ndi::NdiError> {
    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    windows::ensure_loaded().map_err(|error| match error {
        windows::RuntimeLoadError::Missing(message) => {
            crate::ndi::NdiError::RuntimeMissing(message)
        }
        windows::RuntimeLoadError::Invalid(message) => crate::ndi::NdiError::Runtime(message),
    })?;
    // grafton keeps its existing shared initialization/destruction refcount.
    // No caller here invokes NDIlib_initialize or NDIlib_destroy independently.
    grafton_ndi::NDI::new().map_err(|error| crate::ndi::NdiError::Runtime(error.to_string()))
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
mod windows {
    use std::{ffi::OsString, path::PathBuf};

    use libloading::os::windows::{
        Library, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };

    #[cfg(target_arch = "x86_64")]
    const RUNTIME_DLL: &str = "Processing.NDI.Lib.x64.dll";
    #[cfg(target_arch = "x86")]
    const RUNTIME_DLL: &str = "Processing.NDI.Lib.x86.dll";
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    const RUNTIME_DLL: &str = "unsupported-windows-ndi-architecture";

    // Includes constructors, capture/free pairs, and destructors reached by the
    // synchronous sender and receiver paths in grafton-ndi 0.11.0. No Advanced
    // SDK-only asynchronous-completion entry point is required.
    const REQUIRED_SYMBOLS: &[&[u8]] = &[
        b"NDIlib_initialize\0",
        b"NDIlib_destroy\0",
        b"NDIlib_version\0",
        b"NDIlib_is_supported_CPU\0",
        b"NDIlib_find_create_v2\0",
        b"NDIlib_find_destroy\0",
        b"NDIlib_find_wait_for_sources\0",
        b"NDIlib_find_get_current_sources\0",
        b"NDIlib_find_get_sources\0",
        b"NDIlib_recv_create_v3\0",
        b"NDIlib_recv_destroy\0",
        b"NDIlib_recv_capture_v3\0",
        b"NDIlib_recv_free_video_v2\0",
        b"NDIlib_recv_free_audio_v3\0",
        b"NDIlib_recv_free_metadata\0",
        b"NDIlib_recv_get_no_connections\0",
        b"NDIlib_send_create\0",
        b"NDIlib_send_destroy\0",
        b"NDIlib_send_send_video_v2\0",
        b"NDIlib_send_send_video_async_v2\0",
        b"NDIlib_send_get_no_connections\0",
    ];

    #[derive(Debug)]
    pub(super) enum RuntimeLoadError {
        Missing(String),
        Invalid(String),
    }

    fn runtime_candidates(
        lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Vec<PathBuf>, RuntimeLoadError> {
        let checked_directory = |key: &str, value: OsString| {
            let directory = PathBuf::from(value);
            if !directory.is_absolute() {
                return Err(RuntimeLoadError::Invalid(format!(
                    "{key} must be an absolute NDI runtime directory"
                )));
            }
            Ok(directory)
        };
        // An explicit selection is authoritative. Never fall back to PATH,
        // the working directory, or a different installed version if it fails.
        for key in [
            "NDI_RUNTIME_DIR_V6",
            "NDI_RUNTIME_DIR_V5",
            "NDI_RUNTIME_DIR_V4",
            "NDI_RUNTIME_DIR_V3",
            "NDI_RUNTIME_DIR_V2",
        ] {
            if let Some(value) = lookup(key) {
                return Ok(vec![checked_directory(key, value)?.join(RUNTIME_DLL)]);
            }
        }
        let architecture = if cfg!(target_arch = "x86_64") {
            "x64"
        } else {
            "x86"
        };
        if let Some(value) = lookup("NDI_SDK_DIR") {
            return Ok(vec![checked_directory("NDI_SDK_DIR", value)?
                .join("Bin")
                .join(architecture)
                .join(RUNTIME_DLL)]);
        }
        let program_files = lookup("ProgramFiles").ok_or_else(|| {
            RuntimeLoadError::Missing("No NDI runtime or SDK directory is configured".to_string())
        })?;
        let root = checked_directory("ProgramFiles", program_files)?.join("NDI");
        Ok(vec![
            root.join("NDI 6 Runtime").join("v6").join(RUNTIME_DLL),
            root.join("NDI 6 SDK")
                .join("Bin")
                .join(architecture)
                .join(RUNTIME_DLL),
        ])
    }

    fn validate_symbols(library: &Library) -> Result<(), RuntimeLoadError> {
        for symbol in REQUIRED_SYMBOLS {
            // Only resolve the address. The deliberately generic function
            // pointer is never called; grafton supplies each actual ABI.
            unsafe { library.get::<unsafe extern "C" fn()>(symbol) }.map_err(|_| {
                RuntimeLoadError::Invalid(format!(
                    "NDI runtime is incompatible: missing SDK entry point {}",
                    String::from_utf8_lossy(&symbol[..symbol.len() - 1]),
                ))
            })?;
        }
        Ok(())
    }

    fn reject_preloaded_library(name: &str) -> Result<(), RuntimeLoadError> {
        if Library::open_already_loaded(name).is_ok() {
            return Err(RuntimeLoadError::Invalid(
                "NDI runtime was loaded outside the guarded path; restart before selecting a runtime"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn load_runtime(candidates: &[PathBuf]) -> Result<Library, RuntimeLoadError> {
        // The delay helper resolves by basename. An earlier unverified load
        // would make the selected absolute path ambiguous, so reject it.
        reject_preloaded_library(RUNTIME_DLL)?;
        for path in candidates {
            let canonical = match path.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(RuntimeLoadError::Invalid(format!(
                        "Cannot resolve NDI runtime '{}': {error}",
                        path.display(),
                    )))
                }
            };
            if !canonical.is_file() {
                return Err(RuntimeLoadError::Invalid(format!(
                    "NDI runtime '{}' is not a regular file",
                    canonical.display(),
                )));
            }
            // Search only the selected DLL's own directory and System32 for
            // dependencies, never the process PATH or working directory.
            let library = unsafe {
                Library::load_with_flags(
                    &canonical,
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
                )
            }
            .map_err(|error| {
                RuntimeLoadError::Missing(format!(
                    "Cannot load NDI runtime '{}': {error}",
                    canonical.display(),
                ))
            })?;
            validate_symbols(&library)?;
            // Detect a competing basename load before permitting delayed FFI.
            // Each raw handle is immediately restored to its original owner;
            // into_raw/from_raw do not change the Windows reference count.
            let selected_handle = library.into_raw();
            let library = unsafe { Library::from_raw(selected_handle) };
            let basename = Library::open_already_loaded(RUNTIME_DLL).map_err(|error| {
                RuntimeLoadError::Invalid(format!("Cannot verify NDI runtime identity: {error}"))
            })?;
            let basename_handle = basename.into_raw();
            let basename = unsafe { Library::from_raw(basename_handle) };
            if selected_handle != basename_handle {
                return Err(RuntimeLoadError::Invalid(
                    "NDI runtime basename resolves to a different module; restart before using NDI"
                        .to_string(),
                ));
            }
            drop(basename);
            return Ok(library);
        }
        Err(RuntimeLoadError::Missing(
            "Install NDI 6 Runtime or configure NDI_RUNTIME_DIR_V6 / NDI_SDK_DIR".to_string(),
        ))
    }

    #[cfg(feature = "ndi")]
    pub(super) fn ensure_loaded() -> Result<(), RuntimeLoadError> {
        use std::sync::Mutex;

        // A static has no destructor. Holding the Library for the process
        // lifetime keeps delayed-import pointers valid after receiver teardown.
        // Failed attempts are not cached, so Refresh can retry after installation.
        static RUNTIME: Mutex<Option<Library>> = Mutex::new(None);
        let mut runtime = RUNTIME.lock().map_err(|_| {
            RuntimeLoadError::Invalid("NDI runtime loader state was poisoned".to_string())
        })?;
        if runtime.is_none() {
            let candidates = runtime_candidates(|key| std::env::var_os(key))?;
            *runtime = Some(load_runtime(&candidates)?);
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn explicit_runtime_directory_has_priority_and_is_never_path_searched() {
            let paths = runtime_candidates(|key| match key {
                "NDI_RUNTIME_DIR_V6" => Some(OsString::from(r"C:\NDI Runtime\v6")),
                "NDI_SDK_DIR" => Some(OsString::from(r"C:\SDK")),
                "PATH" => panic!("PATH must never be inspected"),
                _ => None,
            })
            .unwrap();
            assert_eq!(
                paths,
                vec![PathBuf::from(r"C:\NDI Runtime\v6").join(RUNTIME_DLL)]
            );
        }

        #[test]
        fn relative_sdk_path_is_rejected_without_fallback() {
            let result = runtime_candidates(|key| match key {
                "NDI_SDK_DIR" => Some(OsString::from("relative-sdk")),
                "ProgramFiles" => Some(OsString::from(r"C:\Program Files")),
                _ => None,
            });
            assert!(
                matches!(result, Err(RuntimeLoadError::Invalid(message)) if message.contains("absolute"))
            );
        }

        #[test]
        fn absent_runtime_returns_missing_without_entering_the_sdk() {
            let result = load_runtime(&[]);
            assert!(
                matches!(result, Err(RuntimeLoadError::Missing(message)) if message.contains("Install NDI 6 Runtime"))
            );
        }

        #[test]
        fn an_already_loaded_module_is_rejected_without_sdk_calls() {
            let result = reject_preloaded_library("kernel32.dll");
            assert!(
                matches!(result, Err(RuntimeLoadError::Invalid(message)) if message.contains("outside the guarded path"))
            );
        }

        #[test]
        fn incompatible_library_is_rejected_before_any_sdk_call() {
            let library = Library::open_already_loaded("kernel32.dll").unwrap();
            let result = validate_symbols(&library);
            assert!(
                matches!(result, Err(RuntimeLoadError::Invalid(message)) if message.contains("NDIlib_initialize"))
            );
        }
    }
}
