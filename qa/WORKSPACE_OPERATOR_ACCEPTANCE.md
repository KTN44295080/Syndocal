# T22 Named Workspace and Operator Lock Acceptance

- Date: 2026-07-23
- Baseline: `5a128e0`
- Scope: named multi-window workspaces, seven detachable operator panes, Full/Partial show-operation lock
- Product boundary: workspace geometry is device-local; operator policy is show state. The lock is an operational guard, not an operating-system or adversarial security boundary.

## Result

**T22 software and native-window PASS.** Operators can save, recall and delete bounded named workspaces containing the main layout and native placement of Stage, Timeline, Programmer, Setup, Live, Mixer and Touch panes. A show can persist a Full or Partial operator policy without storing a plaintext or reversibly encoded credential. Real WebView2 acceptance restored all seven panes to their requested positions and sizes.

## Named workspace contract

- Up to 16 machine-local profiles are stored under the versioned `syndocal.namedWorkspaces.v1` key. Names are 1-48 characters; layouts and placements are validated before use.
- Profiles capture the current main `WorkspaceLayout` plus the placement, dimensions and maximized state of each open native pane. Duplicate pane entries, unknown kinds, invalid geometry and off-screen placements fail closed.
- Device-specific monitor coordinates are deliberately excluded from `.sdc`. They are operator convenience state and do not dirty or alter the show file.
- The seven detachable panes are Stage 2D Map, Timeline, Programmer, Setup, Live Desk, Live Mixer and Touch. Detached pane windows do not inherit the main window's maximize-on-start controller.

## Operator policy contract

- `ProjectFile.operator_policy` is additive, optional and omitted when unset. Legacy projects without the field keep their prior serialization shape.
- The policy stores `lock_mode`, `lock_on_load` and a versioned `PBKDF2-SHA256` verifier. The frontend creates a 16-byte random salt, uses 600,000 iterations and stores a 32-byte verifier. Backend validation accepts only the named scheme, 100,000-2,000,000 iterations, 16-64 decoded salt bytes and exactly 32 verifier bytes.
- Plaintext and reversible credentials are never written to `.sdc`, local storage or workspace profiles. User templates strip the policy so reusable creation aids never become credential carriers.
- Full Lock leaves output running and exposes only emergency DMX blackout, all blackout, all clear and unlock. Programming, normal operation and project replacement are blocked.
- Partial Lock keeps Control/Live, Live Mixer and Touch available while blocking Setup, Programmer, project replacement and project mutations. Detached Setup/Programmer panes show the same restricted boundary.
- Command gating is shared by the normal frontend invoke path. Lock state is synchronized across the application's WebView windows by a verifier-keyed device-local record; project load applies `lock_on_load` only after the policy has been read successfully.
- Policy-only changes participate in project dirty state, save and undo. A failed project load cannot replace the active policy.

## Automated evidence

- Full Rust workspace: PASS, exit code 0.
- Engine: 400 passed, 1 manual release benchmark ignored.
- Tauri/Syndocal: 332 passed, 9 hardware/tool-dependent ignored.
- Video: 114 passed, 1 real-GPU acceptance ignored.
- Workspace/operator helper: 27 assertions passed, including bounded profiles, strict parsing, password absence, PBKDF2 verification and Full/Partial command gates.
- TypeScript: `tsc --noEmit` PASS.
- Localization: 2802/2802 static Japanese strings; zero unprotected user-data labels.
- Terminology, project-storage and desktop-window-mode helper gates: PASS.
- Focused workspace/operator viewport: PASS 5/5 at 1920x1080, 1920x1032, 2048x1152, 1366x768 and 1280x720. The menu, disclosures, both lock modes and all seven pane routes remained contained.
- Full viewport matrix: PASS 232/232 at the same five sizes, including all existing large-data, Scene Matrix and pane-window surfaces.
- Native WebView2 placement acceptance: PASS 7/7 in one process. Requested placements were restored within 16 px, with outer sizes in the accepted 840-900 by 500-580 px range: Stage (48,64), Timeline (90,98), Programmer (132,132), Setup (174,166), Live (216,200), Mixer (258,234), Touch (300,268). Evidence: `C:\Users\kouty\AppData\Local\Temp\syndocal-workspace-acceptance-20260723-002700\native-workspace-acceptance.json`.
- Production frontend build: PASS; main entry 506.33 kB (150.35 kB gzip), CSS 474.20 kB (80.25 kB gzip). Existing chunk-size and manual-chunk cycle warnings remain non-fatal build debt.

## Remaining external boundary

- T22 does not claim resistance against a user with filesystem, browser-devtools or operating-system access. It is a control-surface guard for show operation.
- Physical multi-monitor hot-plug/topology changes remain operator acceptance. Invalid or off-screen saved placements fail closed rather than opening unreachable windows.
- Internal 3D visualization remains outside the product goal. Representative lighting output is accepted through the T23 Art-Net external-visualizer path.
