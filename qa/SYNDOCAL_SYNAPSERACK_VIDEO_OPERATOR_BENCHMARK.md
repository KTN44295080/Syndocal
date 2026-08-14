# Syndocal vs SynapseRack: Video Operator Benchmark

Status: acceptance plan, not a completed comparison. Results must be recorded from the pinned builds and the same operator start state.

## Fixed test rig

- Primary Syndocal production rig: RTX 5090, Intel 13900KF, 128 GB RAM.
- Two independent 1920x1080 HDMI displays plus the operator display.
- Same 32-file media pack: 24 H.264 MP4 loops, 4 stills, 2 alpha-capable assets, 2 intentionally missing/relinked files.
- Same audio input and 120 BPM reference track with predeclared beat markers.
- Same MIDI controller, OSC sender, and DMX input where the competing product supports the route.
- Fresh project and warm-cache runs are recorded separately.
- The operator receives the same one-page task wording and may not use hidden developer commands.

## Measurements

For every task record:

- elapsed time to a visibly correct result;
- pointer clicks/taps, keyboard actions, drag operations, and modal confirmations;
- navigation reversals or opening the wrong surface;
- errors, accidental live-output changes, and recovery actions;
- whether the operator could predict the result before committing;
- output-frame drops, audio discontinuities, and DMX/engine telemetry where applicable.

Syndocal passes a task only when it is no slower and uses no more primary interactions than the pinned SynapseRack flow, or when a measured safety/clarity advantage justifies one explicit confirmation. Merely exposing more controls is not an advantage.

## Operator tasks

| ID | Start state | Required result | Syndocal product requirement |
| --- | --- | --- | --- |
| V01 | Empty project | Import 12 MP4 files into a reusable library | One Import or one drop; files do not become 12 unrelated layers; truthful per-file result |
| V02 | Populated library | Assign three existing assets to Layer 1 slots | Drag or direct assign, no file dialog, no live replacement |
| V03 | Four layers | Add Layer 5 and duplicate Layer 2 settings without duplicating media bytes | One contextual layer action; stable layer ids and reusable assets |
| V04 | Layer 1 has four slots | Queue slot 4 and Take on the next bar | Active/queued truth, one dominant Take, deterministic late-trigger policy |
| V05 | Layer 1 active | Add Blur, RGB Shift, and custom ISF; reorder and bypass RGB Shift | Ordered Layer FX chain, one Add Effect action, no node graph required |
| V06 | Clip slot selected | Apply an effect only to that slot | Clip FX remains with slot; Layer FX remains unchanged |
| V07 | Layers 1/2 visible, logo Layer 4 visible | Crossfade Layer 1 to 2 over one bar | Transition Bus affects only members; logo remains visible |
| V08 | Same | Change crossfade to Luma Wipe with a matte and Glow transition FX | Preset plus contextual advanced disclosure; no manual opacity graph |
| V09 | 120 BPM | Change to 128 BPM during a queued bar transition | Phase/timing policy is visible and deterministic; no jump or double Take |
| V10 | Audio input active | Bind bass energy to Layer 3 scale and onset to slot advance | Clear Armed/Running/Hold/Fault; bounded loss-to-safe state |
| V11 | Two compositions | Route different compositions to HDMI Output 1 and 2 | Physical monitor identity, resolution, live/blackout truth visible together |
| V12 | Output 2 mapped | Correct keystone/crop/mask without disturbing Output 1 | Mapping remains output-scoped; immediate preview and reversible edit |
| V13 | One media file removed | Diagnose and relink the missing asset | Missing tile remains identifiable; hash verification; all referencing slots recover |
| V14 | Live show | Map one Take, one layer opacity, one FX bypass via MIDI/OSC/DMX | Stable target ids, learn feedback, conflict visibility, undo/remove mapping |
| V15 | Live show | Record the two-output program with audio and stop safely | Obvious record truth, drop telemetry, finalized playable artifact |
| V16 | Populated/full bank | Import more media at 1920, 1366, and 1280 widths | Exactly one reachable import route; no outer scroll, overlap, or lost slots |
| V17 | Effect or decoder failure | Keep valid layers/output alive and recover | Fault is scoped and truthful; explicit retry; no false Ready state |

## SynapseRack reference boundary

The pinned SynapseRack build and license tier must be recorded before measurement. Its official product surface currently advertises layer compositing, more than 100 effects, nodes, MIDI/OSC/DMX, Audio/Tempo, grouping, four-layer Demo, and unlimited-layer Pro. Its tutorial media flow uses a Source directory and ContentsBrowser, and its two-layer example mixes by opacity faders. These facts define comparison tasks but do not pre-judge measured results.

Syndocal's intended advantages to test rather than assume:

- first-class MediaAsset identity/relink instead of path-only placement;
- standard Clip Take and scoped Layer Transition Bus without requiring a node graph for common transitions;
- unified Lighting/Video Timeline and ShowClock;
- explicit Preview/Program and dual-HDMI truth;
- one project, one clock, one undo/save/recovery model for lighting and video;
- advanced node/effect extensibility behind progressive disclosure rather than as the prerequisite for routine operation.

## Evidence package

Each release candidate stores:

- pinned build/version/license and machine configuration;
- task start project/hash and media manifest/hash;
- screen recording with timestamps and input overlay;
- completed measurement table and operator/date;
- Syndocal telemetry and output-drop evidence;
- failures and retests, without deleting unfavorable first-run results.
