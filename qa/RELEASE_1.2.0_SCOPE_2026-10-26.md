# 1.2.0 release scope — owner contract, 2026-10-26

Owner: product owner (user), decisions 2026-10-25/26; recorded by task46/task46b
on shared `main`, base `7b760b15eb71b9be5fa8e435ff2942ced49fca9b`.
This is the authoritative 1.2.0 scope decision page, not release acceptance.
No row becomes Complete in this checkpoint. The completion flow sections 2/4/6–9/11,
completion ledger and master Q1–Q5 mirror carry the same bounded contract.

| Decision | Date | Effect on stable ledger IDs | Remaining item / removed claim |
| --- | --- | --- | --- |
| Completion **2026-11-12**, Windows only | 10/26 | `WARN-MACOS-001`, `PLATFORM-MAC-LINUX-001` stay Deferred | macOS/Linux acceptance; no cross-platform release claim |
| Freeze **unpublished final-version 1.2.0 exact bytes by 11/1**; publish only after all required gates | 10/26 | `RELEASE-METADATA-GATE-001` stays historical software Complete; `DIST-PUBLICATION-001` reopens Open; Q2 `DEC-RELEASE-SCOPE-001` records approval | Explicit exception to flow §4 stable-suffix order: test publishable bytes during 11/2–8, avoid 11/9 RC-to-stable rebuild invalidating physical evidence. No premature completion/publication |
| Separate optional GPLv3 ASIO component, bridge GPL-3.0-only; GPL-ASIO disables NDI load/use; MIT/WASAPI separate | 10/25–26 | `ASIO-LICENSE-001` and all other ASIO rows stay Open; Q2 `DEC-ASIO-001` policy Accepted | Corresponding source/notices, packaging, profile rejection and real-device acceptance pending. Owner signed/emailed proprietary SDK agreement **10/25; Steinberg countersignature pending, not approval**. No approval flag changed |
| NSIS only | 10/26 | `DIST-PLATFORM-PACKAGE-001` reopens Open; split `DIST-MSI-PORTABLE-001` Deferred | MSI/portable supported install/update claims removed; each offered normal/GPL-ASIO profile needs exact acceptance |
| OpenDMX supported; Enttec USB PRO / DMXKing unsupported | 10/26 | `DMX-USB-RDM-001` retains ID, Open for OpenDMX; split `DMX-USB-PRO-001` Deferred | PRO/DMXKing product-output support; OpenDMX long-run/payload/Blackout/reconnect/ownership safety remains required |
| DJ-Link experimental, OFF by default, minimal actual-peer acceptance | 10/26 | `DJ-LINK-HARDWARE-001` stays Open; split `DJ-LINK-FULL-MATRIX-001` Deferred | Full HW4 twelve-case/advanced production integration; minimum retains restart OFF/no listener/output, opt-in/pair, wrong/stale/replay and non-playing rejection, mapped play/Release with ACK/idempotence, disconnect/local operation, reconnect StateSync and disable/credential handling (HW-AUTOMATION-01 §4.4) |
| Two-PC manual failover only | 10/26 | `SHOWCLOCK-VENUE-001` stays Open; split `SHOWCLOCK-AUTO-FAILOVER-001` Deferred | Automatic takeover/fence guarantees removed; operator must confirm old physical output stopped before explicit Re-arm; real zero-overlap proof required |
| Corrupt project: preserve/reject and manually restore backup | 10/26 | `MIGRATION-COMPATIBILITY-001` stays Open; split `MIGRATION-AUTO-FALLBACK-001` Deferred | Automatic repair/restore/older-generation fallback; manual native/clean-machine recovery and supported migration remain required |
| **FX Display 60fps REQUIRED:** H264 1080p, 3 layers × 3 ISF stages, actual Display 2560×1600 | 10/25–26 | `VENUE-SOAK-001` stays Open; `COV-PERFORMANCE-001` retains release-blocking proof | ADR-01 fixed native/target-soak gates plus integrated one-hour venue soak; no resolution/stage/threshold reduction or known-limitation waiver |
| Zero spending; free SignPath Foundation first, otherwise unsigned with explicit SmartScreen note | 10/26 | `DIST-SIGNING-001` reopens Open while free approval is pending; `DIST-UPDATER-001` reopens Open | If unavailable by freeze, record `DIST-SIGNING-001` **Deferred**, owner/date/reason/removed Authenticode claim and count/mirror update. Reopen for new candidate/version when free signing is possible. Updater crypto is mandatory regardless |
| No paid hardware/certificates/competitor licenses or substitute spending; comparisons only within permitted free editions | 10/26 | `COMPARE-PINNED-001`, own-product `ASIO-LATENCY-001` stay Open; split `COMPARE-PAID-001` Deferred; Q2 `DEC-COMPARE-001` policy Accepted | Paid/unpermitted trials, including paid-only TD trials, make no parity/latency-superiority claim. Free version/license pins/results remain pending. Split `COMPARE-SYNAPSE-PERF-001` Deferred preserves GAP-01 §9's already cancelled Synapse performance comparison; no restart |
| No RDM devices, scope/analyzer or network switch; no purchases; direct-connected Art-Net accepted | 10/26 | `DMX-ARTNET-001` stays Open; split `DMX-RDM-PHYSICAL-001`, `DMX-WAVEFORM-001`, `DMX-SWITCH-TOPOLOGY-001` Deferred | Physical RDM/TOD, Break 176µs / MAB 16µs waveform timing, switch topology (DMX/two-PC). Reopen only with actual equipment and captures; simulation never closes physical claims |
| Reopen Windows distribution, replacing old Windows-local exemption | 10/26 | All six `DIST-*` original rows Open: platform/package, signing, notices, clean-machine, updater, publication | NSIS/legal/source/clean/update/publication remain required; only documented unsigned Authenticode and MSI/portable/platform exclusions narrow this set |

