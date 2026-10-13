//! MSI-only action library. Settings policy is shared with the app; no app startup.
#![deny(warnings)]
#[path = "../../../app/src-tauri/src/extensions_config.rs"]
mod settings;
use std::{fs, io::Write, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};

#[link(name = "msi")]
extern "system" {
    fn MsiGetPropertyW(handle: u32, name: *const u16, value: *mut u16, length: *mut u32) -> u32;
    fn MsiSetPropertyW(handle: u32, name: *const u16, value: *const u16) -> u32;
    fn MsiCreateRecord(fields: u32) -> u32;
    fn MsiRecordSetStringW(record: u32, field: u32, value: *const u16) -> u32;
    fn MsiProcessMessage(handle: u32, kind: u32, record: u32) -> i32;
    fn MsiCloseHandle(handle: u32) -> u32;
}
#[repr(C)]
struct Guid { a: u32, b: u16, c: u16, d: [u8; 8] }
#[link(name = "shell32")]
extern "system" { fn SHGetKnownFolderPath(id: *const Guid, flags: u32, token: isize, result: *mut *mut u16) -> i32; }
#[link(name = "ole32")]
extern "system" { fn CoTaskMemFree(memory: *mut std::ffi::c_void); }
#[link(name = "kernel32")]
extern "system" { fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32; }

fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> { value.as_ref().encode_wide().chain(Some(0)).collect() }
fn property(handle: u32, name: &str) -> Result<String, String> {
    let mut size = 0;
    let mut empty = 0u16;
    let result = unsafe { MsiGetPropertyW(handle, wide(name).as_ptr(), &mut empty, &mut size) };
    if result != 0 && result != 234 { return Err(format!("Cannot read installer property {name}: {result}")); }
    if size > 32768 { return Err("Installer property exceeds limit".into()); }
    let mut buffer = vec![0u16; size as usize + 1];
    size += 1;
    let result = unsafe { MsiGetPropertyW(handle, wide(name).as_ptr(), buffer.as_mut_ptr(), &mut size) };
    if result != 0 { return Err(format!("Cannot read installer property {name}: {result}")); }
    String::from_utf16(&buffer[..size as usize]).map_err(|_| "Invalid installer property encoding".into())
}
fn set_property(handle: u32, name: &str, value: &str) -> Result<(), String> {
    let result = unsafe { MsiSetPropertyW(handle, wide(name).as_ptr(), wide(value).as_ptr()) };
    if result == 0 { Ok(()) } else { Err(format!("Cannot set installer property {name}: {result}")) }
}
fn product_directory() -> Result<PathBuf, String> {
    let id = Guid { a: 0xf1b32785, b: 0x6fba, c: 0x4fcf, d: [0x9d,0x55,0x7b,0x8e,0x7f,0x15,0x70,0x91] };
    let mut pointer = std::ptr::null_mut();
    let result = unsafe { SHGetKnownFolderPath(&id, 0, 0, &mut pointer) };
    if result < 0 || pointer.is_null() { return Err("Cannot resolve the installing user's LocalAppData".into()); }
    let mut length = 0;
    unsafe { while *pointer.add(length) != 0 { length += 1; } }
    let path = String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) });
    unsafe { CoTaskMemFree(pointer.cast()); }
    let directory = PathBuf::from(path.map_err(|_| "Invalid LocalAppData encoding")?).join("jp.seraf.ktn.syndocal");
    check_path(&directory)?;
    Ok(directory)
}
fn check_path(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_attributes() & 0x400 != 0 => return Err("Redirected extension settings rejected".into()),
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("Cannot inspect extension path: {error}")),
        }
    }
    Ok(())
}
fn read_settings(directory: &Path) -> Result<Option<String>, String> {
    let path = directory.join("extensions.ini");
    check_path(&path)?;
    match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() || meta.len() > 4096 => return Err("Invalid extension settings file or size".into()),
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect extension settings: {error}")),
    }
    let text = fs::read_to_string(path).map_err(|error| format!("Cannot read extension settings: {error}"))?;
    settings::decode(&text)?;
    Ok(Some(text))
}
fn prepare(handle: u32) -> Result<(), String> {
    let directory = product_directory()?;
    let previous = read_settings(&directory)?;
    if property(handle, "SYNDOCAL_CHOICES_READY")? != "1" {
        let enabled = match property(handle, "DJLINK")?.as_str() {
            "" => previous.as_deref().map(settings::decode).transpose()?.unwrap_or(false),
            "0" => false,
            "1" => true,
            _ => return Err("DJLINK must be 0 or 1".into()),
        };
        set_property(handle, "SYNDOCAL_DJLINK_SELECTED", if enabled { "1" } else { "" })?;
        set_property(handle, "SYNDOCAL_CHOICES_READY", "1")?;
    }
    Ok(())
}
fn plan(handle: u32) -> Result<(), String> {
    let directory = product_directory()?;
    read_settings(&directory)?;
    let selected = property(handle, "SYNDOCAL_DJLINK_SELECTED")?;
    let enabled = match selected.as_str() { "" => "0", "1" => "1", _ => return Err("Invalid installer extension choice".into()) };
    let token = format!("{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Invalid clock")?.as_nanos());
    let data = format!("{enabled}\n{token}");
    for action in ["SyndocalWriteExtensions", "SyndocalRollbackExtensions", "SyndocalCommitExtensions"] { set_property(handle, action, &data)?; }
    Ok(())
}
fn action_data(handle: u32) -> Result<(bool, String), String> {
    let data = property(handle, "CustomActionData")?;
    let (enabled, token) = data.split_once('\n').ok_or("Missing extension transaction data")?;
    let enabled = match enabled { "0" => false, "1" => true, _ => return Err("Invalid transaction choice".into()) };
    if token.is_empty() || token.len() > 80 || !token.bytes().all(|b| b.is_ascii_digit() || b == b'-') { return Err("Invalid transaction identity".into()); }
    Ok((enabled, token.to_owned()))
}
fn transaction_file(directory: &Path, token: &str, suffix: &str) -> PathBuf { directory.join(format!(".extensions-{token}.{suffix}")) }
fn exclusive_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    check_path(path)?;
    let mut file = fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|error| format!("Cannot create extension transaction: {error}"))?;
    file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|error| format!("Cannot persist extension transaction: {error}"))
}
fn replace(directory: &Path, token: &str, text: &str) -> Result<(), String> {
    let temporary = transaction_file(directory, token, "new");
    let target = directory.join("extensions.ini");
    exclusive_write(&temporary, text.as_bytes())?;
    check_path(&target)?;
    if unsafe { MoveFileExW(wide(&temporary).as_ptr(), wide(&target).as_ptr(), 9) } == 0 {
        return Err(format!("Cannot replace extension settings: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}
fn write_choice(directory: &Path, enabled: bool, token: &str) -> Result<(), String> {
    let previous = read_settings(directory)?;
    let existed = directory.exists();
    fs::create_dir_all(directory).map_err(|error| format!("Cannot create extension directory: {error}"))?;
    check_path(directory)?;
    let backup = match previous { Some(text) => format!("present\n{text}"), None => if existed { "absent\n".into() } else { "absent-directory\n".into() } };
    exclusive_write(&transaction_file(directory, token, "rollback"), backup.as_bytes())?;
    replace(directory, token, if enabled { "[SyndocalExtensions]\nSchema=1\nDJLink=1\n" } else { "[SyndocalExtensions]\nSchema=1\nDJLink=0\n" })
}
fn remove_if_present(path: &Path) -> Result<(), String> {
    check_path(path)?;
    match fs::remove_file(path) { Ok(()) => Ok(()), Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()), Err(error) => Err(format!("Cannot clean extension transaction: {error}")) }
}
fn finish(directory: &Path, token: &str, rollback: bool) -> Result<(), String> {
    let backup = transaction_file(directory, token, "rollback");
    check_path(&backup)?;
    let mut remove_directory = false;
    if rollback {
        match fs::symlink_metadata(&backup) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(meta) if !meta.is_file() || meta.len() > 4120 => return Err("Invalid extension rollback file".into()),
            Err(error) => return Err(format!("Cannot inspect extension rollback: {error}")),
            Ok(_) => (),
        }
        let bytes = fs::read_to_string(&backup).map_err(|error| error.to_string())?;
        remove_if_present(&transaction_file(directory, token, "new"))?;
        match bytes.split_once('\n') {
            Some(("present", text)) => { settings::decode(text)?; replace(directory, token, text)?; },
            Some((state @ ("absent" | "absent-directory"), "")) => { remove_if_present(&directory.join("extensions.ini"))?; remove_directory = state == "absent-directory"; },
            _ => return Err("Invalid extension rollback state".into()),
        }
    }
    remove_if_present(&transaction_file(directory, token, "new"))?;
    remove_if_present(&backup)?;
    if remove_directory {
        match fs::remove_dir(directory) { Ok(()) => (), Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => (), Err(error) => return Err(error.to_string()) }
    }
    Ok(())
}
fn run(handle: u32, action: impl FnOnce() -> Result<(), String> + std::panic::UnwindSafe) -> u32 {
    let error = match std::panic::catch_unwind(action) { Ok(Ok(())) => return 0, Ok(Err(error)) => error, Err(_) => "Extension installer action failed".into() };
    unsafe {
        let record = MsiCreateRecord(1);
        if record != 0 {
            MsiRecordSetStringW(record, 0, wide("Syndocal extension settings: [1]").as_ptr());
            MsiRecordSetStringW(record, 1, wide(error).as_ptr());
            MsiProcessMessage(handle, 0x0100_0000, record);
            MsiCloseHandle(record);
        }
    }
    1603
}
#[no_mangle]
pub extern "system" fn PrepareExtensions(handle: u32) -> u32 { run(handle, || prepare(handle)) }
#[no_mangle]
pub extern "system" fn PlanExtensions(handle: u32) -> u32 { run(handle, || plan(handle)) }
#[no_mangle]
pub extern "system" fn WriteExtensions(handle: u32) -> u32 { run(handle, || { let (enabled, token) = action_data(handle)?; write_choice(&product_directory()?, enabled, &token) }) }
#[no_mangle]
pub extern "system" fn RollbackExtensions(handle: u32) -> u32 { run(handle, || { let (_, token) = action_data(handle)?; finish(&product_directory()?, &token, true) }) }
#[no_mangle]
pub extern "system" fn CommitExtensions(handle: u32) -> u32 { run(handle, || { let (_, token) = action_data(handle)?; finish(&product_directory()?, &token, false) }) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn choices_commit_rollback_and_invalid_preservation() {
        let root = std::env::temp_dir().join(format!("syndocal-msi-policy-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        write_choice(&root, true, "1").unwrap(); finish(&root, "1", false).unwrap();
        assert_eq!(settings::decode(&read_settings(&root).unwrap().unwrap()), Ok(true));
        write_choice(&root, false, "2").unwrap(); finish(&root, "2", true).unwrap();
        assert_eq!(settings::decode(&read_settings(&root).unwrap().unwrap()), Ok(true));
        write_choice(&root, false, "3").unwrap(); finish(&root, "3", false).unwrap();
        assert_eq!(settings::decode(&read_settings(&root).unwrap().unwrap()), Ok(false));
        let future = "[SyndocalExtensions]\nSchema=2\nDJLink=1\n";
        fs::write(root.join("extensions.ini"), future).unwrap();
        assert!(write_choice(&root, true, "4").is_err());
        assert_eq!(fs::read_to_string(root.join("extensions.ini")).unwrap(), future);
        fs::remove_file(root.join("extensions.ini")).unwrap(); fs::remove_dir(&root).unwrap();
        write_choice(&root, true, "5").unwrap(); finish(&root, "5", true).unwrap(); assert!(!root.exists());
        finish(&root, "6", true).unwrap(); assert!(!root.exists());
    }
}
