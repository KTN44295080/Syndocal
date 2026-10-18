//! Child-process constructor that keeps console-subsystem children from opening
//! a visible console window when the GUI application spawns them on Windows.
//!
//! Production code must build every child through [`hidden_command`] instead of
//! `Command::new` (enforced by `app/scripts/check-hidden-child-console.mjs`).
//!
//! `CREATE_NO_WINDOW` gives the child a hidden console instead of a new visible
//! one. Standard handles configured through `Stdio` (pipes, null, inherited) are
//! unaffected, and the child stays killable and waitable through its `Child`
//! handle. It is not a process-group or job flag, so it composes with them.
//!
//! `Command::creation_flags` replaces the previous value. A caller that needs
//! another creation flag must OR it with [`CREATE_NO_WINDOW`].

use std::{ffi::OsStr, process::Command};

/// Windows `CREATE_NO_WINDOW` process creation flag.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a `Command` that does not open a visible console window on Windows.
/// Identical to `Command::new` on other platforms.
pub fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn hidden_child_keeps_piped_stdio_and_exit_status() {
        let shell = std::env::var_os("COMSPEC").expect("COMSPEC must be available");
        let output = hidden_command(shell)
            .args(["/d", "/c", "echo hidden-ok & exit /b 3"])
            .stdin(Stdio::null())
            .output()
            .expect("hidden console child must start");
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stdout).contains("hidden-ok"));
    }

    #[test]
    fn hidden_child_can_still_be_killed() {
        let shell = std::env::var_os("COMSPEC").expect("COMSPEC must be available");
        let mut child = hidden_command(shell)
            .args(["/d", "/s", "/c", "for /L %i in (1,1,2147483647) do @rem"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("hidden console child must start");
        child.kill().expect("hidden console child must be killable");
        assert!(!child.wait().expect("child must be reaped").success());
    }
}
