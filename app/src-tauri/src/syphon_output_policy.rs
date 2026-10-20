//! Platform-independent validation and the Syphon sender publication boundary.

const MAX_SYPHON_NAME_BYTES: usize = 255;
const MAX_SYPHON_DIMENSION: u32 = 8192;

pub(crate) fn validate_syphon_config(
    endpoint_name: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if endpoint_name.trim().is_empty() {
        return Err("Syphon server name must not be empty".to_string());
    }
    if endpoint_name.len() > MAX_SYPHON_NAME_BYTES {
        return Err("Syphon server name must not exceed 255 UTF-8 bytes".to_string());
    }
    if endpoint_name.chars().any(char::is_control) {
        return Err("Syphon server name must not contain control characters".to_string());
    }
    if width == 0 || height == 0 || width > MAX_SYPHON_DIMENSION || height > MAX_SYPHON_DIMENSION {
        return Err("Syphon output dimensions must each be between 1 and 8192 pixels".to_string());
    }
    Ok(())
}

#[cfg(any(all(feature = "syphon", target_os = "macos"), test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyphonOutputPhase {
    Constructing,
    Ready,
    Published,
    Stopping,
    Stopped,
    Failed,
}

#[cfg(any(all(feature = "syphon", target_os = "macos"), test))]
pub(crate) struct SyphonOutputLifecycle {
    phase: SyphonOutputPhase,
}

#[cfg(any(all(feature = "syphon", target_os = "macos"), test))]
impl SyphonOutputLifecycle {
    pub(crate) fn new() -> Self {
        Self {
            phase: SyphonOutputPhase::Constructing,
        }
    }

    /// Called only after the native constructor has reported success.
    pub(crate) fn ready(&mut self) -> Result<(), String> {
        if self.phase != SyphonOutputPhase::Constructing {
            return Err("Syphon readiness arrived after startup was invalidated".to_string());
        }
        self.phase = SyphonOutputPhase::Ready;
        Ok(())
    }

    /// Called only after committing the creation lease. The worker still waits
    /// for the engine's READY activation barrier before rendering or sending.
    pub(crate) fn publish(&mut self) -> Result<(), String> {
        if self.phase != SyphonOutputPhase::Ready {
            return Err("Syphon publication requires a ready sender".to_string());
        }
        self.phase = SyphonOutputPhase::Published;
        Ok(())
    }

    pub(crate) fn stopping(&mut self) {
        if !matches!(
            self.phase,
            SyphonOutputPhase::Stopped | SyphonOutputPhase::Failed
        ) {
            self.phase = SyphonOutputPhase::Stopping;
        }
    }

    /// Resource retirement is acknowledged only after the owner thread joins.
    pub(crate) fn stopped(&mut self) -> Result<(), String> {
        if self.phase != SyphonOutputPhase::Stopping {
            return Err("Syphon teardown acknowledgement requires a stopping sender".to_string());
        }
        self.phase = SyphonOutputPhase::Stopped;
        Ok(())
    }

    pub(crate) fn failed(&mut self) {
        self.phase = SyphonOutputPhase::Failed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syphon_config_accepts_unicode_and_valid_dimensions() {
        assert!(validate_syphon_config("Syndocal 映像", 1920, 1080).is_ok());
        assert!(validate_syphon_config("Syndocal", 1, 8192).is_ok());
        assert!(validate_syphon_config(&"a".repeat(255), 8192, 8192).is_ok());
    }

    #[test]
    fn syphon_config_rejects_ambiguous_names_and_oversized_frames() {
        for name in [
            "",
            " \t",
            "Syndocal\0other",
            "Syndocal\nother",
            "Syndocal\u{7f}",
        ] {
            assert!(
                validate_syphon_config(name, 1920, 1080).is_err(),
                "{name:?}"
            );
        }
        assert!(validate_syphon_config(&"映".repeat(86), 1920, 1080).is_err());
        for (width, height) in [
            (0, 1080),
            (1920, 0),
            (8193, 1),
            (1, 8193),
            (u32::MAX, u32::MAX),
        ] {
            assert!(validate_syphon_config("Syndocal", width, height).is_err());
        }
    }

    #[test]
    fn syphon_publication_requires_native_readiness_and_is_single_use() {
        let mut state = SyphonOutputLifecycle::new();
        assert!(state.publish().is_err());
        assert_eq!(state.phase, SyphonOutputPhase::Constructing);
        state.ready().unwrap();
        assert!(state.ready().is_err());
        state.publish().unwrap();
        assert!(state.publish().is_err());
        state.stopping();
        state.stopping();
        assert!(state.publish().is_err());
        state.stopped().unwrap();
        assert_eq!(state.phase, SyphonOutputPhase::Stopped);
        assert!(state.ready().is_err());
    }

    #[test]
    fn syphon_cancelled_or_failed_startup_cannot_publish_late() {
        let mut cancelled = SyphonOutputLifecycle::new();
        assert!(cancelled.stopped().is_err());
        cancelled.stopping();
        assert!(cancelled.ready().is_err());
        assert!(cancelled.publish().is_err());
        cancelled.stopped().unwrap();
        let mut failed = SyphonOutputLifecycle::new();
        failed.ready().unwrap();
        failed.failed();
        failed.stopping();
        assert_eq!(failed.phase, SyphonOutputPhase::Failed);
        assert!(failed.publish().is_err());
        assert!(failed.ready().is_err());
        assert!(failed.stopped().is_err());
    }
}
