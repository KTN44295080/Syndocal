//! Process shutdown after the operator's main-window close.
//!
//! Tauri ends the process only when its last window is destroyed. A native
//! Display output window is a Tauri window, so before this boundary a
//! confirmed main-window close ("Stop and Close") left the Display presenting
//! and the process alive with no operator UI. An armed output without a
//! window (DMX, NDI, Spout) was instead dropped by process exit without the
//! Standby all-deny.
//!
//! When the main window is destroyed while any output is armed or any Display
//! window/worker exists, this module runs the canonical Standby boundary on a
//! dedicated thread (it waits for native windows, which needs the event loop).
//! Every `ExitRequested` raised meanwhile is held: the last Display window can
//! close before its renderer and GPU retirement are joined. Afterwards exit is
//! requested once no window remains, or when an exit was held. The existing
//! managed-lease exit boundary still decides the final exit.
//!
//! A Standby failure is logged and does not keep a window-less process alive.
//! A process that cannot be operated or relaunched (single instance) is worse
//! than ending outputs whose windows are already closed. The managed-lease exit
//! boundary keeps its fail-closed authority over an unreleased external lease.

use std::{
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
    time::Instant,
};

use tauri::{AppHandle, Manager};

use super::{
    apply_standby_output_ownership_role_command, native_video_output_retirement_labels, AppState,
    MachineOutputRole,
};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

const IDLE: u8 = 0;
const RUNNING: u8 = 1;
const FINISHED: u8 = 2;

/// One-shot main-close shutdown state, managed by the Tauri application.
#[derive(Default)]
pub(crate) struct MainWindowCloseShutdown {
    phase: AtomicU8,
    exit_held: AtomicBool,
}

