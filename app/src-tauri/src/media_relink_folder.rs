//! Read-only folder listing used by the Media Health folder relink review.
//!
//! The scan only reports file names and sizes. It never opens, hashes or
//! mutates a file, never follows symbolic links or junctions, and is bounded in
//! depth, folders, entries and wall-clock time (the command runs on the same
//! thread as the native folder dialog, so it must stay short). Matching against
//! Media Library assets stays in the renderer, and every accepted match is still
//! validated and committed through the ordinary Media Asset relink operation, so
//! asset identity rules apply.
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Bounds of one scan. Reaching any of them ends the scan early and marks the
/// result `truncated` so the operator is told matches may be missing.
#[derive(Debug, Clone, Copy)]
pub(super) struct MediaRelinkScanLimits {
    /// Folder levels below the chosen folder that are searched.
    pub(super) max_depth: usize,
    /// Folders read.
    pub(super) max_directories: usize,
    /// Media files reported.
    pub(super) max_files: usize,
    /// Directory entries of any kind examined.
    pub(super) max_entries: usize,
    /// Wall-clock budget for the whole scan.
    pub(super) time_budget: Duration,
}

pub(super) const MEDIA_RELINK_SCAN_LIMITS: MediaRelinkScanLimits = MediaRelinkScanLimits {
    max_depth: 4,
    max_directories: 2_000,
    max_files: 20_000,
    max_entries: 200_000,
    time_budget: Duration::from_secs(3),
};

/// How many entries are processed between wall-clock checks inside one folder.
const DEADLINE_CHECK_INTERVAL: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct MediaRelinkFolderFile {
    pub(super) path: String,
    pub(super) file_name: String,
    pub(super) byte_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct MediaRelinkFolderScan {
    pub(super) directory: String,
    pub(super) files: Vec<MediaRelinkFolderFile>,
    /// True when the listing may be incomplete: a bound was reached, a folder
    /// was deeper than the depth bound, or a subfolder could not be read.
    pub(super) truncated: bool,
}

fn has_media_extension(name: &str, extensions: &[&str]) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(extension))
        })
}

/// Lists media files below `directory` with the production bounds.
pub(super) fn scan_media_relink_folder(
    directory: &Path,
    extensions: &[&str],
) -> Result<MediaRelinkFolderScan, String> {
    scan_media_relink_folder_with_limits(directory, extensions, MEDIA_RELINK_SCAN_LIMITS)
}

