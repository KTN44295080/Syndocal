# VJ Operator Desk Acceptance

Updated: 2026-07-14

## Scope

This gate fixes the live operator contract for a populated VJ show. It is separate from the empty-show first-run gate, native window/F11 acceptance, video decode benchmarks, and hardware output acceptance.

The fixture contains:

- seven 1080p/60 video layers, shown as a six-layer live bank plus a one-layer terminal bank;
- mixed layer FX states: enabled, bypassed, no effect, parameterized and control-free;
- a two-control Threshold effect on layer 1;
- three routed outputs: `Main LED` (`Live`), `Side Projection` (`Off`) and `Stream Fill` (`BO`).

## Operator contract

- Every visible compact layer keeps source state, opacity/transport actions, built-in FX selection and Enable/Bypass/Advanced actions on the live surface.
- Deep ISF import, reset, description and parameter controls are not mounted while Advanced is closed. Initial Advanced DOM count is zero; opening layer 1 mounts one editor with two controls; closing it returns the count to zero.
- Enable/Bypass uses the normal project transaction boundary. The test requires begin, mutation, commit, refreshed snapshot, `aria-pressed` and visible state to agree.
- The output rail keeps every output and its `Live`/`Off`/`BO` state visible. Selecting a rail item must synchronize the selected rail item, the only visible output detail card, the Program monitor label and the Program monitor request `outputId`.
- Internal scroll containers may scroll vertically where intended, but hidden overflow and action/control rectangles outside their owning pane are rejected.

## Automated acceptance

Run:

```powershell
pnpm --dir app run check:vj-operator
```

The gate runs English and Japanese at 1920x1080, 1366x768 and 1280x720. On 2026-07-14 all six cases passed with, at each measured phase:

- unsafe internal overflow: `0`;
- operator/control rectangles outside their owner: `0`;
- layer bank transition: `1-6 / 7` to `7-7 / 7`;
- Advanced lifecycle: `0` editors to `1` editor / `2` controls to `0` editors;
- Bypass snapshot refreshes: `1`, with the stored and published layer effect disabled;
- output synchronization: initial output 1, rail-selected output 3, detail `Stream Fill`, Program label `Stream Fill` and Program request `outputId = 3`.

Setting `SYNDOCAL_VIEWPORT_SCREENSHOT_DIR` captures the initial and expanded operator states for visual review. The 1920x1080 capture is the design review surface; 1366x768 and 1280x720 are containment fallbacks.

The first element-level run found a real approximately four-pixel overflow in the compact Advanced action row at fallback widths. Compact button and parameter grids were constrained, then all six cases passed.

The same current source was then rebuilt as the separate `Syndocal QA - F11 Focus` ASIO-capable native target. Its maximized Windows work-area capture was 1920x1032 with the normal `VIDEO CONTROL` header. F11 produced an origin-0 exact 1920x1080 surface without that duplicate header; Escape restored 1920x1032 and the header. The native run used an empty project, so populated Layer/Output behavior remains the browser stateful evidence above rather than a claimed native media test.

## Claim boundary

This proves the compact operator interaction, state synchronization, lazy editor DOM budget and containment contract. It does not prove native output-frame latency, GPU effect-stack breadth, 4K multi-layer endurance, TouchDesigner/SynapseRack performance parity, or venue usability. Those remain governed by the competitive audits and hardware acceptance documents.
