# Pre-rendered VJ workflow walkthrough — 2026-11-10

Owner's target use (not generative): import clips as a library, load them into
layers, switch between them, per-layer FX, compositing, BPM sync. Claude drove
isolated native QA builds over loopback CDP; output on the owner's second
display (`\\.\DISPLAY2`, 1920×1200). Evidence: `target/qa/vj-walk-20261110/`.

## Material

`target/qa/vj-clips-20261110/make-clips.sh` (bundled-style LGPL FFmpeg,
`h264_amf`): eight 1920×1080 30 fps H.264 clips, 120 BPM × 4 bars = 8 s.
Six are seamless by construction (expressions periodic in 8 s); `07_life`
(Game of Life) and `08_mandel_zoom` are not loops.

## Steps and results (builds `90d3543f`, `3ee44213`)

| Step | Result |
| --- | --- |
| Media Library > Import, native multi-select of 8 files | PASS — `Media library: 8 imported.`, thumbnails |
| Seamless loop on `07_life` (new, `90d3543f`) | PASS — 3.7 s, `07_life_seamless_1000ms` imported beside the source; last/first frames continuous (`vj-clips-20261110/seam.png`) |
| Library clip → layer | **Gap**: library import never creates layers and the Perform first-run picker disappears once the catalog is non-empty; Perform only said "Add one with Import Media". Fixed in `3ee44213` (card **Add layer**). PASS on the fix build: 3 layers |
| Add a clip to a layer's empty slot | Works via Inspector > Clip > Media asset + **Create Slot**. Friction: an empty slot's `+` only selects ("Queue clip 2: Empty"); it does not offer a clip |
| Display output on DISPLAY2 | PASS — `Display output added`, `Display #1 Output 59.9fps`, no confirmation dialog |
| Queue + TAKE | PASS — layer LIVE, Program and DISPLAY2 show it |
| Blend Screen / Add on layers 2 / 3 | PASS — composite visible on DISPLAY2 |
| Kaleidoscope Add on layer 2 (Browser: Try = Preview, Add = LIVE) | PASS — DISPLAY2 shows the kaleidoscoped rings |
| Clip end | **Bug for VJ use**: a new slot was `Once`; the clip stopped after 8 s. Fixed in `8c94ee77`: new video-file slots loop (stills/inputs keep `Once`; stored projects unchanged) |

## Open findings

1. **Unlaunched layers are on Program.** A layer added from the library shows
   its first frame at 100 % opacity on Program/DISPLAY2 before any clip is
   launched (a paused top layer covered the mix). In a running show, adding a
   layer changes the venue output immediately. Design decision for the owner:
   start new layers hidden (opacity 0 or bypassed) or keep the current
   "a layer always shows its default slot" model.
2. Empty slot `+` should offer a library clip (or accept a drag from the
   library); today it is Create Slot in the Inspector, or a native file drop.
3. **Slot settings cannot be saved from Perform** (build `8c94ee77`): clicking
   a slot selects *and queues* it, and Inspector > Clip > Save Clip is then
   rejected ("Video clip slot 2 on layer 1 is active, queued, pending, or
   transitioning; authored mutation is rejected while it participates in live
   program state"), also after Stop. So launch quantization (NextBar) could
   not be set on a slot from the Perform screen.
4. TAKE after queueing slot 2 reported `Launched video clip slot default.` and
   Program then read `Waiting for a program source.`; which slot launched is
   not established.
5. Loop default fix verified natively on `8c94ee77`: the Add-layer slot and a
   Create Slot slot both read `Loop`.
6. Not yet walked: quantized launch timing, BPM-synced loop length,
   tempo-synced FX parameters, Auto VJ, crossfade/transition takes.
