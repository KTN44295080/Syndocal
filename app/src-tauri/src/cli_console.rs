//! Console output for terminal-invoked command-line modes.
//!
//! Release builds use the Windows GUI subsystem so launching Syndocal never
//! opens a console window. A GUI-subsystem process started from a terminal has
//! no console and no standard handles unless the caller redirected them, so a
//! human-invoked CLI mode attaches to the parent console before printing.
//! Redirected handles (pipes or files, as used by the release scripts) are
//! valid already and are left untouched.

/// Attaches stdout/stderr to the parent console when they are not already
/// connected. Best effort: without a parent console (for example when launched
/// from Explorer) nothing changes. A no-op outside Windows.
pub(crate) fn attach_parent_console_for_cli() {
    #[cfg(windows)]
    attach_parent_console();
}

#[cfg(windows)]
fn attach_parent_console() {
    use std::{fs::OpenOptions, os::windows::io::IntoRawHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        System::Console::{
            AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
            STD_HANDLE, STD_OUTPUT_HANDLE,
        },
    };

    fn is_connected(handle: STD_HANDLE) -> bool {
        matches!(unsafe { GetStdHandle(handle) }, Ok(value) if !value.0.is_null())
    }

    let stdout_missing = !is_connected(STD_OUTPUT_HANDLE);
    let stderr_missing = !is_connected(STD_ERROR_HANDLE);
    if !stdout_missing && !stderr_missing {
        return;
    }
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_err() {
        return;
    }
    for (std_handle, missing) in [
        (STD_OUTPUT_HANDLE, stdout_missing),
        (STD_ERROR_HANDLE, stderr_missing),
    ] {
        if !missing {
            continue;
        }
        // The console screen buffer stays open for the life of the short CLI
        // process; the handle is intentionally handed to the standard slot.
        if let Ok(console) = OpenOptions::new().write(true).open("CONOUT$") {
            let _ = unsafe { SetStdHandle(std_handle, HANDLE(console.into_raw_handle())) };
        }
    }
}
