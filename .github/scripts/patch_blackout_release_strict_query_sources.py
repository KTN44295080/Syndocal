from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count == 0:
        raise RuntimeError(f"{label}: expected at least one match")
    path.write_text(text.replace(old, new), encoding="utf-8")


main = Path("app/src-tauri/src/main.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
checker = Path("app/scripts/check-blackout-release-runtime.mjs")
manifest = Path("app/src/tauri-invoke-manifest.json")
invoke_types = Path("app/src/tauriInvokeCommands.ts")
control = Path("app/src-tauri/src/control_plane.rs")

# Keep generic runtime helpers/protocol DTOs available for future R4 slices, but
# give the production Release workflow dedicated Tauri source names. One source
# can then continue to resolve to exactly one canonical operation when Arm and
# Takeover are implemented later.
replace_exact(
    main,
    '''fn query_output_control_authority_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
) -> Result<OutputControlAuthorityBundleV1, String> {
''',
    '''fn query_blackout_release_authority_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
) -> Result<OutputControlAuthorityBundleV1, String> {
''',
    "dedicated Release authority Tauri source",
)
replace_exact(
    main,
    '''fn query_output_consent_status_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: OutputConsentStatusRequestV1,
) -> Result<OutputConsentStatusV1, String> {
''',
    '''fn query_blackout_release_consent_status_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: OutputConsentStatusRequestV1,
) -> Result<OutputConsentStatusV1, String> {
''',
    "dedicated Release consent-status Tauri source",
)
replace_exact(
    main,
    '''            query_output_control_authority_v1,
''',
    '''            query_blackout_release_authority_v1,
''',
    "register dedicated Release authority source",
)
replace_exact(
    main,
    '''            query_output_consent_status_v1,
''',
    '''            query_blackout_release_consent_status_v1,
''',
    "register dedicated Release consent-status source",
)

for path in (controller, checker, manifest, invoke_types):
    replace_all(
        path,
        '"query_output_control_authority_v1"',
        '"query_blackout_release_authority_v1"',
        f"dedicated Release authority source in {path}",
    )
    replace_all(
        path,
        '"query_output_consent_status_v1"',
        '"query_blackout_release_consent_status_v1"',
        f"dedicated Release consent-status source in {path}",
    )

# Workflow source classification was generated earlier with the generic helper
# names. Rename only the source identity strings; operation IDs and generic
# protocol query DTOs remain stable internal contracts.
replace_all(
    control,
    '"query_output_control_authority_v1"',
    '"query_blackout_release_authority_v1"',
    "canonical Release authority workflow source identity",
)
replace_all(
    control,
    '"query_output_consent_status_v1"',
    '"query_blackout_release_consent_status_v1"',
    "canonical Release consent-status workflow source identity",
)