/// Lists media files below `directory` (breadth first, names sorted per folder
/// so the result is deterministic). Only the chosen folder itself must be
/// readable; an unreadable subfolder is skipped and marks the scan truncated.
pub(super) fn scan_media_relink_folder_with_limits(
    directory: &Path,
    extensions: &[&str],
    limits: MediaRelinkScanLimits,
) -> Result<MediaRelinkFolderScan, String> {
    let Some(directory_text) = directory.to_str() else {
        return Err("The chosen folder path is not valid UTF-8".to_string());
    };
    let root_metadata = fs::metadata(directory)
        .map_err(|error| format!("Cannot read the folder '{}': {error}", directory.display()))?;
    if !root_metadata.is_dir() {
        return Err(format!("'{}' is not a folder", directory.display()));
    }
    let deadline = Instant::now() + limits.time_budget;
    let mut files = Vec::new();
    let mut truncated = false;
    let mut visited_directories = 0usize;
    let mut examined_entries = 0usize;
    let mut pending: VecDeque<(PathBuf, usize)> = VecDeque::new();
    pending.push_back((directory.to_path_buf(), 0));
    'scan: while let Some((current, depth)) = pending.pop_front() {
        if visited_directories >= limits.max_directories || Instant::now() >= deadline {
            truncated = true;
            break;
        }
        visited_directories += 1;
        let reader = match fs::read_dir(&current) {
            Ok(reader) => reader,
            Err(error) if depth == 0 => {
                return Err(format!(
                    "Cannot read the folder '{}': {error}",
                    current.display()
                ));
            }
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        let remaining_entries = limits.max_entries.saturating_sub(examined_entries);
        let mut entries = Vec::new();
        let mut entry_bound_reached = false;
        for entry in reader.flatten() {
            if entries.len() >= remaining_entries {
                entry_bound_reached = true;
                break;
            }
            entries.push(entry);
        }
        examined_entries += entries.len();
        entries.sort_by_key(|entry| entry.file_name());
        for (index, entry) in entries.into_iter().enumerate() {
            if index % DEADLINE_CHECK_INTERVAL == DEADLINE_CHECK_INTERVAL - 1
                && Instant::now() >= deadline
            {
                truncated = true;
                break 'scan;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if depth < limits.max_depth {
                    pending.push_back((entry.path(), depth + 1));
                } else {
                    truncated = true;
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !has_media_extension(&file_name, extensions) {
                continue;
            }
            let path = entry.path();
            let Some(path) = path.to_str().map(str::to_string) else {
                continue;
            };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if files.len() >= limits.max_files {
                truncated = true;
                break 'scan;
            }
            files.push(MediaRelinkFolderFile {
                path,
                file_name,
                byte_size: metadata.len(),
            });
        }
        if entry_bound_reached {
            truncated = true;
            break;
        }
    }
    Ok(MediaRelinkFolderScan {
        directory: directory_text.to_string(),
        files,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct FixtureDirectory(PathBuf);

    impl FixtureDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "syndocal-media-relink-scan-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for FixtureDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const EXTENSIONS: &[&str] = &["mp4", "png"];

    fn limits() -> MediaRelinkScanLimits {
        MEDIA_RELINK_SCAN_LIMITS
    }

    #[test]
    fn lists_only_media_files_with_names_and_sizes_in_a_stable_order() {
        let fixture = FixtureDirectory::new();
        fixture.write("b.mp4", b"12345");
        fixture.write("a.PNG", b"123");
        fixture.write("notes.txt", b"ignored");
        fixture.write("sub/c.mp4", b"1234567");
        let scan = scan_media_relink_folder(&fixture.0, EXTENSIONS).unwrap();
        assert!(!scan.truncated);
        let listed = scan
            .files
            .iter()
            .map(|file| (file.file_name.as_str(), file.byte_size))
            .collect::<Vec<_>>();
        assert_eq!(listed, vec![("a.PNG", 3), ("b.mp4", 5), ("c.mp4", 7)]);
        assert!(scan
            .files
            .iter()
            .all(|file| Path::new(&file.path).is_absolute() && Path::new(&file.path).is_file()));
    }

    #[test]
    fn does_not_descend_below_the_depth_bound_and_says_so() {
        let fixture = FixtureDirectory::new();
        let max_depth = limits().max_depth;
        let mut nested = String::new();
        for level in 0..=max_depth + 1 {
            nested.push_str(&format!("d{level}/"));
            fixture.write(&format!("{nested}level{level}.mp4"), b"x");
        }
        fixture.write("top.mp4", b"x");
        let scan = scan_media_relink_folder(&fixture.0, EXTENSIONS).unwrap();
        let names = scan
            .files
            .iter()
            .map(|file| file.file_name.clone())
            .collect::<Vec<_>>();
        assert!(names.contains(&"top.mp4".to_string()));
        assert!(names.contains(&format!("level{}.mp4", max_depth - 1)));
        assert!(!names.contains(&format!("level{}.mp4", max_depth + 1)));
        assert!(scan.truncated, "unsearched deeper folders are reported");
    }

    #[test]
    fn a_complete_scan_within_every_bound_is_not_truncated() {
        let fixture = FixtureDirectory::new();
        fixture.write("a.mp4", b"x");
        fixture.write("one/b.mp4", b"x");
        let scan = scan_media_relink_folder(&fixture.0, EXTENSIONS).unwrap();
        assert_eq!(scan.files.len(), 2);
        assert!(!scan.truncated);
    }

    #[test]
    fn each_bound_ends_the_scan_early_and_marks_it_truncated() {
        let fixture = FixtureDirectory::new();
        for index in 0..6 {
            fixture.write(&format!("clip{index}.mp4"), b"x");
        }
        fixture.write("one/inner.mp4", b"x");
        fixture.write("two/inner.mp4", b"x");

        let file_bound = scan_media_relink_folder_with_limits(
            &fixture.0,
            EXTENSIONS,
            MediaRelinkScanLimits {
                max_files: 3,
                ..limits()
            },
        )
        .unwrap();
        assert_eq!(file_bound.files.len(), 3);
        assert!(file_bound.truncated);

        let directory_bound = scan_media_relink_folder_with_limits(
            &fixture.0,
            EXTENSIONS,
            MediaRelinkScanLimits {
                max_directories: 1,
                ..limits()
            },
        )
        .unwrap();
        assert_eq!(
            directory_bound.files.len(),
            6,
            "only the chosen folder was read"
        );
        assert!(directory_bound.truncated);

        let entry_bound = scan_media_relink_folder_with_limits(
            &fixture.0,
            EXTENSIONS,
            MediaRelinkScanLimits {
                max_entries: 2,
                ..limits()
            },
        )
        .unwrap();
        assert!(entry_bound.files.len() <= 2);
        assert!(entry_bound.truncated);

        let no_time = scan_media_relink_folder_with_limits(
            &fixture.0,
            EXTENSIONS,
            MediaRelinkScanLimits {
                time_budget: Duration::ZERO,
                ..limits()
            },
        )
        .unwrap();
        assert!(no_time.files.is_empty());
        assert!(no_time.truncated);
    }

    #[test]
    fn rejects_a_missing_folder_and_a_file_path() {
        let fixture = FixtureDirectory::new();
        fixture.write("clip.mp4", b"x");
        let missing = scan_media_relink_folder(&fixture.0.join("absent"), EXTENSIONS);
        assert!(missing.unwrap_err().contains("Cannot read the folder"));
        let not_folder = scan_media_relink_folder(&fixture.0.join("clip.mp4"), EXTENSIONS);
        assert!(not_folder.unwrap_err().contains("is not a folder"));
    }

    #[cfg(windows)]
    #[test]
    fn a_non_utf8_root_is_rejected_before_any_scan() {
        use std::os::windows::ffi::OsStringExt;
        let invalid = std::ffi::OsString::from_wide(&[0x0043, 0x003a, 0x005c, 0xd800]);
        let error = scan_media_relink_folder(Path::new(&invalid), EXTENSIONS).unwrap_err();
        assert!(error.contains("not valid UTF-8"));
    }

    #[test]
    fn extension_matching_is_case_insensitive_and_requires_an_extension() {
        assert!(has_media_extension("Clip.MP4", EXTENSIONS));
        assert!(has_media_extension("a.b.png", EXTENSIONS));
        assert!(!has_media_extension("mp4", EXTENSIONS));
        assert!(!has_media_extension("clip.mp4.txt", EXTENSIONS));
        assert!(!has_media_extension("", EXTENSIONS));
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_not_followed_or_listed() {
        let fixture = FixtureDirectory::new();
        fixture.write("real/clip.mp4", b"x");
        std::os::unix::fs::symlink(fixture.0.join("real"), fixture.0.join("link")).unwrap();
        std::os::unix::fs::symlink(fixture.0.join("real/clip.mp4"), fixture.0.join("alias.mp4"))
            .unwrap();
        let scan = scan_media_relink_folder(&fixture.0, EXTENSIONS).unwrap();
        assert_eq!(scan.files.len(), 1);
        assert!(scan.files[0].path.ends_with("clip.mp4"));
    }

    /// A directory junction is a Windows reparse point that is not a symlink in
    /// name only: it must be skipped, or the scan could leave the chosen folder.
    #[cfg(windows)]
    #[test]
    fn directory_junctions_are_not_followed() {
        let fixture = FixtureDirectory::new();
        fixture.write("real/clip.mp4", b"x");
        let link = fixture.0.join("junction");
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(fixture.0.join("real"))
            .output();
        let Ok(output) = status else {
            return;
        };
        if !output.status.success() {
            // Junction creation can be unavailable (restricted environment);
            // the Windows symlink path is then not exercisable here.
            return;
        }
        println!("junction created; verifying it is not followed");
        let scan = scan_media_relink_folder(&fixture.0, EXTENSIONS).unwrap();
        assert_eq!(
            scan.files.len(),
            1,
            "the junction target is listed once, not twice"
        );
        assert!(scan.files[0].path.contains("real"));
    }
}
