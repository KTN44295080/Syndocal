# Mapping orientation browser checker repair — 2026-09-09

## Bounded repair

The checker had stale browser discovery and translation assumptions. It now
honors explicit/per-user Chrome and Edge paths. Its fixture locators also use
the current component source contract: the axis action is
`Aim installation axis (+Z) at target` and the inputs are `Target X`, `Target Y`
and `Target Z`. The product component and its assertions were not changed.

- Source before this checkpoint: `main` at `4f8e71f4f156b3b302a2f0ea1abd19b45cc8b267`.
- `node --check app/scripts/check-mapping-orientation-browser.mjs`: PASS
- `node app/scripts/check-mapping-orientation-browser.mjs`: PASS in the
  configured per-user Chrome.
- Evidence covers 1280/640 viewports, axis inputs, keyboard action, busy
  exclusion, selection availability, and control sizing/containment.
- `git diff --check`: PASS

This is a checker-only repair. It does not claim native, hardware, macOS,
venue or product-wide completion.
