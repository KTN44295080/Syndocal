#![cfg(windows)]
use std::{
    fs::File,
    mem::{offset_of, size_of},
    os::{
        raw::c_void,
        windows::{
            ffi::OsStrExt,
            io::{AsRawHandle, FromRawHandle},
        },
    },
    path::Path,
    ptr::copy_nonoverlapping,
};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FileAttributeTagInfo, FileDispositionInfo, FileRenameInfo,
            GetFileInformationByHandleEx, SetFileInformationByHandle, CREATE_NEW, DELETE,
            FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
            FILE_CREATION_DISPOSITION, FILE_DISPOSITION_INFO, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_RENAME_INFO, FILE_RENAME_INFO_0, FILE_SHARE_READ, OPEN_EXISTING,
        },
    },
};

fn encode_path(path: &Path) -> Result<Vec<u16>, String> {
    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if units.is_empty() {
        return Err("Recording path must not be empty".into());
    }
    if units.contains(&0) {
        return Err(format!(
            "Recording path contains an embedded NUL: {}",
            path.display()
        ));
    }
    Ok(units)
}

fn validate_regular_file(file: &File, path: &Path) -> Result<(), String> {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast::<c_void>(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(|error| {
        format!(
            "Cannot inspect locked recording path {}: {error}",
            path.display()
        )
    })?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(format!(
            "Recording path must be a regular file, not a reparse point: {}",
            path.display()
        ));
    }
    let metadata = file.metadata().map_err(|error| {
        format!(
            "Cannot inspect locked recording path {}: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "Recording path must be a regular file, not a directory: {}",
            path.display()
        ));
    }
    Ok(())
}

fn open_with(
    path: &Path,
    disposition: FILE_CREATION_DISPOSITION,
    operation: &str,
) -> Result<File, String> {
    let mut units = encode_path(path)?;
    units.push(0);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(units.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0 | DELETE.0,
            FILE_SHARE_READ,
            None,
            disposition,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|error| {
        format!(
            "Cannot {operation} locked recording path {}: {error}",
            path.display()
        )
    })?;
    let file = unsafe { File::from_raw_handle(handle.0) };
    validate_regular_file(&file, path)?;
    Ok(file)
}

pub(super) fn open_locked(path: &Path) -> Result<File, String> {
    open_with(path, OPEN_EXISTING, "open")
}

pub(super) fn create_locked(path: &Path) -> Result<File, String> {
    open_with(path, CREATE_NEW, "create")
}

pub(super) fn rename_no_replace(source: &File, target: &Path) -> Result<(), String> {
    if !target.is_absolute() {
        return Err(format!(
            "Recording publication target must be absolute: {}",
            target.display()
        ));
    }
    let name = encode_path(target)?;
    let name_bytes = name
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u32::try_from(length).ok())
        .ok_or_else(|| "Recording publication target path is too long".to_string())?;
    let header_size = offset_of!(FILE_RENAME_INFO, FileName);
    let byte_size = header_size
        .checked_add(name_bytes as usize)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| "Recording rename information is too large".to_string())?;
    let word_count = (byte_size as usize + size_of::<u64>() - 1) / size_of::<u64>();
    let mut info = vec![0_u64; word_count];
    unsafe {
        let header = info.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*header).Anonymous = FILE_RENAME_INFO_0 {
            ReplaceIfExists: false,
        };
        (*header).RootDirectory = HANDLE::default();
        (*header).FileNameLength = name_bytes;
        copy_nonoverlapping(
            name.as_ptr(),
            info.as_mut_ptr()
                .cast::<u8>()
                .add(header_size)
                .cast::<u16>(),
            name.len(),
        );
        SetFileInformationByHandle(
            HANDLE(source.as_raw_handle()),
            FileRenameInfo,
            info.as_ptr().cast::<c_void>(),
            byte_size,
        )
    }
    .map_err(|error| {
        format!(
            "Cannot rename recording without replacing {}: {error}",
            target.display()
        )
    })
}

pub(super) fn delete_owned(file: &File) -> Result<(), String> {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast::<c_void>(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(|error| format!("Cannot mark owned recording for deletion: {error}"))
}