impl MainWindowCloseShutdown {
    /// Claim the shutdown. Only the first main-window destruction runs it.
    fn begin(&self) -> bool {
        self.phase
            .compare_exchange(IDLE, RUNNING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// `ExitRequested` adapter: true while outputs are still being stopped.
    /// A held request is remembered so that the shutdown honours it.
    pub(crate) fn hold_exit(&self) -> bool {
        if self.phase.load(Ordering::Acquire) != RUNNING {
            return false;
        }
        self.exit_held.store(true, Ordering::Release);
        // Re-check: if the shutdown finished in between, it may already have
        // read `exit_held`, so do not hold this request.
        self.phase.load(Ordering::Acquire) == RUNNING
    }

    /// Mark the shutdown finished and report whether an exit was held.
    fn finish(&self) -> bool {
        self.phase.store(FINISHED, Ordering::Release);
        self.exit_held.load(Ordering::Acquire)
    }
}

/// Whether destroying `label` must stop outputs before the process may exit.
/// Retirement-label lookup failures count as open outputs (stop rather than
/// skip).
pub(crate) fn main_close_requires_output_shutdown(
    label: &str,
    output_windows_or_workers_open: bool,
    effective_role: MachineOutputRole,
) -> bool {
    label == MAIN_WINDOW_LABEL
        && (output_windows_or_workers_open || effective_role != MachineOutputRole::Standby)
}

/// Stop outputs, then request exit if no window remains or an exit was held.
/// Returns the Standby result for logging.
pub(crate) fn run_main_close_shutdown<T>(
    shutdown: &MainWindowCloseShutdown,
    standby: impl FnOnce() -> Result<T, String>,
    no_window_remains: impl FnOnce() -> bool,
    request_exit: impl FnOnce(),
) -> Result<T, String> {
    let result = standby();
    let exit_held = shutdown.finish();
    if exit_held || no_window_remains() {
        request_exit();
    }
    result
}

/// Global `WindowEvent::Destroyed` hook, called after the destroyed window's
/// authority retirement. Tauri dispatches it before it raises the
/// last-window `ExitRequested`, so the hold is in place before that request.
pub(crate) fn begin_after_window_destroyed(app: &AppHandle, label: &str) {
    if label != MAIN_WINDOW_LABEL {
        return;
    }
    let state = app.state::<AppState>();
    let outputs_open =
        native_video_output_retirement_labels(app, &state.native_video_output_workers)
            .map(|labels| !labels.is_empty())
            .unwrap_or(true);
    let effective_role = state.engine.output_ownership_status().effective_role;
    if !main_close_requires_output_shutdown(label, outputs_open, effective_role) {
        return;
    }
    let shutdown = app.state::<MainWindowCloseShutdown>();
    if !shutdown.begin() {
        return;
    }
    let worker_app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("syndocal-main-close-shutdown".to_string())
        .spawn(move || {
            let app = worker_app;
            let started = Instant::now();
            let state = app.state::<AppState>();
            let shutdown = app.state::<MainWindowCloseShutdown>();
            let result = run_main_close_shutdown(
                &shutdown,
                || {
                    apply_standby_output_ownership_role_command(
                        &app,
                        &state,
                        MachineOutputRole::Standby,
                    )
                },
                || app.windows().is_empty(),
                || app.exit(0),
            );
            match result {
                Ok(status) => eprintln!(
                    "Main window closed: outputs entered Standby in {} ms (role={:?}, state={:?})",
                    started.elapsed().as_millis(),
                    status.effective_role,
                    status.state
                ),
                Err(error) => eprintln!(
                    "Main window closed: output Standby failed after {} ms: {error}",
                    started.elapsed().as_millis()
                ),
            }
        });
    if let Err(error) = spawned {
        // Without the worker nothing would release a held exit.
        app.state::<MainWindowCloseShutdown>().finish();
        eprintln!("Main window closed: output shutdown worker could not start: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn only_the_main_window_with_live_or_armed_outputs_needs_the_shutdown() {
        assert!(!main_close_requires_output_shutdown(
            MAIN_WINDOW_LABEL,
            false,
            MachineOutputRole::Standby
        ));
        assert!(main_close_requires_output_shutdown(
            MAIN_WINDOW_LABEL,
            true,
            MachineOutputRole::Standby
        ));
        assert!(main_close_requires_output_shutdown(
            MAIN_WINDOW_LABEL,
            false,
            MachineOutputRole::Video
        ));
        assert!(!main_close_requires_output_shutdown(
            "video-output-1",
            true,
            MachineOutputRole::Video
        ));
        assert!(!main_close_requires_output_shutdown(
            "pane-timeline",
            true,
            MachineOutputRole::Video
        ));
    }

    #[test]
    fn exit_is_held_only_while_the_shutdown_runs() {
        let shutdown = MainWindowCloseShutdown::default();
        assert!(!shutdown.hold_exit(), "no hold before a main close");
        assert!(shutdown.begin());
        assert!(
            !shutdown.begin(),
            "a second main destruction cannot restart it"
        );
        assert!(
            shutdown.hold_exit(),
            "the last Display window closing must not end the process"
        );
        assert!(shutdown.finish(), "the held request is reported");
        assert!(!shutdown.hold_exit(), "the final exit request passes");
        assert!(!shutdown.begin(), "the shutdown stays one-shot");
    }

    #[test]
    fn standby_completes_before_exit_is_requested() {
        let shutdown = MainWindowCloseShutdown::default();
        assert!(shutdown.begin());
        let order = RefCell::new(Vec::new());
        let result = run_main_close_shutdown(
            &shutdown,
            || {
                // The last output window closes during retirement.
                assert!(shutdown.hold_exit());
                order.borrow_mut().push("standby");
                Ok::<_, String>(())
            },
            || {
                order.borrow_mut().push("windows");
                true
            },
            || order.borrow_mut().push("exit"),
        );
        assert!(result.is_ok());
        assert_eq!(*order.borrow(), ["standby", "exit"]);
        assert!(!shutdown.hold_exit());
    }

    #[test]
    fn remaining_windows_keep_the_process_unless_an_exit_was_held() {
        let shutdown = MainWindowCloseShutdown::default();
        assert!(shutdown.begin());
        let exits = RefCell::new(0);
        run_main_close_shutdown(
            &shutdown,
            || Ok::<_, String>(()),
            || false,
            || *exits.borrow_mut() += 1,
        )
        .unwrap();
        assert_eq!(
            *exits.borrow(),
            0,
            "an open pane window keeps the application"
        );

        let shutdown = MainWindowCloseShutdown::default();
        assert!(shutdown.begin());
        run_main_close_shutdown(
            &shutdown,
            || {
                assert!(shutdown.hold_exit());
                Ok::<_, String>(())
            },
            || false,
            || *exits.borrow_mut() += 1,
        )
        .unwrap();
        assert_eq!(*exits.borrow(), 1, "a held exit is honoured");
    }

    #[test]
    fn standby_failure_still_releases_a_window_less_process() {
        let shutdown = MainWindowCloseShutdown::default();
        assert!(shutdown.begin());
        let exited = RefCell::new(false);
        let result = run_main_close_shutdown(
            &shutdown,
            || {
                Err::<(), _>(
                    "Native video output video-output-1 did not acknowledge retirement".to_string(),
                )
            },
            || true,
            || *exited.borrow_mut() = true,
        );
        assert!(result.is_err(), "the failure is reported");
        assert!(*exited.borrow());
        assert!(!shutdown.hold_exit());
    }

    #[test]
    fn production_wiring_holds_exit_before_the_managed_boundary() {
        let source = include_str!("main.rs");
        let hook = source
            .find(".on_window_event(|window, event|")
            .expect("global window event hook");
        let hook_end = source[hook..]
            .find(".plugin(tauri_plugin_updater")
            .map(|offset| hook + offset)
            .expect("global window event hook end");
        let hook_body = &source[hook..hook_end];
        let retire = hook_body
            .find("retire_destroyed_window_authority(window)")
            .expect("destroyed authority retirement");
        let begin = hook_body
            .find("main_window_close_shutdown::begin_after_window_destroyed(")
            .expect("main-close shutdown hook");
        assert!(
            retire < begin,
            "authority retirement runs before the shutdown worker starts"
        );
        assert!(source
            .contains(".manage(main_window_close_shutdown::MainWindowCloseShutdown::default())"));

        let exit = source
            .find("if let tauri::RunEvent::ExitRequested { api, .. } = &event {")
            .expect("ExitRequested handler");
        let exit_body = &source[exit..];
        let hold = exit_body
            .find(".hold_exit()")
            .expect("main-close exit hold");
        let boundary = exit_body
            .find("output_lease_keepalive_integration::execute_current_boundary(")
            .expect("managed exit boundary");
        assert!(
            hold < boundary,
            "a held exit must not run the managed exit boundary early"
        );
    }
}
