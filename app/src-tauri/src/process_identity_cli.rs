//! Read-only macOS process inspection before Tauri/Engine initialization.
use std::{ffi::OsString, path::PathBuf};

const FLAG: &str = "--syndocal-process-path-v1";

pub(crate) fn requested(args: impl Iterator<Item = OsString>) -> bool {
    args.into_iter().any(|arg| arg == FLAG)
}

fn parse_pid(args: impl Iterator<Item = OsString>) -> Result<i32, String> {
    let args = args.collect::<Vec<_>>();
    if args.len() != 2 || args[0] != FLAG {
        return Err("Expected process-path flag and one positive PID".into());
    }
    let value = args[1].to_str().ok_or("PID must be ASCII decimal")?;
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("PID must be canonical positive decimal".into());
    }
    value
        .parse::<i32>()
        .map_err(|_| "PID is out of range".into())
}

pub(crate) fn run(args: impl Iterator<Item = OsString>) -> Result<(), String> {
    let pid = parse_pid(args)?;
    let executable = process_path(pid)?;
    let executable = executable.to_str().ok_or("Process path is not UTF-8")?;
    println!(
        "{}",
        serde_json::json!({"schemaVersion": 1, "processId": pid, "executablePath": executable})
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn process_path(pid: i32) -> Result<PathBuf, String> {
    use std::{ffi::c_void, os::unix::ffi::OsStringExt};
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut c_void, size: u32) -> i32;
    }
    // Apple's PROC_PIDPATHINFO_MAXSIZE is 4 * MAXPATHLEN (4096).
    let mut buffer = [0_u8; 4096];
    let length = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 || length as usize >= buffer.len() {
        return Err(format!(
            "Cannot inspect process path: {}",
            std::io::Error::last_os_error()
        ));
    }
    let length = length as usize;
    if buffer[length] != 0 || buffer[..length].contains(&0) {
        return Err("Process path was not a complete terminated string".into());
    }
    let path = PathBuf::from(OsString::from_vec(buffer[..length].to_vec()));
    if !path.is_absolute() {
        return Err("Process path is not absolute".into());
    }
    Ok(path)
}

#[cfg(not(target_os = "macos"))]
fn process_path(_pid: i32) -> Result<PathBuf, String> {
    Err("Native process-path inspection is available only on macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> impl Iterator<Item = OsString> {
        values
            .iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .into_iter()
    }
    #[test]
    fn malformed_inspection_never_becomes_normal_startup() {
        for values in [
            vec![FLAG],
            vec![FLAG, "0"],
            vec![FLAG, "01"],
            vec![FLAG, "-1"],
            vec![FLAG, "1;echo"],
            vec![FLAG, "2147483648"],
            vec![FLAG, "1", "extra"],
            vec!["other", FLAG, "1"],
        ] {
            assert!(requested(args(&values)));
            assert!(parse_pid(args(&values)).is_err());
        }
        assert!(!requested(args(&["project.sdc"])));
        assert_eq!(parse_pid(args(&[FLAG, "123"])).unwrap(), 123);
    }
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_platform_rejects_without_startup() {
        assert!(run(args(&[FLAG, "1"]))
            .unwrap_err()
            .contains("only on macOS"));
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_kernel_path_matches_current_executable() {
        let found = process_path(std::process::id() as i32).unwrap();
        assert_eq!(
            std::fs::canonicalize(found).unwrap(),
            std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap()
        );
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_reaped_process_has_no_inspectable_path() {
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id() as i32;
        assert!(child.wait().unwrap().success());
        assert!(process_path(pid).is_err());
    }
}
