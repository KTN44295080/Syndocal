# DVC Touch Action 223 — Feature Preset parity

Validated 2026-08-09 against Daslight 5 with
`C:\Users\kouty\Desktop\homecoming2026\homecoming2606-Laser.dvc`.
Daslight remained maximized throughout the interactive comparison.

## Daslight evidence

The Touch Mappings window identifies both imported vertical faders as
`Feature Preset` actions over the same 39-beam selection:

| Touch control | Daslight target | Grid | DMX Levels at 100% |
| --- | --- | --- | --- |
| `Dimmer` | `Generic: Dimmer (39 Beam(s))` / DVC target `:-1:4` | `(0,0,1,4)` | U1: 1, 14, 173, 178, 183, 188 = 255 |
| `Dimmer linear` | `MEGA BAR RGBA: Dimmer Dimmer linear (39 Beam(s))` / DVC target `69bdd010-d626-11ea-b9df-7da99bfefe5c-3afb4fbb:33:0` | `(0,4,1,4)` | U1: 66, 100, 134, 168 = 255 |

This proves that Action 223 must not be widened to a group-wide `Dimmer`
write. The generic preset excludes the four Mega Bar RGBA master dimmers;
the profile-specific preset addresses only those four channels.

## Syndocal representation

`TouchControlBinding::FeaturePreset` persists:

- the exact ordered fixture/attribute target set;
- normalized minimum and maximum values;
- inversion;
- the original Touch fader layout.

The UI sends one batch command per fader update. The engine resolves every
target before mutation, so a missing fixture or attribute rejects the entire
batch without a partial DMX update. Imported Feature Presets remain visible in
Touch EDIT with their target count and preservation note.

## Regression gates

- `cargo test -p syndocal dvc_local_homecoming_laser_touch_faders_map_verified_feature_presets_when_present --offline`
  imports the real local `.dvc`, validates both bindings, drives the engine,
  and asserts the two captured DMX address sets above.
- `cargo test -p engine fixture_attribute_batch --offline` proves exact-target
  application and rejection without partial mutation.
- `cargo test -p syndocal project_touch_feature_preset_validates_exact_fixture_attributes --offline`
  proves `.sdc` validation.
- `cargo test -p protocol touch_surface_roundtrips_all_binding_variants --offline`
  proves persistence round-trip.
- `pnpm --dir app run check:touch` includes a 10-assertion static contract and
  the Touch viewport matrix at 1920x1080, native-like 1920x1032, 2048x1152,
  1366x768, and 1280x720 for both default and composed surfaces.
- `pnpm --dir app run check:localization` remains 100%.

## Remaining boundary

The known Action 223 forms in this project are exact. Unknown generic Feature
Preset IDs or ambiguous profiles with multiple canonical Dimmer presets remain
truthfully `Unsupported`; the importer does not guess their channel mapping.
