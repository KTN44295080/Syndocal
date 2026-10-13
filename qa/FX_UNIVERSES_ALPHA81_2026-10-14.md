# FX scrolling and multiple network universes — alpha.81

Checkout main; base e7ce813f08f10748638f9c668513b73ef74d7884. This checkpoint owns the listed runtime, UI and package changes. Old-worktree consolidation additionally integrates the secure PIN generator and preserves historical MIDI observations; see WORKTREE_CONSOLIDATION_2026-10-14.md. Delivered alpha.80 bytes remain immutable.

The FX editor previously collapsed to 14 px inside its scroll owner at 1366×768 despite about 2154 px of content. The form now contributes its intrinsic height to the existing inspector scroller. A short guide describes family, fixtures, attribute and Save. Typography and targets retain their sizes.

Setup I/O now edits additional Art-Net/sACN destinations with five routes per page, explicit Apply, validation, and stale-project/draft-conflict rejection. The built-in enabled U0 route must be prepared first. USB-DMX remains the dedicated single-universe U0 device; additional universes require network nodes. Additional routes do not replace primary or USB worker ownership. Persisted network route configurations use the existing authored route storage.

The new canonical OutputControl action keeps project/output/S0 fences, Both lease ownership, bounded socket staging, expected-route comparison and snapshot acknowledgement. Failed publication restores the exact prior sender owners. Retired serial setters remain rejected. Configured-network retirement zeros and retires all added network senders and the verified USB worker; ExactShow retirement retains its narrower contract. The USB primary-mirror hot path stays constant time and adds no allocations.

## Evidence

- Browser FX: five viewport sizes × five actual families (25 cases); wheel and End reach Save through the existing scroller. Setup I/O containment: five viewports, one operator surface and one scroll owner. Network editor: five viewports, exact Apply, duplicate rejection, dirty external conflict, project replacement and 13-route pagination.
- Engine: three network-route tests include real private loopback Art-Net U0/U1 and sACN U1/U2 packets carrying independent 64/192 values, stale/expired/S0 rejection, and sender-preserving rollback. Two mixed USB/network retirement tests use recorded USB transport plus private UDP; 23 prior managed-retirement and 27 prior USB tests pass. These tests do not open physical devices.
- Admission delta: exactly one native command and one EngineCommand; frontend manifest 487, native routes 567, engine variants 284. New action is discriminator 26 with strict protocol payload validation; legacy paths are unchanged. Strict inventory assertions now explicitly require the new route.
- Final release aggregate and Japanese localization (3915/3915) pass. Protocol: 265 tests; native registry/authority tests: 42 pass. Windows native no-bundle build passes with zero first-party compiler warnings; the existing Vite chunk advisory remains one. The initial and post-PIN native window gates pass, including exact path/hash, one maximized visible responsive window, unauthenticated broker rejection and graceful close. The full release aggregate passes again after PIN integration. Final packaging and hosted lifecycle acceptance remain pending. No new package has been delivered yet.

## Boundaries

Physical nodes/USB, MIDI timecode devices and Mac GUI are unobserved. MTC quarter-frame/full-frame reception exists in current source; audio LTC decoding is not implemented (the LTC position-injection API is not an audio receiver). Multiple physical USB interfaces are outside this bounded change. First-party warning baseline Windows 0, Mac tests 29 / release 82; Windows current/delta 0/0; Mac current measurements remain pending. Developer ID/notarization is absent. This is not full Recording, beta/RC, physical-show or complete-ledger acceptance.

## Main integration

The preceding validated checkpoint was integrated into main and 11 fully merged unused local/remote branches were deleted. Active checkouts and unmerged branches were retained. Current work reuses main; a daily 09:00 heartbeat performs validated integration/merged-branch cleanup and remains quiet when unchanged.
