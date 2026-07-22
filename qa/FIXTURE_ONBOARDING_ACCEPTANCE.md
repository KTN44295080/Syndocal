# T21 Fixture Onboarding Acceptance

- Date: 2026-07-23
- Baseline: `db12f12`
- Scope: GDTF Share search, favorites, offline cache and health, missing-profile repair, verified common-rig profiles
- Product boundary: Syndocal does not build an internal 3D visualizer. Representative output remains a T23 Art-Net external-visualizer gate.

## Result

**T21 software PASS.** The Setup / Library surface now provides one fixture-onboarding workflow across online GDTF Share revisions, machine-local offline profiles, project-profile health and a bundled generic common-rig pack. Live GDTF Share authentication, native WebView2 use with a real account and physical/external-visualizer output are deliberately not counted as completed evidence here.

## Implemented contract

### Online catalog

- Structured Share search supports global query, manufacturer, fixture, mode and DMX-footprint bands from the documented public list. Release-only, tested-in-Visualizer and tested-in-real-life filters apply only when the service returns that optional metadata; unknown values fail closed and the UI discloses unavailable filter metadata.
- Search results expose available modes, revision state and test badges. Favorites can filter online, cached and verified results.
- An already valid cached revision loads directly without another authenticated download. An invalid cached file is not presented as usable and can be replaced only after a newly downloaded `.gdtf` validates.
- The UI follows the public GDTF Share navigation model rather than claiming to mirror a proprietary commercial fixture count. The website exposes release/test filters, while the documented public `getList` response guarantees revision identity, fixture/manufacturer, version and mode/footprint but not release/test fields. Downloads use the documented revision-ID GET endpoint. References: <https://gdtf-share.com/help/users/gdtf_share/navigate/index.html> and <https://github.com/mvrdevelopment/tools/blob/main/GDTF_Share_API/GDTF%20Share%20API.md>.

### Credential and persistence boundary

- Share user and password values exist only in component memory. Inputs disable browser autocomplete.
- Credentials are absent from the project, favorites, cache filenames and cache sidecar schema.
- Favorites use bounded machine-local frontend storage. They are operator convenience state, not show state.
- Downloaded GDTF files and identity-only JSON sidecars use Tauri's application-local `fixture-profile-cache`. A `.partial` download is parsed and validated before replacing a cache entry.
- T21 adds no `.sdc` schema field, so an untouched legacy project has no serialization-shape change from this tranche.

### Health and repair

- Project profiles report `healthy`, `warnings`, `embedded`, `fallback` or `missing` status. Offline cache entries report healthy/warnings/invalid with parse detail and mode summaries.
- Repair is an explicit project mutation for one patched fixture. It requires a case-insensitive manufacturer/profile identity match and an exact mode layout match: attribute, geometry, DMX offsets, control order and resolution.
- A successful repair retains the fixture ID, patch address, authored values, limits, groups and Cue references, then rebuilds command-time target caches. Layout mismatch fails closed without partial mutation.
- Catalog and repair work add no per-tick lookup, allocation, string normalization or solver to the 44 Hz evaluator path.

### Verified common-rig pack

The built-in pack contains four deterministic generic structural profiles:

1. Generic Dimmer 1ch
2. Generic RGB PAR 4ch
3. Generic RGBW PAR 5ch
4. Generic Moving Head RGBW 10ch with 16-bit pan/tilt and canonical 1-25 Hz strobe metadata

These are labelled `Syndocal Verified`, not manufacturer-specific profiles. The operator must compare the selected layout with the fixture manual before patching real hardware.

## Automated evidence

- Full Rust workspace: green.
- Engine: 400 passed, 1 manual release benchmark ignored. The new repair test fixes identity/value retention and fail-closed layout mismatch.
- Tauri: 330 passed, 9 hardware/tool-dependent ignored. New tests cover faceted Share extraction/filtering, credential-free cache metadata, bounded stable paths, valid/invalid offline health, project fallback, verified-pack layouts and repair compatibility.
- Fixture catalog helper gate: 29 assertions passed.
- TypeScript: `tsc --noEmit` green.
- Localization: 2766/2766 static Japanese strings; zero unprotected user-data labels.
- Terminology and project-storage helper gates: green.
- Focused fixture-catalog viewport: 5/5 at 1920x1080, 1920x1032, 2048x1152, 1366x768 and 1280x720. Each has four verified profiles, four catalog sections, five facet groups, working favorite persistence and zero outer scroll.
- Full viewport matrix: green at all five sizes, including the three-pane Setup / Library contract and all pre-existing application surfaces.
- Production frontend build: green with a separate 14.85 kB fixture-catalog chunk; main chunk 490.79 kB, CSS 470.20 kB.

## T23 external evidence still open

- Sign in to the live GDTF Share service with user-provided credentials and prove search, download, restart and offline reload. No credentials were available during T21 automation.
- Use native WebView2 with a real project to replace one intentionally missing compatible profile, save, reopen and verify fixture/Cue identity.
- Patch representative dimmer, RGB/RGBW and moving-head profiles and inspect DMX through the official Art-Net external-visualizer path.
- Test a real fixture/node when hardware is available. Browser fixtures and synthetic Art-Net loopback do not substitute for this evidence.
