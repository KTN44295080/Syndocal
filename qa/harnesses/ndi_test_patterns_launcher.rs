use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    // The development host's .NET 7 WindowsDesktop runtimeconfig is corrupt.
    // LatestMajor keeps this QA workaround process-local and leaves Program Files untouched.
    std::env::set_var("DOTNET_ROLL_FORWARD", "LatestMajor");
    let executable = r"C:\Program Files\NDI\NDI 6 Tools\Test Patterns\Application.Network.TestPatterns.exe";
    match Command::new(executable).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("NDI Test Patterns exited with {status}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("Failed to launch NDI Test Patterns: {error}");
            ExitCode::FAILURE
        }
    }
}