Candidate manifest fixes source commit/tag, EXE/bridge/runtime/resources/configuration,
profile, signing state, NSIS, updater signature and hashes. Later evidence-only
commits name that build-source tag and those same bytes separately. Every changed
input/byte, rebuild or re-sign creates a distinct retained unpublished candidate
and reruns affected acceptance; it does not overwrite an attempt or inherit its
hash-bound proof. Final-version assignment is not release approval. Published
versions/tags/bytes remain immutable; later public artifacts advance version.
Normal-profile NDI remains subject to legal/native/physical gates if offered.
Unsigned release notes must identify Authenticode absence, possible SmartScreen
warnings/execution policy restrictions, official download/checksum verification
and retained updater signature verification, without promising execution.

| Schedule (JST) | Required result — all pending, not measured by this task |
| --- | --- |
| S0, 10/26 | Receive same-parent source/EXE/hash evidence; fix measured spans/residual, no assumed speedup |
| S1, 10/26–27 | Shared context/typed queue/budget; preserve 8-slot selection/replay/full-history transfer; focused proof |
| S2, 10/28–29 | GPU ISF/composition, one completed CPU snapshot/LastValid, direct present; native A/B and counters |
| S3, decision by 10/30, finish 10/31 | Bounded async only if S2 needs it; preserve latency/Stop/budget; independent measurement |
| 10/31–11/1 | Stable review, native performance/fault/Stop/target-soak gates; signing disposition; freeze final-version exact candidate by **11/1** |
| 11/2–11/8 | Physical/native sessions on identical frozen bytes/profile/signing state; no concurrent GPU measurement interference |
| 11/9–11/11 | Same candidate integrated verification, evidence/Q5 review and publication preparation; changed candidate requires affected reacceptance |
| **11/12** | Fixed completion date; publish only if all required gates pass. Remaining blocker means no completion claim |

Planning inputs: [GAP-01 §1/5/9](../target/qa/release-1.2.0-plan-20261025/GAP-01.md),
[HW-AUTOMATION-01 §0/3/6/10](../target/qa/release-1.2.0-plan-20261025/HW-AUTOMATION-01.md),
[ADR-01 §1/12](../target/qa/steroids-60fps-design-20261025/ADR-01.md),
[ASIO release boundary](ASIO_INPUT_ACCEPTANCE.md#release-boundary).
Owner decisions above supersede those inputs' proposed broader scope and 11/2 freeze;
the saved planning files are not modified or treated as acceptance evidence.

Checkpoint verification is docs/ledger only: four existing Node checker/self-test
gates and owned-file `git diff --check`; no build, app, browser, GPU or physical run.
All four Node gates passed with exit 0 on the final ledger; owned diff checks
passed with exit 0. Independent read-only review found two contract wording gaps
(Q5 source/evidence commits and DJ minimum); both were fixed and re-reviewed with
no remaining findings. Q0 registry and all Q4 evidence entries are unchanged.
Counts change **58 → 68**, **Open 23 → 29**, **Deferred 8 → 12**, Complete **27 → 27**:
six distribution rows reopen; ten excluded compound subclaims gain stable Deferred
IDs. Sections **6: 37 → 38, 7: 6 → 6, 8: 9 → 17, 9: 6 → 7**.
Q1 remains 32, Q2 decisions **16 → 17**, Q3 risks 14 and Q4 evidence 199 unchanged;
no proof is promoted. Checker/self-test counts and display denominator follow these
exact changes; the frozen-distribution negative fixture now attempts to freeze a
reopened row without changing its Open marker and still must fail closed. All
19 completion negative cases and 46 Q1–Q4 negative cases remain enforced.
Compiler-warning baseline/current/delta: unmeasured (no compiler run).

Next action belongs to implementation/acceptance owners: synchronize product UI,
installer/release gates/runbooks and claims to this scope, prepare the frozen
candidate and produce the required evidence. This document changes no runtime,
SDK approval, hardware state or publication.
