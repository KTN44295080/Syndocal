# Video Clip Slot B5 native evidence — 2026-08-14

## Build and process identity

- Command: `pnpm --dir app tauri build --no-bundle`
- Result: PASS (exit 0, 101.2 s)
- Executable: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- SHA-256: `947716999AD8FEFFC10F0DE19056638A7D1073AC0BD562ED4D7EE0F6C7FD1191`
- The exact resolved executable path was checked before build and only exact-path processes were eligible for termination.
- After launch: exact process count 1, responsive count 1, `Syndocal` window count 1.
- The verified Syndocal window was maximized before UI actions.

## Native UI acceptance

Project opened from `samples\phase1-mini-show.sdc`; all mutations below remained unsaved QA state and the source project file was not saved.

- Edit > Video showed the 32-slot bank in the native app with no outer application scroll.
- The legacy layer was normalized to slot ID 3.
- Inspector opened inside the Video surface and exposed asset, slot/runtime identity, assign, duplicate, default, reorder, delete guards, in/out points, loop mode, speed, launch quantization, cue points, effect overrides, and Save.
- Duplicate created slot ID 4 and updated the bank from 1/32 to 2/32.
- Control > Video was visible as a real native surface with the same two slots, Import, quantization, Cancel Queue, Take, and output pane.
- Queue slot 4 enabled Take and Cancel Queue after authoritative runtime refresh.
- Cancel Queue returned both controls to disabled and removed the queued runtime state.
- Queue slot 4 followed by Take changed the native runtime display from `再生中 #3` to `再生中 #4`.

## Cleanup

- The exact checkout process was force-terminated after QA.
- Exact process count after cleanup: 0.
- Only unsaved QA duplication/runtime state was discarded.

## Boundary

- Transition duration remains visibly disabled and belongs to the later C2 transition tranche; this B5 evidence does not claim transition execution.
