# Codex handoff: post-unification continuation

Date: 2026-08-11. Direction: Fable (supervisor) -> Codex. The user asked for a single
consolidated handoff so Codex can carry the queue forward. Supervisor-exclusive items
that Codex cannot perform are listed at the end and remain with Fable/the user.

## Checkpoint

- Branch `codex/syndocal-v1.0`, HEAD `ab160ed feat(effects): unify spatial evaluator domains`,
  pushed to origin (`f4b386b..ab160ed`).
- `5198610` (corrected DVC compatibility) and `ab160ed` (dual-route unification) are both
  supervisor-verified on the unified HEAD: cargo fmt / engine full / syndocal 409 /
  `cargo test --workspace --all-targets` / release gate p95=2.118 ms (max-parameter Sparkle
  rows + 254-lane Knight Rider row, immutable 5/8/12 ms) / tsc / check:value-generator /
  check:fx-visual / check:fx-color-palettes / check:localization 100% / vite build /
  full viewport matrix exit 0 (NEW baseline: pass 208 after the harness rework in ab160ed;
  judge by exit code + zero `^fail|^FAIL` lines + zero nonzero category-summary counts) /
  native acceptance (HEAD-rebuilt exe, exactly one responsive `Tauri Window` "Syndocal",
  zero leftover processes).

## In-flight Codex job (finish or take over FIRST)

`task-msngu33n-z1vdzl` — "#11: release-gate representativeness + supported envelope".
Prompt file content is authoritative; scope summary:

1. Add validated-maxima 44 Hz release-gate rows for Chaser (256 steps x 16 features x
   16 wings BuildUpDown), Curve (32 control points), Smooth Move, LFO / Position Wave.
   Thresholds 5/8/12 ms immutable; optimize evaluators if needed (e.g. Curve segment
   lookup -> `partition_point`), never weaken gates.
2. Supported-envelope policy (user ruling: NO hard cap): envelope constants (64 enabled
   effects x 200 fixtures at validated per-effect maxima) as single source of truth,
   engine telemetry fields (enabled-effect count + `effects_over_supported_envelope`),
   frontend telemetry-panel warning line with JA localization, validation stays permissive,
   tests coupling the constants to the gate shape.

Its in-progress uncommitted edits at handoff time: `app/src-tauri/src/main.rs`,
`app/src/components/EngineTelemetryPanel.tsx`, `app/src/initialEngineSnapshot.ts`,
`app/scripts/check-localization.mjs` (+ engine files as it proceeds).

Uncommitted files NOT belonging to that job (supervisor work, commit together with or
separately from #11 as coherent units):

- `RELEASE_STATUS.md` — evidence paragraph for the two landed tranches (2026-08-10〜11).
- `qa/DVC_CURVE_SOURCE_PARITY.md` — +68-line real-specimen section (see below).
- `qa/specimens/CurveCatalog-Unrouted.dvc` — new specimen (untracked).
- `qa/CODEX_CURVE_TRANCHE_DRAFT_2026-08-11.md`, this handoff file (untracked).

## Session rulings registry (binding, from the user)

1. Corrected Sparkle: SINGLE VISUAL CLOCK (spawn + lifetime in visual ms) and
   AUTHORED PERIOD = RANDOM-LOOP LENGTH (seed wrap at `max(1, floor(period/40))`,
   absolute epoch for identity/expiry). Implemented in `5198610`.
2. Dual evaluator routes abolished ("DasLight特有の挙動は不要"): one analytic evaluator
   per family, Syndocal-native domains, import-time conversion, load-time migration.
   Implemented in `ab160ed`. New IDs land directly in this regime.
3. Effect-stack cardinality: envelope guarantee + warning, no hard cap (job #11).
4. Division of labor: Codex implements code; Fable does analysis/docs/spec drafting/
   verification/rulings/commits/native GUI. Reviews are never skipped and never
   downgraded.
5. Model tiering for Codex jobs: default `--model gpt-5.6-luna --effort xhigh`; only
   genuinely hard tasks (state machines, concurrency, perf-critical, large design
   discretion) use Sol. Effort below high is never allowed. Claude side may freely use
   Opus for reviews/heavy assists, Sonnet for light lanes.

## Verification discipline (supervisor contract — reproduce before any commit)

- Rust: `cargo fmt --all -- --check`; `cargo test -p engine --locked`;
  `cargo test -p syndocal --locked`; every release gate `--release -- --nocapture`
  (record p95/p99/max; 5/8/12 ms immutable); `cargo test --workspace --all-targets --locked`;
  `git diff --check` (CRLF warnings acceptable).
- Frontend (direct node, never npm --silent): tsc --noEmit; check-value-effect-generator;
  check-effect-visualization; check-fx-color-palettes; check-localization (100% required);
  vite build (in Codex sandbox add `--configLoader runner`).
- Full viewport matrix from `app/`: judge by exit code, `^fail|^FAIL` grep, category
  summary counts, pass count (baseline 208).
- Adversarial review loop before commit for any nontrivial engine change: independent
  reviewer tries to refute the implementation claims; this loop caught two real defects
  (Sparkle clock asymmetry; palette-shrink migration panic) in the last tranche.

## Work queue (in order)

1. Finish #11, verify with the discipline above, commit (with the supervisor files
   above as appropriate), push.
2. DVC-CURVE unrouted IDs tranche. Delegation prompt draft:
   `qa/CODEX_CURVE_TRANCHE_DRAFT_2026-08-11.md` (review/trim before sending; classified
   HARD -> Sol). Ground truth: specimen `qa/specimens/CurveCatalog-Unrouted.dvc` with
   confirmed mapping Ramp=5, Random=6, Sinus3=8, Tangeant=11, Triangle=12 (document
   order [8,11,12,5,6]); all five share `PARAMS NB=5` (Rate int 1..10 = 2, Size f 0..2 = 1,
   Phase f 0..1 = 0, Offset f -1..1 = 0, Phasing f 0..1 = 0), DURATION=5000, concrete
   Dimmer channel PRESET (SSLCHANNEL=33) + 32-beam IDSELECTION. Evaluator formulas are
   NOT yet recovered — decompile `CSinus3Effect` (evaluator 0x140370070) and
   `CTangeantEffect` (0x140370760) etc. from the pinned Daslight binary before coding;
   prefer LfoShape reuse for Ramp/Random/Triangle after verification.
3. CHASER 323/324 (distinct evaluator semantics unproven — static recovery first).
4. MAPPINGS 522-529 / COLOR MAPPINGS 2D group (per roadmap order; Media/Text embeds
   reuse the Colour Mapping base).
5. CURVE Custom (ID 13, separate schema): specimen NOT yet captured — needs the native
   point-list editor, i.e. a Fable capture session.

Roadmap of record: `qa/DVC_FULL_FX_PARITY_ROADMAP.md`. Acceptance per tranche: static
proof doc, specimen/golden cross-check, `.sdc` legacy regression, 44 Hz gate
non-regression, Scene Settings editor exposure with JA localization.

## Codex-incapable items (stay with Fable / the user)

- Native window acceptance (`pnpm --dir app tauri build --no-bundle` + single responsive
  window check) — sandbox cannot launch GUI. Never spin on it; report and stop.
- Daslight native specimen capture (e.g. CURVE Custom 13) — same reason.
- Final commit/push authority sits with the supervisor flow unless the user says
  otherwise for a given tranche.
