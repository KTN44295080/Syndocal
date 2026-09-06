//! A recording becomes the selected output only after the encoder has finished.
//! Staging files are deliberately recognizable after an interrupted process.
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_RESERVATION: AtomicU64 = AtomicU64::new(0);

pub(super) fn drain_encoder_stderr(mut stderr: impl Read) -> std::io::Result<Vec<u8>> {
    let mut tail = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => return Ok(tail),
            Ok(count) => {
                tail.extend_from_slice(&buffer[..count]);
                if tail.len() > 65_536 {
                    tail.drain(..tail.len() - 65_536);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FileIdentity(u64, u64);

#[derive(Debug, PartialEq, Eq)]
enum TargetState {
    Absent,
    Present {
        identity: FileIdentity,
        len: u64,
        modified: SystemTime,
    },
}

fn target_state(path: &Path) -> Result<TargetState, String> {
    reject_special_path(path, true)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TargetState::Absent)
        }
        Err(error) => {
            return Err(format!(
                "Cannot reserve recording target {}: {error}",
                path.display()
            ))
        }
    };
    let metadata = file.metadata().map_err(|error| {
        format!(
            "Cannot inspect recording target {}: {error}",
            path.display()
        )
    })?;
    Ok(TargetState::Present {
        identity: identity(&file)?,
        len: metadata.len(),
        modified: metadata.modified().map_err(|error| {
            format!(
                "Cannot read target modification time {}: {error}",
                path.display()
            )
        })?,
    })
}

/// Publish an initially absent destination without overwriting a concurrent
/// creator. The bool reports whether a staging link still needs removal.
fn publish_new_file(staging: &Path, target: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH},
        };
        let staging_wide = staging
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let target_wide = target
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe { MoveFileExW(PCWSTR(staging_wide.as_ptr()), PCWSTR(target_wide.as_ptr()), MOVEFILE_WRITE_THROUGH) }
            .map_err(|error| format!("Cannot publish new recording {}; a concurrent destination will not be overwritten: {error}", target.display()))?;
        Ok(false)
    }
    #[cfg(unix)]
    {
        // A same-filesystem link atomically refuses any existing destination.
        fs::hard_link(staging, target).map_err(|error| format!("Cannot publish new recording {}; a concurrent destination will not be overwritten: {error}", target.display()))?;
        Ok(true)
    }
}

fn identity(file: &File) -> Result<FileIdentity, String> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
        };
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
            .map_err(|error| format!("Cannot identify recording staging file: {error}"))?;
        if info.nNumberOfLinks != 1 {
            return Err("Recording staging file must have exactly one link".into());
        }
        Ok(FileIdentity(
            info.dwVolumeSerialNumber as u64,
            ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        ))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        if metadata.nlink() != 1 {
            return Err("Recording staging file must have exactly one link".into());
        }
        Ok(FileIdentity(metadata.dev(), metadata.ino()))
    }
}

fn reject_special_path(path: &Path, allow_missing: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            #[cfg(windows)]
            let reparse = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let reparse = false;
            if !metadata.is_file() || metadata.file_type().is_symlink() || reparse {
                return Err(format!(
                    "Recording path must be a regular file, not a link or directory: {}",
                    path.display()
                ));
            }
            Ok(())
        }
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Cannot inspect recording path {}: {error}",
            path.display()
        )),
    }
}

pub(super) struct RecordingArtifact {
    target: PathBuf,
    target_state: TargetState,
    staging: PathBuf,
    file: Option<File>,
    identity: FileIdentity,
    owned: bool,
}

