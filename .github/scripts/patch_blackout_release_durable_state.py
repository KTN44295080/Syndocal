from pathlib import Path
import runpy


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
main = Path("app/src-tauri/src/main.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
durable = Path("app/src-tauri/src/control_plane_durability.rs")

# Keep the durable model as a real reviewable Rust source file on the branch.
# This generator only wires it into the compiled binary after the typed recovery
# error exists, so the generated-source tranche stays deterministic.
if not durable.exists():
    raise RuntimeError("R4 durable state module is missing")
durable_source = durable.read_text(encoding="utf-8")
for marker in (
    "pub(crate) struct DurableR4StateV1",
    "DurableR4RecordStateV1::Prepared",
    "OutputControlErrorCodeV1::InterruptedBeforeCommit",
    "persist_r4_durability_state",
):
    if marker not in durable_source:
        raise RuntimeError(f"R4 durable state module missing reviewed marker: {marker}")

# Make activation visible in the bounded generated diff rather than silently
# compiling a pre-existing staged file. The final source keeps the stricter
# wording after the patch scaffolding is removed.
replace_exact(
    durable,
    '''//! This module owns only the strict durable state machine. Filesystem path
//! resolution and integration with the live Release transaction remain in the
//! app adapter so the state model can be tested without Tauri or output I/O.
''',
    '''//! This module owns the strict durable state machine compiled into the R4
//! Release lane. Filesystem path resolution and live transaction persistence are
//! app-adapter responsibilities so the state model remains independently testable.
''',
    "activate reviewed durable R4 state module",
)

# A process crash after durable prepare but before the terminal commit is not an
# ordinary internal error. Recovery deliberately rolls the ambiguous release
# toward Blackout-on and records a typed terminal fact so the same request is
# never republished implicitly after restart.
replace_exact(
    protocol,
    '''    ConsentDeviceRemoved,
    Busy,
''',
    '''    ConsentDeviceRemoved,
    InterruptedBeforeCommit,
    Busy,
''',
    "typed interrupted-before-commit output error",
)

replace_exact(
    controller,
    '''  | "consent_device_removed"
  | "busy"
''',
    '''  | "consent_device_removed"
  | "interrupted_before_commit"
  | "busy"
''',
    "renderer interrupted-before-commit error union",
)
replace_exact(
    controller,
    '''  "consent_device_removed",
  "busy",
''',
    '''  "consent_device_removed",
  "interrupted_before_commit",
  "busy",
''',
    "renderer interrupted-before-commit accepted error",
)

replace_exact(
    main,
    '''mod control_plane_runtime;
mod control_plane_security;
mod dvc_import;
''',
    '''mod control_plane_runtime;
mod control_plane_security;
mod control_plane_durability;
mod dvc_import;
''',
    "compile R4 durable state module",
)

# Startup recovery is an internal helper, not a separately inventoried patch
# root. The graph therefore has one durable-state root while still keeping the
# large main.rs replacement logic independently reviewable.
runpy.run_path(
    ".github/scripts/r4_durable_startup_helper.py",
    run_name="__main__",
)
