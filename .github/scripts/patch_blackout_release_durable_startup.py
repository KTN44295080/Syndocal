from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


main = Path("app/src-tauri/src/main.rs")
durable = Path("app/src-tauri/src/control_plane_durability.rs")

replace_exact(
    durable,
    '''    sync::atomic::{AtomicU64, Ordering},
};
''',
    '''    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
};
''',
    "durable R4 process lock imports",
)
replace_exact(
    durable,
    '''static R4_DURABILITY_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
''',
    '''static R4_DURABILITY_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static R4_DURABILITY_FILE_LOCK: Mutex<()> = Mutex::new(());
static R4_DURABILITY_FAILED: AtomicBool = AtomicBool::new(false);

pub(crate) fn lock_r4_durability() -> Result<MutexGuard<'static, ()>, String> {
    R4_DURABILITY_FILE_LOCK
        .lock()
        .map_err(|_| "R4 durability lock was poisoned".to_string())
}

pub(crate) fn r4_durability_failed() -> bool {
    R4_DURABILITY_FAILED.load(Ordering::Acquire)
}

pub(crate) fn latch_r4_durability_failure() {
    R4_DURABILITY_FAILED.store(true, Ordering::Release);
}
''',
    "durable R4 process lock and failure latch",
)
replace_exact(
    durable,
    '''    pub(crate) fn finish_session(&mut self) -> Result<(), DurableR4StateError> {
        self.clean_shutdown = true;
        self.bump_state_generation()?;
        self.validate()
    }

    fn bump_state_generation(&mut self) -> Result<(), DurableR4StateError> {
''',
    '''    pub(crate) fn finish_session(&mut self) -> Result<(), DurableR4StateError> {
        self.clean_shutdown = true;
        self.bump_state_generation()?;
        self.validate()
    }

    pub(crate) fn set_safety_authority(
        &mut self,
        engaged: bool,
        epoch: u64,
        generation: u64,
    ) -> Result<(), DurableR4StateError> {
        validate_nonzero_safe(epoch)?;
        validate_nonzero_safe(generation)?;
        self.safety_blackout_engaged = engaged;
        self.safety_blackout_epoch = epoch;
        self.safety_blackout_generation = generation;
        self.bump_state_generation()?;
        self.validate()
    }

    fn bump_state_generation(&mut self) -> Result<(), DurableR4StateError> {
''',
    "durable R4 authoritative safety snapshot update",
)

replace_exact(
    main,
    '''const OUTPUT_OWNERSHIP_STATE_VERSION: u32 = 1;
const OUTPUT_OWNERSHIP_STATE_FILE: &str = "machine-output-ownership.json";
const PROJECT_RECOVERY_AUTHORITY_STATE_VERSION: u32 = 1;
''',
    '''const OUTPUT_OWNERSHIP_STATE_VERSION: u32 = 1;
const OUTPUT_OWNERSHIP_STATE_FILE: &str = "machine-output-ownership.json";
const R4_DURABILITY_STATE_FILE: &str = "machine-r4-control-plane.json";
const PROJECT_RECOVERY_AUTHORITY_STATE_VERSION: u32 = 1;
''',
    "machine-local R4 durability filename",
)
replace_exact(
    main,
    '''fn initialize_output_ownership(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
''',
    '''fn r4_durability_state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|directory| directory.join(R4_DURABILITY_STATE_FILE))
        .map_err(|error| format!("Unable to resolve R4 durability state path: {error}"))
}

fn initialize_r4_durability(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
    let _guard = control_plane_durability::lock_r4_durability()?;
    let path = r4_durability_state_path(app)?;
    let engine_authority = state.engine.safety_blackout_authority();
    let mut durable = control_plane_durability::load_r4_durability_state(&path)?.unwrap_or(
        control_plane_durability::DurableR4StateV1::new(
            engine_authority.engaged,
            engine_authority.epoch,
            engine_authority.generation,
        )
        .map_err(|error| format!("Unable to initialize R4 durability state: {error:?}"))?,
    );
    let interrupted = durable
        .begin_session()
        .map_err(|error| format!("Unable to begin R4 durability session: {error:?}"))?;

    // This runs while the engine output-ownership gate is still StartupDenied.
    // Unclean/Prepared recovery converges the fresh Engine toward Blackout-on
    // through the independent S0 priority queue before any output can be armed.
    if durable.safety_blackout_engaged && !engine_authority.engaged {
        state
            .engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(2))?;
    }
    let recovered_authority = state.engine.safety_blackout_authority();
    durable
        .set_safety_authority(
            recovered_authority.engaged,
            recovered_authority.epoch,
            recovered_authority.generation,
        )
        .map_err(|error| format!("Unable to record recovered safety authority: {error:?}"))?;
    control_plane_durability::persist_r4_durability_state(&path, &durable)?;
    if interrupted > 0 {
        eprintln!(
            "R4 recovery terminalized {interrupted} interrupted Blackout Release request(s) behind StartupDenied"
        );
    }
    Ok(())
}

fn finish_r4_durability(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
    let _guard = control_plane_durability::lock_r4_durability()?;
    let path = r4_durability_state_path(app)?;
    let Some(mut durable) = control_plane_durability::load_r4_durability_state(&path)? else {
        return Ok(());
    };
    let authority = state.engine.safety_blackout_authority();
    durable
        .set_safety_authority(authority.engaged, authority.epoch, authority.generation)
        .map_err(|error| format!("Unable to record shutdown safety authority: {error:?}"))?;
    durable
        .finish_session()
        .map_err(|error| format!("Unable to close R4 durability session: {error:?}"))?;
    control_plane_durability::persist_r4_durability_state(&path, &durable)
}

fn initialize_output_ownership(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
''',
    "R4 durability startup and clean shutdown helpers",
)
replace_exact(
    main,
    '''            if let Err(error) = initialize_output_ownership(app.handle(), &state) {
                eprintln!("machine output ownership remains Standby: {error}");
            }
            Ok(())
''',
    '''            if let Err(error) = initialize_r4_durability(app.handle(), &state) {
                state.engine.mark_output_ownership_startup_failure(format!(
                    "R4 durability recovery failed: {error}"
                ));
                eprintln!(
                    "machine output ownership remains Standby: R4 durability recovery failed: {error}"
                );
            } else if let Err(error) = initialize_output_ownership(app.handle(), &state) {
                eprintln!("machine output ownership remains Standby: {error}");
            }
            Ok(())
''',
    "recover R4 durability before ownership initialization",
)
replace_exact(
    main,
    '''        .run(|app_handle, event| {
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
''',
    '''        .run(|app_handle, event| {
            if matches!(&event, tauri::RunEvent::Exit) {
                let state = app_handle.state::<AppState>();
                if let Err(error) = finish_r4_durability(app_handle, &state) {
                    // Keep the prior `clean_shutdown=false` marker. The next
                    // startup recovers toward Blackout-on rather than trusting
                    // a shutdown whose durable commit failed.
                    eprintln!("R4 clean-shutdown durability commit failed: {error}");
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
''',
    "durable R4 clean shutdown marker",
)