impl RecordingArtifact {
    pub(super) fn reserve(target: &Path) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Cannot reserve recording path: {error}"))?
            .as_nanos();
        let serial = NEXT_RESERVATION.fetch_add(1, Ordering::Relaxed);
        Self::reserve_named(
            target,
            &format!(
                ".syndocal-recording-{}-{nonce}-{serial}.partial.mp4",
                std::process::id()
            ),
        )
    }

    fn reserve_named(target: &Path, staging_name: &str) -> Result<Self, String> {
        let name = target
            .file_name()
            .ok_or("Recording target has no file name")?;
        let parent = target
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent).map_err(|error| {
            format!(
                "Cannot resolve recording directory {}: {error}",
                parent.display()
            )
        })?;
        let target = parent.join(name);
        let target_state = target_state(&target)?;
        let staging = parent.join(staging_name);
        if staging == target {
            return Err("Recording staging path must differ from its target".into());
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // FFmpeg may write the reservation, but nobody may rename/delete it
            // while encoding. Publication closes this handle before MoveFileExW.
            options.share_mode(3);
        }
        let file = options.open(&staging).map_err(|error| {
            format!(
                "Cannot reserve recording staging file {}: {error}",
                staging.display()
            )
        })?;
        let identity = identity(&file).map_err(|error| {
            format!(
                "{error}; inspect reserved partial {} manually",
                staging.display()
            )
        })?;
        Ok(Self {
            target,
            target_state,
            staging,
            file: Some(file),
            identity,
            owned: true,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.staging
    }

    fn verify_owned(&self) -> Result<(), String> {
        reject_special_path(&self.staging, false)?;
        let file = File::open(&self.staging).map_err(|error| {
            format!("Cannot inspect partial {}: {error}", self.staging.display())
        })?;
        if identity(&file)? != self.identity {
            return Err(format!(
                "Recording partial ownership changed; refusing to publish or remove {}",
                self.staging.display()
            ));
        }
        Ok(())
    }

    pub(super) fn publish(&mut self, frames_written: u64) -> Result<(), String> {
        if frames_written == 0 {
            return Err(
                "Recording produced no complete video frames; previous output was preserved".into(),
            );
        }
        self.verify_owned()?;
        let file = self
            .file
            .as_ref()
            .ok_or("Recording staging handle is unavailable")?;
        if file.metadata().map_err(|error| error.to_string())?.len() == 0 {
            return Err("FFmpeg produced an empty recording; previous output was preserved".into());
        }
        file.sync_all().map_err(|error| {
            format!(
                "Cannot sync recording partial {}: {error}",
                self.staging.display()
            )
        })?;
        if target_state(&self.target)? != self.target_state {
            return Err(format!("Recording target changed during capture; refusing to overwrite {}. Choose another destination and record again", self.target.display()));
        }
        self.file.take();
        self.verify_owned()?;
        let staging_link_remains = match self.target_state {
            TargetState::Absent => publish_new_file(&self.staging, &self.target)?,
            TargetState::Present { .. } => {
                // This existing-target check + replace is not CAS: even an
                // ordinary writer can race after the final comparison. It
                // requires exclusive destination use during final publication.
                super::replace_file_atomically(&self.staging, &self.target)?;
                false
            }
        };
        self.owned = false;
        if staging_link_remains {
            fs::remove_file(&self.staging).map_err(|error| format!("Recording was published to {}, but its partial link {} could not be removed: {error}; remove that partial manually", self.target.display(), self.staging.display()))?;
        }
        Ok(())
    }

    pub(super) fn discard(&mut self) -> Result<(), String> {
        if !self.owned {
            return Ok(());
        }
        self.verify_owned()?;
        self.file.take();
        self.verify_owned()?;
        fs::remove_file(&self.staging).map_err(|error| format!("Cannot remove failed recording partial {}: {error}; remove it manually after the encoder exits", self.staging.display()))?;
        self.owned = false;
        Ok(())
    }
}

