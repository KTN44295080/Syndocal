//! Per-user, opt-in bundled integrations; independent of show/project schemas.
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct ExtensionStatus {
    pub configured: bool,
    pub dj_link_enabled: bool,
    pub installer_managed: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Extensions {
    path: Option<PathBuf>,
    status: ExtensionStatus,
}

pub fn decode(text: &str) -> Result<bool, String> {
    if text.len() > 4096 {
        return Err("Extension settings exceed their size limit".into());
    }
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != 3 || lines[0] != "[SyndocalExtensions]" || lines[1] != "Schema=1" {
        return Err("Unsupported extension settings; preserve the file and install a compatible Syndocal version".into());
    }
    match lines[2] {
        "DJLink=0" => Ok(false),
        "DJLink=1" => Ok(true),
        _ => Err("Invalid DJ Link extension choice".into()),
    }
}

impl Extensions {
    pub fn initialize(&mut self, directory: &Path) {
        #[cfg(target_os = "macos")]
        let installed = Some(Path::new(
            "/Library/Application Support/jp.seraf.ktn.syndocal/extensions.ini",
        ));
        #[cfg(not(target_os = "macos"))]
        let installed: Option<&Path> = None;
        self.initialize_with_installer(directory, installed);
    }
    fn initialize_with_installer(&mut self, directory: &Path, installed: Option<&Path>) {
        let installed = installed.filter(|path| fs::symlink_metadata(path).is_ok());
        if installed.is_some_and(|path| {
            path.ancestors().any(|ancestor| {
                fs::symlink_metadata(ancestor)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
            })
        }) {
            self.status = ExtensionStatus {
                error: Some("Redirected installer settings rejected".into()),
                ..Default::default()
            };
            return;
        }
        let path = installed
            .map(Path::to_path_buf)
            .unwrap_or_else(|| directory.join("extensions.ini"));
        self.path = Some(path.clone());
        if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 4096) {
            self.status = ExtensionStatus {
                error: Some("Extension settings exceed their size limit".into()),
                ..Default::default()
            };
            return;
        }
        self.status = match fs::read_to_string(&path) {
            Ok(text) => match decode(&text) {
                Ok(dj_link_enabled) => ExtensionStatus {
                    configured: true,
                    dj_link_enabled,
                    error: None,
                    installer_managed: installed.is_some(),
                },
                Err(error) => ExtensionStatus {
                    error: Some(error),
                    ..Default::default()
                },
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ExtensionStatus {
                configured: true,
                ..Default::default()
            },
            Err(error) => ExtensionStatus {
                error: Some(format!("Cannot read extension settings: {error}")),
                ..Default::default()
            },
        };
    }
    pub fn status(&self) -> ExtensionStatus {
        self.status.clone()
    }
    pub fn require_dj_link(&self) -> Result<(), String> {
        if let Some(error) = &self.status.error {
            return Err(error.clone());
        }
        if self.status.dj_link_enabled {
            Ok(())
        } else {
            Err("DJ Link extension is not enabled. Open Setup > I/O > Extensions.".into())
        }
    }
    pub fn set(&mut self, dj_link_enabled: bool) -> Result<ExtensionStatus, String> {
        if self.status.installer_managed {
            return Err("Change extensions using the Syndocal installer".into());
        }
        if let Some(error) = &self.status.error {
            return Err(error.clone());
        }
        let path = self
            .path
            .as_ref()
            .ok_or("Extension settings are not initialized")?;
        let parent = path
            .parent()
            .ok_or("Invalid extension settings directory")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let temporary = parent.join(format!("extensions-{}-{nonce}.pending", std::process::id()));
        let text = format!(
            "[SyndocalExtensions]\nSchema=1\nDJLink={}\n",
            u8::from(dj_link_enabled)
        );
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let result = file
            .write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| error.to_string());
        drop(file);
        if let Err(error) = result.and_then(|_| super::replace_file_atomically(&temporary, path)) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        self.status = ExtensionStatus {
            configured: true,
            dj_link_enabled,
            installer_managed: false,
            error: None,
        };
        Ok(self.status())
    }
    #[cfg(test)]
    pub fn test_enabled() -> Self {
        Self {
            path: None,
            status: ExtensionStatus {
                configured: true,
                dj_link_enabled: true,
                installer_managed: false,
                error: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_choice_requires_exact_supported_schema() {
        assert!(!decode("[SyndocalExtensions]\nSchema=1\nDJLink=0\n").unwrap());
        assert!(decode("[SyndocalExtensions]\nSchema=1\nDJLink=1\n").unwrap());
        for text in [
            "",
            "[SyndocalExtensions]\nSchema=2\nDJLink=1",
            "[SyndocalExtensions]\nSchema=1\nDJLink=1\nUnexpected=1",
            "[SyndocalExtensions]\nSchema=1\nDJLink=1\nDJLink=0",
            "[SyndocalExtensions]\nSchema=1\nDJLink=true",
        ] {
            assert!(decode(text).is_err());
        }
    }
    #[test]
    fn extension_settings_survive_restart_and_future_data_is_not_overwritten() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "syndocal-extension-test-{}-{nonce}",
            std::process::id()
        ));
        let mut manager = Extensions::default();
        manager.initialize(&directory);
        assert!(manager.status().configured);
        manager.set(true).unwrap();
        let mut restarted = Extensions::default();
        restarted.initialize(&directory);
        assert!(restarted.require_dj_link().is_ok());
        restarted.set(false).unwrap();
        manager.initialize(&directory);
        assert!(!manager.status().dj_link_enabled);
        let path = directory.join("extensions.ini");
        let future = "[SyndocalExtensions]\nSchema=2\nDJLink=1\n";
        fs::write(&path, future).unwrap();
        manager.initialize(&directory);
        assert!(manager.status().error.is_some());
        assert!(manager.set(false).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), future);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn fresh_core_rejects_dj_link_and_explicit_enabled_accepts() {
        assert!(Extensions::default().require_dj_link().is_err());
        assert!(Extensions::test_enabled().require_dj_link().is_ok());
    }

    #[test]
    fn installer_choice_overrides_old_user_settings_and_requires_installer_to_change() {
        let directory = std::env::temp_dir().join(format!(
            "syndocal-installed-extension-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let installed = directory.join("installed.ini");
        fs::write(
            directory.join("extensions.ini"),
            "[SyndocalExtensions]\nSchema=1\nDJLink=1\n",
        )
        .unwrap();
        fs::write(&installed, "[SyndocalExtensions]\nSchema=1\nDJLink=0\n").unwrap();
        let mut manager = Extensions::default();
        manager.initialize_with_installer(&directory, Some(&installed));
        assert!(manager.status().installer_managed);
        assert!(!manager.status().dj_link_enabled);
        assert!(manager.set(true).is_err());
        let future = "[SyndocalExtensions]\nSchema=2\nDJLink=1\n";
        fs::write(&installed, future).unwrap();
        manager.initialize_with_installer(&directory, Some(&installed));
        assert!(manager.status().error.is_some());
        assert!(manager.set(false).is_err());
        assert_eq!(fs::read_to_string(&installed).unwrap(), future);
        fs::remove_file(installed).unwrap();
        fs::remove_file(directory.join("extensions.ini")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
