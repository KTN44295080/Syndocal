from pathlib import Path
import re


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


engine = Path("crates/engine/src/lib.rs")
runtime = Path("app/src-tauri/src/control_plane_runtime.rs")

engine_source = engine.read_text(encoding="utf-8")
method_name = "acquire_output_capability_for_control_plane"
if method_name not in engine_source:
    pattern = re.compile(
        r"(?P<method>    pub fn output_ownership_status\(&self\) -> OutputOwnershipStatus \{\n"
        r"        self\.(?P<field>[A-Za-z_][A-Za-z0-9_]*)\.status\(\)\n"
        r"    \})"
    )
    matches = list(pattern.finditer(engine_source))
    if len(matches) != 1:
        raise RuntimeError(
            f"EngineHandle output-ownership status accessor: expected one simple gate accessor, found {len(matches)}"
        )
    match = matches[0]
    field = match.group("field")
    addition = f'''{match.group("method")}

    /// Hold the existing output-ownership gate across an R4 control-plane
    /// commit boundary. A normal ownership transition fences new permits and
    /// waits for this permit before it can retire the Lighting resource.
    pub fn {method_name}(
        &self,
        capability: OutputCapability,
    ) -> Result<OutputOwnershipPermit, String> {{
        self.{field}.acquire(capability)
    }}'''
    engine_source = (
        engine_source[: match.start()]
        + addition
        + engine_source[match.end() :]
    )
    engine.write_text(engine_source, encoding="utf-8")

# Owner-gate + terminal-rejection patches run before this one, so a failed
# permit acquisition can become a retained stale-fence terminal fact. Keep the
# permit alive through consent consumption, Engine publication, and terminal
# receipt storage: ownership transfer cannot straddle that correctness boundary.
replace_exact(
    runtime,
    '''    let (inflight, audit_sequence) =
        match state.runtime_control_plane.admit_and_audit_output_control(
''',
    '''    let _lighting_owner_permit = if matches!(&request.action, OutputControlActionV1::ReleaseBlackout) {
        match state.engine.acquire_output_capability_for_control_plane(
            engine::OutputCapability::Lighting,
        ) {
            Ok(permit) => Some(permit),
            Err(_) => {
                drop(coordinator);
                drop(external_admission);
                return retain_output_control_rejection(
                    state,
                    &request,
                    key,
                    shape_sha256,
                    OutputControlErrorCodeV1::StaleFence,
                );
            }
        }
    } else {
        None
    };

    let (inflight, audit_sequence) =
        match state.runtime_control_plane.admit_and_audit_output_control(
''',
    "hold Lighting output permit across Release terminal boundary",
)