impl Drop for RecordingArtifact {
    fn drop(&mut self) {
        if let Err(error) = self.discard() {
            eprintln!("Recording cleanup failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "syndocal-recording-test-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_RESERVATION.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn target(&self) -> PathBuf {
            self.0.join("output.mp4")
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn failed_or_dropped_recording_preserves_previous_output() {
        let dir = TempDir::new();
        fs::write(dir.target(), b"previous complete movie").unwrap();
        let partial;
        {
            let artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
            partial = artifact.path().to_path_buf();
            fs::write(&partial, b"incomplete encoder output").unwrap();
            assert_eq!(fs::read(dir.target()).unwrap(), b"previous complete movie");
        }
        assert!(!partial.exists());
        assert_eq!(fs::read(dir.target()).unwrap(), b"previous complete movie");
    }

    #[test]
    fn complete_output_replaces_target_only_on_publish() {
        let dir = TempDir::new();
        fs::write(dir.target(), b"old").unwrap();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        assert_eq!(
            artifact.path().parent().unwrap(),
            fs::canonicalize(&dir.0).unwrap()
        );
        assert_eq!(artifact.path().extension().unwrap(), "mp4");
        fs::write(artifact.path(), b"complete encoded movie").unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"old");
        artifact.publish(1).unwrap();
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"complete encoded movie");
        assert!(!artifact.path().exists());
    }

    #[test]
    fn collision_and_invalid_target_never_overwrite_existing_paths() {
        let dir = TempDir::new();
        let collision = dir.0.join("collision.partial.mp4");
        fs::write(&collision, b"not ours").unwrap();
        assert!(RecordingArtifact::reserve_named(&dir.target(), "collision.partial.mp4").is_err());
        assert_eq!(fs::read(collision).unwrap(), b"not ours");
        fs::create_dir(dir.target()).unwrap();
        assert!(RecordingArtifact::reserve(&dir.target()).is_err());
    }

    #[test]
    fn zero_frames_and_empty_bytes_cannot_publish() {
        let dir = TempDir::new();
        fs::write(dir.target(), b"old").unwrap();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        assert!(artifact.publish(1).unwrap_err().contains("empty"));
        fs::write(artifact.path(), b"container header only").unwrap();
        assert!(artifact.publish(0).unwrap_err().contains("no complete"));
        assert_eq!(fs::read(dir.target()).unwrap(), b"old");
    }

    #[test]
    fn changed_target_type_blocks_publication_without_removing_it() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"new").unwrap();
        fs::create_dir(dir.target()).unwrap();
        fs::write(dir.target().join("protected"), b"keep").unwrap();
        assert!(artifact.publish(1).is_err());
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target().join("protected")).unwrap(), b"keep");
    }

    #[test]
    fn newly_created_target_is_not_overwritten() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"recording").unwrap();
        fs::write(dir.target(), b"another writer").unwrap();
        assert!(artifact.publish(1).unwrap_err().contains("target changed"));
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"another writer");
    }

    #[test]
    fn absent_target_publication_atomically_rejects_a_late_creator() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"recording").unwrap();
        assert_eq!(target_state(&dir.target()).unwrap(), TargetState::Absent);
        artifact.file.take();
        artifact.verify_owned().unwrap();
        // Simulate creation after publication's last destination comparison.
        fs::write(dir.target(), b"late concurrent writer").unwrap();
        assert!(publish_new_file(artifact.path(), &dir.target()).is_err());
        assert_eq!(fs::read(dir.target()).unwrap(), b"late concurrent writer");
        assert_eq!(fs::read(artifact.path()).unwrap(), b"recording");
        artifact.discard().unwrap();
    }

    #[test]
    fn absent_target_publication_succeeds_and_removes_owned_partial() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"complete new recording").unwrap();
        artifact.publish(1).unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"complete new recording");
        assert!(!artifact.path().exists());
    }

    #[test]
    fn replaced_existing_target_is_not_overwritten() {
        let dir = TempDir::new();
        fs::write(dir.target(), b"old").unwrap();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"recording").unwrap();
        fs::rename(dir.target(), dir.0.join("old.mp4")).unwrap();
        fs::write(dir.target(), b"new").unwrap();
        assert!(artifact.publish(1).unwrap_err().contains("target changed"));
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"new");
    }

    #[test]
    fn modified_existing_target_is_not_overwritten() {
        let dir = TempDir::new();
        fs::write(dir.target(), b"old").unwrap();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"recording").unwrap();
        fs::write(dir.target(), b"changed in place").unwrap();
        assert!(artifact.publish(1).unwrap_err().contains("target changed"));
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"changed in place");
    }

    #[test]
    fn nonowner_partial_is_not_published_or_removed() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        artifact.file.take();
        let moved = dir.0.join("original.partial.mp4");
        fs::rename(artifact.path(), &moved).unwrap();
        fs::write(artifact.path(), b"not ours").unwrap();
        assert!(artifact
            .publish(1)
            .unwrap_err()
            .contains("ownership changed"));
        assert!(artifact
            .discard()
            .unwrap_err()
            .contains("ownership changed"));
        let replacement = artifact.path().to_path_buf();
        drop(artifact);
        assert_eq!(fs::read(replacement).unwrap(), b"not ours");
        assert!(moved.exists());
    }

    #[cfg(windows)]
    #[test]
    fn locked_target_publish_failure_preserves_previous_output() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new();
        fs::write(dir.target(), b"previous").unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(dir.target())
            .unwrap();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        fs::write(artifact.path(), b"new").unwrap();
        assert!(artifact
            .publish(1)
            .unwrap_err()
            .contains("atomically replace"));
        artifact.discard().unwrap();
        assert_eq!(fs::read(dir.target()).unwrap(), b"previous");
        drop(locked);
    }

    #[test]
    fn hard_linked_partial_is_never_removed_or_published() {
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        artifact.file.take();
        let other = dir.0.join("other.mp4");
        fs::hard_link(artifact.path(), &other).unwrap();
        assert!(artifact.discard().is_err());
        let partial = artifact.path().to_path_buf();
        drop(artifact);
        assert!(partial.exists());
        assert!(other.exists());
    }

    #[test]
    fn diagnostic_tail_is_bounded_and_keeps_latest_bytes() {
        let mut bytes = vec![b'a'; 262_144];
        bytes.extend_from_slice(b"last encoder error");
        let result = drain_encoder_stderr(bytes.as_slice()).unwrap();
        assert_eq!(result.len(), 65_536);
        assert!(result.ends_with(b"last encoder error"));
    }

    #[cfg(windows)]
    #[test]
    fn cleanup_failure_reports_the_recoverable_partial() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new();
        let mut artifact = RecordingArtifact::reserve(&dir.target()).unwrap();
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(artifact.path())
            .unwrap();
        let error = artifact.discard().unwrap_err();
        assert!(error.contains("remove it manually"));
        assert!(error.contains(&artifact.path().display().to_string()));
        assert!(artifact.path().exists());
        drop(lock);
        artifact.discard().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn child_stderr_larger_than_pipe_capacity_is_drained_and_reaped() {
        use std::process::{Command, Stdio};
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Error.Write(('x' * 262144) + 'encoder-tail')",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let reader = std::thread::spawn(move || drain_encoder_stderr(stderr));
        assert!(child.wait().unwrap().success());
        let tail = reader.join().unwrap().unwrap();
        assert_eq!(tail.len(), 65_536);
        assert!(tail.ends_with(b"encoder-tail"));
    }
}
