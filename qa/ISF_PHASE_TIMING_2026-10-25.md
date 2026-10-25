# ISF synchronous stack phases, 2026-10-25

Base: `da7d79dd46d1ae04eec559be91581ca4f753de05`, shared `main`.
This adds an opt-in GPU experiment and host phase boundaries compiled only under
`cfg(test)`. The new module, clocks, stored timings and adapter information are
absent from product builds. Removing exactly these guarded additions makes the
two existing source files text-identical to the base after CRLF normalization
(`target/qa/isf-phases-20261025/test-only-proof.json`). Independent source review
also checked every guard. No product backend, queue, feedback, failure,
LastValid, cancellation or output policy changes in this checkpoint.

The experiment runs the same targetless, control-free `rgb * 0.99` shader as
the known native fixture, with synthetic opaque RGBA pixels at 2560 x 1600,
three layers and three stages per layer. It invokes the canonical stage-local
runtime. Sequence increments prove one synchronous GPU stack per layer.
Pixel equality and complete phase coverage are checked; logging happens after
all measurements. Clock overhead is not independently quantified.

## Default-backend phase measurement

Evidence: `target/qa/isf-phases-20261025/phase-01.log/json` and
`reviewer/phase-01-analysis.json`. All 72 stack records (24 rounds x three
layers) are retained. The first four whole rounds are warmup; the accepted
population is **20 complete three-layer rounds / 60 stacks**, with no other
exclusion. The adapter reported AMD Radeon 8060S Graphics, integrated GPU,
Vulkan, AMD driver 26.10.32.01 (LLPC).

Each duration is summed across the three calls of its own round before sorting.
P50 selects the 11th of 20 sorted values and p95 the 19th. Table rows have their
own distributions: do not sum or subtract the printed percentiles.

| Host boundary, three stacks | p50 ms | p95 ms |
| --- | ---: | ---: |
| Enclosing stage-local call walls | 11.7424 | 13.0508 |
| Sum of all recorded phases | 11.7369 | 13.0445 |
| Push GPU error scopes | 0.0006 | 0.0009 |
| Validate/convert/workspace selection | 0.0027 | 0.0040 |
| Input upload call | 1.7961 | 2.1759 |
| Encode, including final texture-to-buffer copy encoding | 0.4131 | 0.6078 |
| Encoder finish and queue submit | 0.2974 | 0.3750 |
| Map registration, device poll/wait, callback/channel receive | 4.4256 | 5.2868 |
| Mapped range, CPU packed copy and unmap | 4.8769 | 5.6221 |
| Feedback bookkeeping | 0.0008 | 0.0011 |
| Inner return/epilogue and error-scope completion | 0.0066 | 0.0115 |
| Matched per-round enclosing-minus-phases residual | 0.0047 | 0.0075 |

The final phase is named `pop_error_scopes` in raw records; it also contains
the inner return/epilogue. The wait boundary includes outstanding GPU work and
host synchronization; it is not GPU timestamp/kernel time. These calls exclude
decoder scheduling, chain preparation, queue publication, CPU composition,
LastValid and presentation. The three-call sum is not a native Display parent
measurement. No native FPS improvement or complete frame-time budget is inferred.

## Default versus DX12 experiment

`backend-01.log/json` retains **144 calls / 48 three-layer sweeps**. The order
alternates ABBA and BAAB over 12 rounds; discard two complete warmup rounds,
leaving **20 sweeps per backend**. Both runtimes coexist, use the same pixels,
shader and runtime path, and compare output bytes exactly. Test-only DX12
initialization mirrors product initialization except for its backend mask.
Both adapters reported the 8060S model with matching vendor/device IDs. This
check is not a universal unique-physical-adapter identity guarantee.

| Three-layer host measurement | Default Vulkan p50 / p95 ms | DX12 p50 / p95 ms |
| --- | ---: | ---: |
| Enclosing call walls | 12.1830 / 13.0969 | 13.0273 / 13.5060 |
| Recorded phase totals | 12.1793 / 13.0918 | 13.0246 / 13.5010 |
| Map/poll/wait/receive | 4.7281 / 5.5592 | 5.3462 / 5.8900 |
| Mapped copy/unmap | 5.0617 / 5.5346 | 5.2079 / 5.4876 |

`reviewer/backend-01-analysis.json` independently recomputes all raw records,
orders, sequences, complete sweeps and 18 printed phase summaries. This bounded
synthetic comparison provides **no basis to adopt DX12**. Product selection
remains unchanged. Warm caches, coexisting runtimes and host observation limit
generalization; these values are not a native before/after comparison.

## Verification and next boundary

All commands ran with pinned MSVC **14.44.35207**, absolute linker first on
PATH, `cargo test -p video --features libav --release`, one test thread.

| Selected filter | Passed | Failed | Ignored | Recorded exit |
| --- | ---: | ---: | ---: | ---: |
| `synchronous_stack_phase_timing_experiment --include-ignored --nocapture` | 1 | 0 | 0 | 0 |
| `isf_` | 76 | 0 | 3 | 0 |
| `synchronous_stack_backend_timing_experiment --include-ignored --nocapture` | 1 | 0 | 0 | 0 |

The ISF regression run includes stage faults, feedback, workspace recovery,
borrowed inputs, scope isolation, dependency order and cancellation. Its three
ignored cases were explicit timing experiments. The backend experiment was
appended afterward; it compiles as another ignored case and was explicitly run
by the third command. No original test or runtime behavior changed between runs.
The initial module SHA256 is `6e113f81...ce05c85`, preserved with both source
files in `phase-01-source/`; the appended module is `f758dc5f...549f13d`.
The two observed runtime files remained `a8791dc1...d7f2510` and
`60d73151...8cb210a` across all three commands. Before/after source hashes and
exit records are retained. Rust warnings: baseline **0**, current **0**, delta
**0** for these configurations; no new Vite/native build was performed.

Independent reviewer `review_readback_copy` found no source or measurement
blocker and recomputed the 72- and 144-record populations. No native gate was
repeated for this test-only addition; native product behavior is unchanged.
The previous composed native observation remains **32.411 fps, 600/600 misses**
as documented in [the row checkpoint](CPU_CONTIGUOUS_ROW_BLEND_2026-10-25.md).
The 60 fps goal remains unmet. Normal runtime files are preserved.

Steroids' read-only investigation is retained in
`target/qa/steroids-isf-next-20261025/INVESTIGATION.md`, bound to the base above.
It identifies a bounded batch of independent resource-neutral chains as the
smaller wait-reduction candidate. It must own separate pending scratch, account
aggregate bytes, preserve logical FRAMEINDEX and ordered failure/cancellation
recovery, and leave CPU LastValid and dependency barriers intact. GPU-resident
handoff is larger because ISF and presentation use separate devices and the
current GPU-success path drops CPU LastValid. Simply relaxing FX exclusion is
incorrect. Current measurements also show mapped copy is substantial; batching
alone does not remove that cost. The next implementation must measure its
specific reduction and retain native Display/Stop acceptance, rather than
claiming the entire wait or copy duration as removable. No new runtime change
is adopted here, and no unknown devices or external outputs were exercised.
