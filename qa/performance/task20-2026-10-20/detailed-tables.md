# Task 20 measurement tables

`—` means unavailable or not applicable; it is not zero. These tables are generated from case evidence.

## Measurement boundaries

Delivered FPS uses per-output delivered-counter deltas divided by monotonic elapsed time. Monotonic timestamps are recorded before the asynchronous health query; no per-query completion timestamp or completion-latency bound is captured. Endpoint query-latency differences can therefore bias counter-delta FPS. It counts output submissions; a 30 FPS source does not provide 60 unique source images. Cadence percentiles are the last bounded health window. Its sampleCount is already the interval count (up to 599), so nothing is subtracted. Prepare percentiles are a separate duration window. Native work is an approximation from differences of rounded lifetime average × lifetime frame count; it includes preparation/present submission and possible vsync wait, and excludes pacing. Decode wall time is synchronous batch demand time; background lookahead is excluded. Session resets count in-place demux rewinds/decoder flushes. Session opens and working-set evictions are separate counters; reset counts alone do not establish cache churn.

Native validation requires 600 frames, state `passed`, and `frame_budget_pass=true`. Native validation average/max and deadline-miss fraction describe that 600-frame or still-sampling native window, separately from the approximate observation mean derived from lifetime counters. An empty validation window has unavailable statistics. Cadence and native budget are reported independently. A low-output-rate sampling case can still be a cadence miss. Envelope pass requires complete measurement and every expected output passing cadence and native Display validation. A verified output miss remains a miss when the only gaps are an explicitly declared missing PresentMon capture or insufficient samples and/or unstarted native lifetime-work counters during verified pending warmup; its measurement validity stays partial and it can never pass the envelope. PresentMon complete statistics require at least 10 exact-process rows. A zero-row trace has unobserved PresentMon identity; no PresentMon FPS or percentiles are established. A missing trace remains PresentMon not_established with unobserved identity and unavailable row counts and timings; an absent-file partial verdict requires both missing-file and insufficient-sample markers and cannot establish an envelope pass. Pending-warmup full native work remains unavailable, not zero; output cadence, preparation and decode evidence retain their own boundaries.

## Declared policy

```json
{
  "declaredBeforeBenchmark": true,
  "targetOutputFps": 60,
  "cadenceMinimumTargetRatio": 0.99,
  "missedFractionMaximum": 0.01,
  "cadenceP99MaximumTargetIntervals": 2,
  "nativeBudgetRequired": true,
  "logicalProcessors": 32,
  "warmupSeconds": 10,
  "nominalObservationSeconds": 20,
  "interpretation": "Short steady-state 60 fps screening. Output submission/send cadence, not unique source images or physical scanout. Native 600-frame work-time validation is separate. No success claim for unmeasured/unsupported cases.",
  "nativeBudget": {
    "frames": 600,
    "averageMaxUs": 16666,
    "singleWorkMaxUs": 33333,
    "deadlineMissFractionMax": 0.05
  }
}
```

## Delivery and verdicts — one row per expected output

| Phase | Case | Output | Validity | Cadence | Native | Envelope | FPS / target | Missed | Cadence p50 / p95 / p99 ms | Intervals |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| before | h264-1080p30-1l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.03 / 60.00 | 0.00 | 16.65 / 18.09 / 19.13 | 599 |
| after | h264-1080p30-1l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.91 / 60.00 | 1.00 | 16.67 / 17.66 / 18.53 | 599 |
| before | h264-2160p30-1l-display-0fx-normal | display:1 | complete | pass | fail | miss | 59.73 / 60.00 | 5.00 | 17.29 / 20.78 / 22.13 | 599 |
| after | h264-2160p30-1l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.86 / 60.00 | 3.00 | 16.78 / 20.30 / 23.17 | 599 |
| after | hap-1080p30-1l-display-0fx-normal | display:1 | complete | pass | fail | miss | 59.91 / 60.00 | 2.00 | 16.70 / 18.87 / 22.54 | 599 |
| after | dxv-1080p30-1l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.01 / 60.00 | 0.00 | 16.61 / 18.86 / 20.38 | 599 |
| before | h264-1080p30-3l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.90 / 60.00 | 2.00 | 16.56 / 19.52 / 20.57 | 599 |
| after | h264-1080p30-3l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.94 / 60.00 | 1.00 | 16.64 / 19.97 / 22.62 | 599 |
| after | h264-2160p30-3l-display-0fx-normal | display:1 | complete | fail | fail | miss | 58.71 / 60.00 | 26.00 | 20.30 / 32.99 / 38.72 | 599 |
| after | hap-1080p30-3l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.00 / 60.00 | 0.00 | 16.67 / 17.57 / 18.31 | 599 |
| after | dxv-1080p30-3l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.00 / 60.00 | 0.00 | 16.63 / 21.00 / 23.28 | 599 |
| before | h264-1080p30-6l-display-0fx-normal | display:1 | complete | fail | fail | miss | 31.73 / 60.00 | 561.00 | 17.70 / 120.85 / 326.68 | 599 |
| after | h264-1080p30-6l-display-0fx-normal | display:1 | complete | fail | fail | miss | 31.27 / 60.00 | 561.00 | 17.04 / 108.89 / 418.25 | 599 |
| after | h264-2160p30-6l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.83 / 60.00 | 1236.00 | 234.19 / 5645.13 / 5645.13 | 19 |
| after | hap-1080p30-6l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.00 / 60.00 | 0.00 | 16.61 / 17.93 / 18.58 | 599 |
| after | dxv-1080p30-6l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.95 / 60.00 | 1.00 | 16.60 / 26.32 / 27.27 | 599 |
| before | h264-1080p30-8l-display-0fx-normal | display:1 | complete | fail | not_established | miss | 12.85 / 60.00 | 912.00 | 19.23 / 259.32 / 1600.30 | 406 |
| after | h264-1080p30-8l-display-0fx-normal | display:1 | complete | fail | fail | miss | 27.93 / 60.00 | 624.00 | 17.82 / 144.26 / 503.41 | 599 |
| before | h264-2160p30-8l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.15 / 60.00 | 1197.00 | 3608.01 / 9762.07 / 9762.07 | 6 |
| after | h264-2160p30-8l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.62 / 60.00 | 901.00 | 217.79 / 4546.85 / 7102.07 | 31 |
| after | hap-1080p30-8l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.94 / 60.00 | 0.00 | 16.67 / 18.01 / 18.56 | 599 |
| after | dxv-1080p30-8l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.98 / 60.00 | 1.00 | 16.73 / 29.37 / 30.83 | 599 |
| before | h264-1080p30-12l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.31 / 60.00 | 1350.00 | 3688.15 / 5182.88 / 5182.88 | 9 |
| after | h264-1080p30-12l-display-0fx-normal | display:1 | complete | fail | not_established | miss | 2.76 / 60.00 | 1140.00 | 59.15 / 2141.61 / 2993.13 | 118 |
| after | h264-2160p30-12l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.15 / 60.00 | 1120.00 | 5006.32 / 8363.05 / 8363.05 | 5 |
| after | hap-1080p30-12l-display-0fx-normal | display:1 | complete | pass | pass | pass | 59.92 / 60.00 | 1.00 | 16.64 / 18.38 / 19.19 | 599 |
| after | dxv-1080p30-12l-display-0fx-normal | display:1 | complete | fail | fail | miss | 57.60 / 60.00 | 47.00 | 18.36 / 35.44 / 39.61 | 599 |
| before | h264-1080p30-16l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.15 / 60.00 | 1008.00 | 4864.57 / 7408.54 / 7408.54 | 5 |
| after | h264-1080p30-16l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.36 / 60.00 | 967.00 | 2147.30 / 4284.69 / 4284.69 | 12 |
| before | h264-2160p30-16l-display-0fx-normal | display | workload_failed | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-2160p30-16l-display-0fx-normal | display:1 | partial | fail | not_established | miss | 0.20 / 60.00 | 1347.00 | 3662.67 / 9723.84 / 9723.84 | 6 |
| after | hap-1080p30-16l-display-0fx-normal | display:1 | complete | pass | pass | pass | 60.02 / 60.00 | 0.00 | 16.58 / 19.05 / 21.21 | 599 |
| after | dxv-1080p30-16l-display-0fx-normal | display:1 | complete | fail | fail | miss | 46.34 / 60.00 | 271.00 | 24.99 / 40.52 / 55.01 | 599 |
| after | h264-1080p30-1l-display4k-0fx-normal | display:1 | complete | fail | fail | miss | 43.94 / 60.00 | 312.00 | 24.26 / 28.32 / 42.15 | 599 |
| after | h264-1080p30-1l-spout-0fx-normal | display:1 | complete | pass | pass | pass | 59.99 / 60.00 | 0.00 | 16.64 / 17.69 / 18.20 | 599 |
| after | h264-1080p30-1l-spout-0fx-normal | spout:2 | complete | pass | not_applicable | pass | 59.94 / 60.00 | 0.00 | 16.67 / 17.77 / 18.11 | 599 |
| after | h264-1080p30-1l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-1l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| before | h264-1080p30-3l-display4k-0fx-normal | display:1 | complete | fail | not_established | miss | 14.33 / 60.00 | 893.00 | 64.48 / 121.89 / 474.26 | 454 |
| after | h264-1080p30-3l-display4k-0fx-normal | display:1 | complete | fail | fail | miss | 23.64 / 60.00 | 701.00 | 29.87 / 93.21 / 374.04 | 599 |
| before | h264-1080p30-3l-spout-0fx-normal | display:1 | complete | pass | fail | miss | 59.90 / 60.00 | 2.00 | 16.68 / 20.72 / 22.03 | 599 |
| before | h264-1080p30-3l-spout-0fx-normal | spout:2 | complete | pass | not_applicable | miss | 59.95 / 60.00 | 1.00 | 16.53 / 19.83 / 21.87 | 599 |
| after | h264-1080p30-3l-spout-0fx-normal | display:1 | complete | fail | pass | miss | 57.83 / 60.00 | 42.00 | 16.51 / 19.31 / 21.43 | 599 |
| after | h264-1080p30-3l-spout-0fx-normal | spout:2 | complete | fail | not_applicable | miss | 57.68 / 60.00 | 46.00 | 16.51 / 19.82 / 21.25 | 599 |
| after | h264-1080p30-3l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-3l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-6l-display4k-0fx-normal | display:1 | complete | fail | not_established | miss | 6.34 / 60.00 | 1032.00 | 89.82 / 552.36 / 1140.11 | 207 |
| after | h264-1080p30-6l-spout-0fx-normal | display:1 | complete | fail | not_established | miss | 18.89 / 60.00 | 803.00 | 20.38 / 197.76 / 670.48 | 597 |
| after | h264-1080p30-6l-spout-0fx-normal | spout:2 | complete | fail | not_applicable | miss | 18.63 / 60.00 | 807.00 | 19.31 / 220.59 / 719.74 | 599 |
| after | h264-1080p30-6l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-6l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-8l-display4k-0fx-normal | display:1 | complete | fail | not_established | miss | 3.86 / 60.00 | 1110.00 | 113.22 / 1193.79 / 2076.00 | 119 |
| after | h264-1080p30-8l-spout-0fx-normal | display:1 | complete | fail | not_established | miss | 4.26 / 60.00 | 1027.00 | 61.87 / 831.13 / 2347.66 | 171 |
| after | h264-1080p30-8l-spout-0fx-normal | spout:2 | complete | fail | not_applicable | miss | 4.11 / 60.00 | 845.00 | 60.91 / 818.28 / 2302.05 | 164 |
| after | h264-1080p30-8l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-8l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-12l-display4k-0fx-normal | display:1 | partial | fail | not_established | miss | 0.97 / 60.00 | 1161.00 | 508.04 / 3048.60 / 3317.32 | 35 |
| after | h264-1080p30-12l-spout-0fx-normal | display:1 | partial | fail | not_established | miss | 0.31 / 60.00 | 994.00 | 2694.73 / 5001.62 / 5001.62 | 10 |
| after | h264-1080p30-12l-spout-0fx-normal | spout:2 | partial | fail | not_applicable | miss | 0.31 / 60.00 | 994.00 | 2704.09 / 4993.34 / 4993.34 | 11 |
| after | h264-1080p30-12l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-12l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-16l-display4k-0fx-normal | display:1 | partial | fail | not_established | miss | 0.36 / 60.00 | 1153.00 | 1689.54 / 4316.68 / 4316.68 | 15 |
| after | h264-1080p30-16l-spout-0fx-normal | display:1 | partial | fail | not_established | miss | 0.20 / 60.00 | 1154.00 | 4235.12 / 6362.93 / 6362.93 | 7 |
| after | h264-1080p30-16l-spout-0fx-normal | spout:2 | partial | fail | not_applicable | miss | 0.20 / 60.00 | 1158.00 | 3418.31 / 7738.22 / 7738.22 | 8 |
| after | h264-1080p30-16l-ndi-0fx-normal | display | unsupported | not_established | not_established | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-16l-ndi-0fx-normal | ndi | unsupported | not_established | not_applicable | not_established | — / — | — | — / — / — | — |
| after | h264-1080p30-1l-display-1fx-normal | display:1 | complete | fail | fail | miss | 50.93 / 60.00 | 176.00 | 19.20 / 22.73 / 25.66 | 599 |
| before | h264-1080p30-1l-display-3fx-normal | display:1 | complete | fail | not_established | miss | 17.21 / 60.00 | 829.00 | 58.02 / 71.30 / 108.84 | 538 |
| after | h264-1080p30-1l-display-3fx-normal | display:1 | complete | fail | not_established | miss | 20.68 / 60.00 | 761.00 | 46.88 / 58.05 / 61.28 | 599 |
| after | h264-1080p30-3l-display-1fx-normal | display:1 | complete | fail | not_established | miss | 11.03 / 60.00 | 956.00 | 67.63 / 258.28 / 436.04 | 354 |
| before | h264-1080p30-3l-display-3fx-normal | display:1 | complete | fail | not_established | miss | 4.76 / 60.00 | 1072.00 | 180.11 / 347.85 / 849.78 | 150 |
| after | h264-1080p30-3l-display-3fx-normal | display:1 | complete | fail | not_established | miss | 5.28 / 60.00 | 1046.00 | 153.40 / 418.28 / 579.85 | 165 |
| after | h264-1080p30-6l-display-1fx-normal | display:1 | complete | fail | not_established | miss | 4.37 / 60.00 | 1081.00 | 132.67 / 739.13 / 1156.31 | 134 |
| after | h264-1080p30-6l-display-3fx-normal | display:1 | complete | fail | not_established | miss | 2.42 / 60.00 | 1109.00 | 328.85 / 1145.31 / 1777.88 | 67 |
| after | h264-1080p30-8l-display-1fx-normal | display:1 | complete | fail | not_established | miss | 2.63 / 60.00 | 1112.00 | 199.47 / 1920.64 / 2370.44 | 75 |
| before | h264-1080p30-8l-display-3fx-normal | display:1 | partial | fail | not_established | miss | 0.57 / 60.00 | 1211.00 | 659.44 / 3937.40 / 4109.23 | 23 |
| after | h264-1080p30-8l-display-3fx-normal | display:1 | partial | fail | not_established | miss | 1.18 / 60.00 | 1041.00 | 495.00 / 2150.42 / 2498.20 | 35 |
| after | h264-1080p30-12l-display-1fx-normal | display:1 | partial | fail | not_established | miss | 0.72 / 60.00 | 1029.00 | 944.42 / 3436.11 / 3583.75 | 20 |
| after | h264-1080p30-12l-display-3fx-normal | display:1 | partial | fail | not_established | miss | 0.56 / 60.00 | 1207.00 | 1754.26 / 4726.14 / 4726.14 | 15 |
| after | h264-1080p30-16l-display-1fx-normal | display:1 | partial | fail | not_established | miss | 0.31 / 60.00 | 1000.00 | 2252.13 / 4675.04 / 4675.04 | 10 |
| after | h264-1080p30-16l-display-3fx-normal | display:1 | partial | fail | not_established | miss | 0.36 / 60.00 | 1157.00 | 2746.37 / 6815.32 / 6815.32 | 10 |
| after | h264-1080p30-3l-display-0fx-add | display:1 | complete | pass | pass | pass | 60.00 / 60.00 | 0.00 | 16.61 / 19.08 / 19.87 | 599 |
| after | h264-1080p30-3l-display-0fx-multiply | display:1 | complete | pass | pass | pass | 60.02 / 60.00 | 0.00 | 16.64 / 19.17 / 20.53 | 599 |
| after | h264-1080p30-3l-display-0fx-screen | display:1 | complete | pass | pass | pass | 60.02 / 60.00 | 0.00 | 16.71 / 19.01 / 19.54 | 599 |
| after | h264-1080p30-3l-display-0fx-overlay | display:1 | complete | pass | pass | pass | 59.98 / 60.00 | 0.00 | 16.64 / 19.27 / 19.93 | 599 |
| after | h264-1080p30-3l-display-0fx-subtract | display:1 | complete | pass | pass | pass | 59.98 / 60.00 | 1.00 | 16.73 / 19.16 / 19.61 | 599 |
| after | h264-1080p30-3l-display-0fx-difference | display:1 | complete | pass | pass | pass | 59.96 / 60.00 | 0.00 | 16.65 / 19.25 / 19.90 | 599 |
| after | h264-1080p30-3l-display-0fx-lighten | display:1 | complete | pass | pass | pass | 60.00 / 60.00 | 0.00 | 16.64 / 19.22 / 19.60 | 599 |
| after | h264-1080p30-3l-display-0fx-darken | display:1 | complete | pass | pass | pass | 60.01 / 60.00 | 0.00 | 16.72 / 19.11 / 19.92 | 599 |
| after | h264-1080p30-3l-display-0fx-softlight | display:1 | complete | pass | pass | pass | 59.97 / 60.00 | 0.00 | 16.66 / 19.11 / 19.56 | 599 |
| after | h264-1080p30-3l-display-0fx-normal-cpu | display:1 | complete | fail | not_established | miss | 11.00 / 60.00 | 957.00 | 73.48 / 245.58 / 383.17 | 344 |
| before | h264-1080p30-8l-display-0fx-normal-cpu | display:1 | partial | fail | not_established | miss | 1.80 / 60.00 | 1192.00 | 299.81 / 2728.29 / 3440.17 | 52 |
| after | h264-1080p30-8l-display-0fx-normal-cpu | display:1 | complete | fail | not_established | miss | 2.49 / 60.00 | 1140.00 | 214.45 / 1886.10 / 2434.82 | 70 |
| after | h264-1080p30-16l-display-0fx-normal-cpu | display:1 | partial | fail | not_established | miss | 0.55 / 60.00 | 1194.00 | 1247.99 / 4185.22 / 4185.22 | 16 |
| after | h264-1080p30-3l-display-groups | display:1 | complete | fail | fail | miss | 57.51 / 60.00 | 48.00 | 16.67 / 19.07 / 20.35 | 599 |
| before | h264-1080p30-8l-display-groups | display:1 | complete | fail | not_established | miss | 18.70 / 60.00 | 814.00 | 23.21 / 328.82 / 989.24 | 455 |
| after | h264-1080p30-8l-display-groups | display:1 | complete | fail | not_established | miss | 19.95 / 60.00 | 721.00 | 20.44 / 170.91 / 892.82 | 568 |
| after | h264-1080p30-16l-display-groups | display:1 | partial | fail | not_established | miss | 0.41 / 60.00 | 1054.00 | 1958.53 / 4418.00 / 4418.00 | 12 |

## Prepare, native work and synchronous decode

| Phase | Case | Output | Prepare p50 / p95 / p99 ms | Prepare n | Native state / n | Native validation avg / max ms | Native validation miss fraction | Observed native work ms ≈ | Decode wall ms / calls | Opens / evictions / resets |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| before | h264-1080p30-1l-display-0fx-normal | display | 3.46 / 4.45 / 5.32 | 600 | passed / 600 | 5.93 / 11.81 | 0.0000 | 5.86 | 189.89 / 1164.00 | 0.00 / 0.00 / 1.00 |
| after | h264-1080p30-1l-display-0fx-normal | display | 3.25 / 3.74 / 4.03 | 600 | passed / 600 | 5.38 / 7.82 | 0.0000 | 5.36 | 190.02 / 1160.00 | 0.00 / 0.00 / 1.00 |
| before | h264-2160p30-1l-display-0fx-normal | display | 4.79 / 9.86 / 14.54 | 600 | failed / 600 | 8.28 / 37.42 | 0.0133 | 8.26 | 290.34 / 1171.00 | 0.00 / 0.00 / 2.00 |
| after | h264-2160p30-1l-display-0fx-normal | display | 3.60 / 8.20 / 10.84 | 600 | passed / 600 | 6.20 / 15.70 | 0.0000 | 6.26 | 225.73 / 1162.00 | 0.00 / 0.00 / 1.00 |
| after | hap-1080p30-1l-display-0fx-normal | display | 9.00 / 12.02 / 13.19 | 600 | failed / 600 | 12.29 / 36.71 | 0.0633 | 11.72 | 376.56 / 1169.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-1l-display-0fx-normal | display | 3.87 / 5.09 / 5.75 | 600 | passed / 600 | 6.95 / 13.13 | 0.0000 | 7.27 | 193.43 / 1180.00 | 0.00 / 0.00 / 2.00 |
| before | h264-1080p30-3l-display-0fx-normal | display | 0.35 / 1.83 / 4.09 | 600 | passed / 600 | 3.32 / 31.89 | 0.0017 | 3.50 | 326.25 / 1180.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-3l-display-0fx-normal | display | 0.35 / 2.95 / 9.01 | 600 | passed / 600 | 3.97 / 22.46 | 0.0033 | 3.90 | 304.93 / 1188.00 | 0.00 / 0.00 / 6.00 |
| after | h264-2160p30-3l-display-0fx-normal | display | 4.78 / 23.60 / 31.73 | 600 | failed / 600 | 9.64 / 38.93 | 0.2200 | 9.14 | 342.20 / 1136.00 | 0.00 / 0.00 / 3.00 |
| after | hap-1080p30-3l-display-0fx-normal | display | 0.77 / 1.07 / 1.19 | 600 | passed / 600 | 2.25 / 4.51 | 0.0000 | 2.31 | 757.11 / 1174.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-3l-display-0fx-normal | display | 0.34 / 0.47 / 1.80 | 600 | passed / 600 | 3.98 / 14.59 | 0.0000 | 4.34 | 288.07 / 1191.00 | 0.00 / 0.00 / 6.00 |
| before | h264-1080p30-6l-display-0fx-normal | display | 2.08 / 113.20 / 272.01 | 600 | failed / 600 | 23.32 / 873.84 | 0.1767 | 23.61 | 6448.75 / 627.00 | 0.00 / 0.00 / 12.00 |
| after | h264-1080p30-6l-display-0fx-normal | display | 0.79 / 100.75 / 373.12 | 600 | failed / 600 | 22.18 / 701.10 | 0.1500 | 22.40 | 4018.30 / 607.00 | 0.00 / 0.00 / 12.00 |
| after | h264-2160p30-6l-display-0fx-normal | display | 230.86 / 5623.65 / 5623.65 | 19 | pending / 0 | — / — | — | — | 20371.88 / 16.00 | 0.00 / 0.00 / 42.00 |
| after | hap-1080p30-6l-display-0fx-normal | display | 1.27 / 1.69 / 1.95 | 600 | passed / 600 | 2.90 / 5.43 | 0.0000 | 2.92 | 1278.06 / 1182.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-6l-display-0fx-normal | display | 0.43 / 0.62 / 1.73 | 600 | passed / 600 | 6.39 / 13.25 | 0.0000 | 6.34 | 371.68 / 1180.00 | 0.00 / 0.00 / 6.00 |
| before | h264-1080p30-8l-display-0fx-normal | display | 4.17 / 244.87 / 1590.55 | 406 | sampling / 348 | 61.80 / 2958.05 | 0.2471 | 72.06 | 10628.84 / 248.00 | 0.00 / 0.00 / 48.00 |
| after | h264-1080p30-8l-display-0fx-normal | display | 2.96 / 113.21 / 439.24 | 600 | failed / 600 | 27.93 / 892.26 | 0.1800 | 28.22 | 3794.51 / 542.00 | 0.00 / 0.00 / 16.00 |
| before | h264-2160p30-8l-display-0fx-normal | display | 3541.30 / 9755.26 / 9755.26 | 6 | pending / 0 | — / — | — | — | 19828.00 / 3.00 | 0.00 / 0.00 / 24.00 |
| after | h264-2160p30-8l-display-0fx-normal | display | 209.68 / 4505.29 / 7094.85 | 31 | pending / 0 | — / — | — | — | 14701.93 / 12.00 | 0.00 / 0.00 / 16.00 |
| after | hap-1080p30-8l-display-0fx-normal | display | 1.57 / 2.02 / 2.33 | 600 | passed / 600 | 3.25 / 8.00 | 0.0000 | 3.27 | 1568.71 / 1174.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-8l-display-0fx-normal | display | 0.51 / 2.25 / 7.00 | 600 | passed / 600 | 8.26 / 22.31 | 0.0183 | 8.22 | 462.93 / 1184.00 | 0.00 / 0.00 / 8.00 |
| before | h264-1080p30-12l-display-0fx-normal | display | 3670.73 / 5105.74 / 5105.74 | 9 | pending / 0 | — / — | — | — | 22352.41 / 6.00 | 72.00 / 72.00 / 0.00 |
| after | h264-1080p30-12l-display-0fx-normal | display | 50.44 / 2132.62 / 2946.97 | 118 | sampling / 57 | 363.96 / 3596.61 | 0.9649 | 368.32 | 16162.89 / 54.00 | 0.00 / 0.00 / 60.00 |
| after | h264-2160p30-12l-display-0fx-normal | display | 4926.61 / 8315.61 / 8315.61 | 5 | pending / 0 | — / — | — | — | 18466.77 / 3.00 | 0.00 / 0.00 / 36.00 |
| after | hap-1080p30-12l-display-0fx-normal | display | 2.22 / 2.78 / 3.17 | 600 | passed / 600 | 4.19 / 7.98 | 0.0000 | 4.28 | 2261.17 / 1190.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-12l-display-0fx-normal | display | 0.64 / 12.09 / 24.86 | 600 | failed / 600 | 12.86 / 50.72 | 0.5117 | 12.84 | 542.32 / 1090.00 | 0.00 / 0.00 / 12.00 |
| before | h264-1080p30-16l-display-0fx-normal | display | 4796.12 / 7347.75 / 7347.75 | 5 | pending / 0 | — / — | — | — | 16702.66 / 3.00 | 48.00 / 48.00 / 0.00 |
| after | h264-1080p30-16l-display-0fx-normal | display | 2130.71 / 4188.36 / 4188.36 | 12 | pending / 0 | — / — | — | — | 15872.10 / 7.00 | 0.00 / 0.00 / 112.00 |
| before | h264-2160p30-16l-display-0fx-normal | display | — / — / — | — | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-2160p30-16l-display-0fx-normal | display | 3629.75 / 9713.34 / 9713.34 | 6 | pending / 0 | — / — | — | — | 21981.40 / 4.00 | 0.00 / 0.00 / 64.00 |
| after | hap-1080p30-16l-display-0fx-normal | display | 2.88 / 3.66 / 4.30 | 600 | passed / 600 | 5.09 / 10.98 | 0.0000 | 5.14 | 2861.77 / 1152.00 | 0.00 / 0.00 / 0.00 |
| after | dxv-1080p30-16l-display-0fx-normal | display | 0.73 / 2.26 / 32.58 | 600 | failed / 600 | 17.49 / 75.20 | 0.6233 | 18.15 | 956.77 / 913.00 | 0.00 / 0.00 / 32.00 |
| after | h264-1080p30-1l-display4k-0fx-normal | display | 20.29 / 23.95 / 37.95 | 600 | failed / 600 | 22.79 / 44.08 | 0.8117 | 22.74 | 527.99 / 854.00 | 0.00 / 0.00 / 1.00 |
| after | h264-1080p30-1l-spout-0fx-normal | display | 3.29 / 3.79 / 4.22 | 600 | passed / 600 | 5.51 / 9.20 | 0.0000 | 5.55 | 200.33 / 1171.00 | 0.00 / 0.00 / 1.00 |
| after | h264-1080p30-1l-spout-0fx-normal | spout | 3.42 / 4.03 / 4.30 | 600 | — / — | — / — | — | — | — / — | — / — / — |
| before | h264-1080p30-3l-display4k-0fx-normal | display | 54.42 / 110.92 / 467.75 | 454 | sampling / 354 | 71.81 / 713.08 | 1.0000 | 69.78 | 9514.28 / 280.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-3l-display4k-0fx-normal | display | 21.02 / 78.92 / 342.56 | 600 | failed / 600 | 41.61 / 518.48 | 0.8033 | 42.00 | 3960.14 / 457.00 | 0.00 / 0.00 / 3.00 |
| before | h264-1080p30-3l-spout-0fx-normal | display | 0.49 / 5.39 / 8.18 | 600 | failed / 600 | 5.48 / 41.76 | 0.0033 | 5.26 | 426.97 / 1195.00 | 0.00 / 0.00 / 6.00 |
| before | h264-1080p30-3l-spout-0fx-normal | spout | 8.39 / 11.39 / 14.65 | 600 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-3l-spout-0fx-normal | display | 0.37 / 1.32 / 2.72 | 600 | passed / 600 | 3.35 / 30.07 | 0.0033 | 4.30 | 731.16 / 1136.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-3l-spout-0fx-normal | spout | 6.98 / 9.24 / 10.96 | 600 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-6l-display4k-0fx-normal | display | 67.70 / 533.36 / 1053.24 | 207 | sampling / 123 | 154.96 / 839.95 | 1.0000 | 154.96 | 12563.32 / 124.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-6l-spout-0fx-normal | display | 10.65 / 190.85 / 663.12 | 597 | sampling / 534 | 42.73 / 1695.79 | 0.4007 | 49.14 | 10049.43 / 370.00 | 0.00 / 0.00 / 18.00 |
| after | h264-1080p30-6l-spout-0fx-normal | spout | 16.75 / 210.04 / 611.00 | 600 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-8l-display4k-0fx-normal | display | 84.98 / 1165.55 / 2058.20 | 119 | sampling / 51 | 286.63 / 2376.14 | 1.0000 | 286.63 | 15145.72 / 76.00 | 0.00 / 0.00 / 24.00 |
| after | h264-1080p30-8l-spout-0fx-normal | display | 52.49 / 822.95 / 2342.50 | 171 | sampling / 110 | 188.77 / 3037.46 | 0.9000 | 222.64 | 14598.58 / 83.00 | 0.00 / 0.00 / 32.00 |
| after | h264-1080p30-8l-spout-0fx-normal | spout | 58.44 / 863.92 / 2290.21 | 165 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-12l-display4k-0fx-normal | display | 426.23 / 2924.33 / 3156.69 | 35 | pending / 0 | — / — | — | — | 17954.94 / 19.00 | 0.00 / 0.00 / 96.00 |
| after | h264-1080p30-12l-spout-0fx-normal | display | 2679.01 / 4948.45 / 4948.45 | 10 | pending / 0 | — / — | — | — | 16333.54 / 6.00 | 0.00 / 0.00 / 72.00 |
| after | h264-1080p30-12l-spout-0fx-normal | spout | 2150.08 / 4958.24 / 4958.24 | 12 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-16l-display4k-0fx-normal | display | 1628.17 / 4197.93 / 4197.93 | 15 | pending / 0 | — / — | — | — | 18325.22 / 7.00 | 0.00 / 0.00 / 112.00 |
| after | h264-1080p30-16l-spout-0fx-normal | display | 4226.03 / 6351.64 / 6351.64 | 7 | pending / 0 | — / — | — | — | 19042.84 / 4.00 | 0.00 / 0.00 / 64.00 |
| after | h264-1080p30-16l-spout-0fx-normal | spout | 3413.14 / 7733.51 / 7733.51 | 9 | — / — | — / — | — | — | — / — | — / — / — |
| after | h264-1080p30-1l-display-1fx-normal | display | 17.36 / 20.67 / 23.46 | 600 | failed / 600 | 19.60 / 35.43 | 0.9967 | 19.64 | 386.39 / 984.00 | 0.00 / 0.00 / 1.00 |
| before | h264-1080p30-1l-display-3fx-normal | display | 56.00 / 68.13 / 106.34 | 538 | sampling / 471 | 57.29 / 106.81 | 1.0000 | 58.17 | 2664.37 / 333.00 | 0.00 / 0.00 / 2.00 |
| after | h264-1080p30-1l-display-3fx-normal | display | 45.00 / 56.02 / 58.45 | 600 | sampling / 579 | 47.67 / 99.36 | 1.0000 | 48.35 | 1883.32 / 400.00 | 0.00 / 0.00 / 1.00 |
| after | h264-1080p30-3l-display-1fx-normal | display | 65.58 / 255.97 / 433.87 | 354 | sampling / 288 | 87.07 / 502.56 | 1.0000 | 91.10 | 6841.08 / 214.00 | 0.00 / 0.00 / 3.00 |
| before | h264-1080p30-3l-display-3fx-normal | display | 177.20 / 345.73 / 829.58 | 150 | sampling / 88 | 209.71 / 911.41 | 1.0000 | 209.71 | 4714.75 / 93.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-3l-display-3fx-normal | display | 151.39 / 416.20 / 577.97 | 165 | sampling / 102 | 187.65 / 579.69 | 1.0000 | 187.65 | 5846.79 / 102.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-6l-display-1fx-normal | display | 130.55 / 724.29 / 1154.55 | 134 | sampling / 73 | 225.91 / 1156.35 | 1.0000 | 225.91 | 10300.58 / 85.00 | 0.00 / 0.00 / 12.00 |
| after | h264-1080p30-6l-display-3fx-normal | display | 327.32 / 1143.52 / 1775.94 | 67 | sampling / 6 | 401.24 / 691.06 | 1.0000 | 401.24 | 7768.74 / 47.00 | 0.00 / 0.00 / 6.00 |
| after | h264-1080p30-8l-display-1fx-normal | display | 197.54 / 1916.84 / 2368.43 | 75 | sampling / 14 | 596.00 / 2370.33 | 1.0000 | 596.00 | 12407.88 / 51.00 | 0.00 / 0.00 / 24.00 |
| before | h264-1080p30-8l-display-3fx-normal | display | 655.25 / 3933.38 / 4080.13 | 23 | pending / 0 | — / — | — | — | 14719.85 / 11.00 | 0.00 / 0.00 / 56.00 |
| after | h264-1080p30-8l-display-3fx-normal | display | 492.19 / 2148.32 / 2496.42 | 35 | pending / 0 | — / — | — | — | 10458.36 / 23.00 | 0.00 / 0.00 / 32.00 |
| after | h264-1080p30-12l-display-1fx-normal | display | 942.13 / 3433.98 / 3566.18 | 20 | pending / 0 | — / — | — | — | 14412.00 / 14.00 | 0.00 / 0.00 / 72.00 |
| after | h264-1080p30-12l-display-3fx-normal | display | 1752.35 / 4708.37 / 4708.37 | 15 | pending / 0 | — / — | — | — | 14963.56 / 11.00 | 0.00 / 0.00 / 84.00 |
| after | h264-1080p30-16l-display-1fx-normal | display | 2249.65 / 4673.00 / 4673.00 | 10 | pending / 0 | — / — | — | — | 15150.14 / 6.00 | 0.00 / 0.00 / 96.00 |
| after | h264-1080p30-16l-display-3fx-normal | display | 2737.00 / 6796.97 / 6796.97 | 10 | pending / 0 | — / — | — | — | 15205.53 / 7.00 | 0.00 / 0.00 / 96.00 |
| after | h264-1080p30-3l-display-0fx-add | display | 0.32 / 0.44 / 0.87 | 600 | passed / 600 | 2.93 / 6.28 | 0.0000 | 2.90 | 272.44 / 1168.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-multiply | display | 0.32 / 0.45 / 1.04 | 600 | passed / 600 | 2.81 / 7.91 | 0.0000 | 2.84 | 273.67 / 1172.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-screen | display | 0.33 / 0.49 / 1.72 | 600 | passed / 600 | 2.88 / 8.31 | 0.0000 | 2.91 | 274.15 / 1171.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-overlay | display | 0.33 / 0.45 / 1.60 | 600 | passed / 600 | 2.86 / 7.38 | 0.0000 | 2.87 | 272.99 / 1167.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-subtract | display | 0.33 / 0.46 / 1.22 | 600 | passed / 600 | 2.86 / 5.29 | 0.0000 | 2.86 | 269.53 / 1168.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-difference | display | 0.33 / 0.48 / 1.56 | 600 | passed / 600 | 2.87 / 7.74 | 0.0000 | 2.90 | 269.46 / 1171.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-lighten | display | 0.32 / 0.47 / 1.57 | 600 | passed / 600 | 2.90 / 9.09 | 0.0000 | 2.94 | 270.39 / 1182.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-darken | display | 0.33 / 0.47 / 1.20 | 600 | passed / 600 | 2.81 / 5.81 | 0.0000 | 2.84 | 273.35 / 1170.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-softlight | display | 0.32 / 0.47 / 1.61 | 600 | passed / 600 | 2.91 / 7.07 | 0.0000 | 2.91 | 270.58 / 1170.00 | 0.00 / 0.00 / 3.00 |
| after | h264-1080p30-3l-display-0fx-normal-cpu | display | 71.28 / 243.60 / 381.21 | 344 | sampling / 279 | 88.53 / 437.84 | 1.0000 | 91.29 | 6952.36 / 213.00 | 0.00 / 0.00 / 3.00 |
| before | h264-1080p30-8l-display-0fx-normal-cpu | display | 296.02 / 2725.85 / 3419.93 | 52 | pending / 0 | — / — | — | — | 14050.16 / 36.00 | 0.00 / 0.00 / 48.00 |
| after | h264-1080p30-8l-display-0fx-normal-cpu | display | 212.21 / 1883.93 / 2432.23 | 70 | sampling / 9 | 310.70 / 1215.51 | 1.0000 | 310.70 | 12399.96 / 49.00 | 0.00 / 0.00 / 32.00 |
| after | h264-1080p30-16l-display-0fx-normal-cpu | display | 1245.90 / 4183.02 / 4183.02 | 16 | pending / 0 | — / — | — | — | 16711.01 / 11.00 | 0.00 / 0.00 / 112.00 |
| after | h264-1080p30-3l-display-groups | display | 11.98 / 14.32 / 14.96 | 600 | failed / 600 | 14.54 / 18.46 | 0.0650 | 15.09 | 474.94 / 1127.00 | 0.00 / 0.00 / 3.00 |
| before | h264-1080p30-8l-display-groups | display | 19.97 / 324.85 / 976.70 | 455 | sampling / 394 | 55.79 / 1006.70 | 0.8604 | 53.32 | 5576.38 / 370.00 | 0.00 / 0.00 / 24.00 |
| after | h264-1080p30-8l-display-groups | display | 17.69 / 168.01 / 890.33 | 568 | sampling / 507 | 43.26 / 922.14 | 0.7041 | 47.02 | 4703.71 / 395.00 | 0.00 / 0.00 / 8.00 |
| after | h264-1080p30-16l-display-groups | display | 1956.10 / 4414.88 / 4414.88 | 12 | pending / 0 | — / — | — | — | 17349.83 / 8.00 | 0.00 / 0.00 / 112.00 |

## Process tree, GPU, PresentMon and monitors

CPU is normalized to 32 logical processors; cumulative readings are retained by PID and process start time, including exited processes. Membership changes are a note rather than an invalidation. Memory is the sampled sum over the process tree. GPU percentages sum reported matching engine instances and are not a device-wide utilization limit. The video-codec metric is the sum of instances named exactly `Video Codec Engine`, can exceed 100%, and is kept separate from the decode counter; an unavailable decode counter remains unavailable. An engine category with no matching instances is unobserved, not zero; provider and per-category coverage are recorded separately. PresentMon uses the exact native PID and a single swapchain. Background build overlap is recorded only when in-sample monitoring is declared available; otherwise overlap is unknown. Process/PresentMon/monitor metrics repeat across outputs only in CSV.

| Phase | Case | CPU % | WS mean / peak GiB | Private mean / peak GiB | GPU 3D / video-codec / copy % | GPU provider / video-codec coverage | PM status / rows | PM FPS | PM p50 / p95 / p99 ms | Monitor PGM / PVW FPS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| before | h264-1080p30-1l-display-0fx-normal | 7.62 | 1.12 / 1.13 | 1.45 / 1.54 | 7.71 / 14.57 / 1.00 | 1.00 / 1.00 | measured / 1198 | 60.03 | 16.66 / 17.98 / 19.13 | 22.88 / 22.97 |
| after | h264-1080p30-1l-display-0fx-normal | 6.88 | 1.11 / 1.13 | 1.49 / 1.56 | 8.00 / 15.71 / 1.00 | 1.00 / 1.00 | measured / 1198 | 60.01 | 16.68 / 17.68 / 18.33 | 26.72 / 26.67 |
| before | h264-2160p30-1l-display-0fx-normal | 8.73 | 1.23 / 1.24 | 1.78 / 1.86 | 7.30 / 55.90 / 1.90 | 1.00 / 1.00 | measured / 1196 | 59.86 | 17.20 / 20.80 / 22.37 | 13.22 / 12.83 |
| after | h264-2160p30-1l-display-0fx-normal | 8.10 | 1.24 / 1.27 | 1.85 / 1.98 | 7.64 / 51.64 / 1.00 | 1.00 / 1.00 | measured / 1196 | 59.89 | 16.79 / 19.74 / 22.89 | 23.16 / 23.26 |
| after | hap-1080p30-1l-display-0fx-normal | 9.32 | 0.92 / 0.93 | 1.18 / 1.22 | 8.09 / 0.00 / 1.27 | 1.00 / 1.00 | measured / 1198 | 59.95 | 16.70 / 18.48 / 21.79 | 27.91 / 27.76 |
| after | dxv-1080p30-1l-display-0fx-normal | 9.12 | 1.01 / 1.03 | 1.16 / 1.27 | 7.50 / 0.00 / 1.00 | 1.00 / 1.00 | measured / 1198 | 60.04 | 16.65 / 19.12 / 22.05 | 22.00 / 21.81 |
| before | h264-1080p30-3l-display-0fx-normal | 7.92 | 1.47 / 1.48 | 2.08 / 2.20 | 15.27 / 36.45 / 1.00 | 1.00 / 1.00 | measured / 1198 | 60.05 | 16.55 / 19.54 / 20.85 | 17.77 / 18.01 |
| after | h264-1080p30-3l-display-0fx-normal | 7.95 | 1.47 / 1.49 | 2.12 / 2.21 | 14.60 / 36.60 / 0.50 | 1.00 / 1.00 | measured / 1201 | 60.07 | 16.64 / 19.79 / 21.90 | 19.84 / 20.60 |
| after | h264-2160p30-3l-display-0fx-normal | 5.58 | 1.74 / 1.84 | 2.98 / 3.09 | 12.36 / 153.00 / 1.00 | 1.00 / 1.00 | measured / 1164 | 58.30 | 19.68 / 31.21 / 38.57 | 3.18 / 2.84 |
| after | hap-1080p30-3l-display-0fx-normal | 6.48 | 0.96 / 0.97 | 1.23 / 1.27 | 15.33 / 0.00 / 0.25 | 1.00 / 1.00 | measured / 1199 | 60.06 | 16.67 / 17.56 / 18.12 | 26.04 / 28.46 |
| after | dxv-1080p30-3l-display-0fx-normal | 9.97 | 1.23 / 1.24 | 1.43 / 1.51 | 15.40 / 0.00 / 0.90 | 1.00 / 1.00 | measured / 1198 | 60.04 | 16.64 / 21.14 / 24.08 | 17.20 / 17.91 |
| before | h264-1080p30-6l-display-0fx-normal | 7.40 | 2.12 / 2.20 | 3.21 / 3.33 | 8.60 / 144.30 / 0.50 | 1.00 / 1.00 | measured / 633 | 31.76 | 17.88 / 120.70 / 277.15 | 0.82 / 0.05 |
| after | h264-1080p30-6l-display-0fx-normal | 5.81 | 2.05 / 2.08 | 2.92 / 2.96 | 14.64 / 97.73 / 0.64 | 1.00 / 1.00 | measured / 642 | 32.23 | 16.99 / 86.79 / 351.59 | 0.93 / 0.00 |
| after | h264-2160p30-6l-display-0fx-normal | 2.65 | 3.07 / 3.12 | 4.63 / 4.66 | 0.55 / 186.09 / 0.00 | 1.00 / 1.00 | measured / 15 | 0.80 | 69.75 / 4274.00 / 4774.09 | 0.39 / 0.00 |
| after | hap-1080p30-6l-display-0fx-normal | 5.37 | 1.04 / 1.05 | 1.25 / 1.30 | 20.83 / 0.00 / 0.58 | 1.00 / 1.00 | measured / 1199 | 60.04 | 16.65 / 17.86 / 18.41 | 13.43 / 13.58 |
| after | dxv-1080p30-6l-display-0fx-normal | 11.81 | 1.69 / 1.73 | 1.91 / 1.97 | 20.82 / 0.00 / 0.64 | 1.00 / 1.00 | measured / 1200 | 60.02 | 16.68 / 26.18 / 27.22 | 6.98 / 6.98 |
| before | h264-1080p30-8l-display-0fx-normal | 6.77 | 2.46 / 2.54 | 3.69 / 3.78 | 8.50 / 143.20 / 0.30 | 1.00 / 1.00 | measured / 284 | 14.69 | 18.77 / 231.83 / 1404.11 | 0.59 / 0.05 |
| after | h264-1080p30-8l-display-0fx-normal | 7.50 | 2.41 / 2.48 | 3.59 / 3.67 | 12.55 / 115.00 / 0.27 | 1.00 / 1.00 | measured / 570 | 28.54 | 17.76 / 111.09 / 457.68 | 0.83 / 0.00 |
| before | h264-2160p30-8l-display-0fx-normal | 3.32 | 4.26 / 4.58 | 5.80 / 6.08 | 0.20 / 188.80 / 0.00 | 1.00 / 1.00 | insufficient_samples / 1 | — | — / — / — | 0.19 / 0.10 |
| after | h264-2160p30-8l-display-0fx-normal | 2.59 | 4.38 / 4.71 | 5.84 / 6.17 | 0.45 / 199.09 / 0.00 | 1.00 / 1.00 | insufficient_samples / 9 | — | — / — / — | 0.19 / 0.00 |
| after | hap-1080p30-8l-display-0fx-normal | 4.80 | 1.07 / 1.08 | 1.22 / 1.25 | 24.42 / 0.00 / 1.17 | 1.00 / 1.00 | measured / 1200 | 60.06 | 16.65 / 18.00 / 18.77 | 7.35 / 7.30 |
| after | dxv-1080p30-8l-display-0fx-normal | 16.43 | 1.93 / 1.99 | 2.17 / 2.26 | 25.36 / 0.00 / 2.18 | 1.00 / 1.00 | measured / 1199 | 60.02 | 16.78 / 29.25 / 30.65 | 6.73 / 6.73 |
| before | h264-1080p30-12l-display-0fx-normal | 4.88 | 3.17 / 3.37 | 4.41 / 4.64 | 0.82 / 98.73 / 0.00 | 1.00 / 1.00 | insufficient_samples / 4 | — | — / — / — | 0.29 / 0.00 |
| after | h264-1080p30-12l-display-0fx-normal | 4.84 | 3.46 / 3.60 | 5.29 / 5.43 | 2.18 / 178.45 / 0.00 | 1.00 / 1.00 | measured / 59 | 3.04 | 62.95 / 2170.49 / 3042.72 | 0.44 / 0.05 |
| after | h264-2160p30-12l-display-0fx-normal | 3.15 | 6.31 / 6.46 | 8.35 / 8.49 | 0.45 / 198.82 / 0.00 | 1.00 / 1.00 | insufficient_samples / 1 | — | — / — / — | 0.15 / 0.00 |
| after | hap-1080p30-12l-display-0fx-normal | 5.89 | 1.17 / 1.19 | 1.32 / 1.35 | 33.55 / 0.00 / 0.82 | 1.00 / 1.00 | measured / 1199 | 60.07 | 16.65 / 18.36 / 19.16 | 6.79 / 7.32 |
| after | dxv-1080p30-12l-display-0fx-normal | 22.00 | 2.58 / 2.68 | 2.88 / 3.00 | 34.40 / 0.00 / 1.30 | 1.00 / 1.00 | measured / 1146 | 57.54 | 18.08 / 35.81 / 39.13 | 5.30 / 6.15 |
| before | h264-1080p30-16l-display-0fx-normal | 5.69 | 3.38 / 3.60 | 4.76 / 4.95 | 0.57 / 81.14 / 0.00 | 1.00 / 1.00 | insufficient_samples / 1 | — | — / — / — | 0.24 / 0.14 |
| after | h264-1080p30-16l-display-0fx-normal | 4.73 | 4.35 / 4.41 | 6.62 / 6.69 | 0.55 / 197.27 / 0.00 | 1.00 / 1.00 | insufficient_samples / 5 | — | — / — / — | 0.44 / 0.05 |
| before | h264-2160p30-16l-display-0fx-normal | — | — / — | — / — | — / — / — | — / — | not_established / — | — | — / — / — | — / — |
| after | h264-2160p30-16l-display-0fx-normal | 3.78 | 8.63 / 9.24 | 10.63 / 11.20 | 0.43 / 199.14 / 0.00 | 1.00 / 1.00 | insufficient_samples / 2 | — | — / — / — | 0.10 / 0.10 |
| after | hap-1080p30-16l-display-0fx-normal | 6.59 | 1.27 / 1.28 | 1.41 / 1.44 | 40.09 / 0.00 / 1.55 | 1.00 / 1.00 | measured / 1200 | 60.07 | 16.63 / 18.95 / 20.58 | 5.17 / 6.60 |
| after | dxv-1080p30-16l-display-0fx-normal | 22.12 | 3.08 / 3.10 | 3.41 / 3.47 | 38.00 / 0.00 / 1.50 | 1.00 / 1.00 | measured / 924 | 46.39 | 25.05 / 41.02 / 60.80 | 1.29 / 1.29 |
| after | h264-1080p30-1l-display4k-0fx-normal | 10.99 | 1.34 / 1.36 | 1.64 / 1.69 | 17.64 / 14.64 / 1.00 | 1.00 / 1.00 | measured / 878 | 43.99 | 24.16 / 28.18 / 41.67 | 7.03 / 7.03 |
| after | h264-1080p30-1l-spout-0fx-normal | 10.74 | 1.66 / 1.68 | 2.03 / 2.15 | 7.09 / 19.45 / 1.00 | 1.00 / 1.00 | measured / 1201 | 60.07 | 16.65 / 17.68 / 18.19 | 22.52 / 22.57 |
| before | h264-1080p30-3l-display4k-0fx-normal | 9.09 | 2.12 / 2.18 | 2.77 / 2.87 | 10.90 / 76.60 / 0.00 | 1.00 / 1.00 | measured / 285 | 14.38 | 65.88 / 108.32 / 252.73 | 1.02 / 0.00 |
| after | h264-1080p30-3l-display4k-0fx-normal | 8.07 | 2.21 / 2.26 | 2.91 / 2.97 | 20.73 / 50.27 / 0.18 | 1.00 / 1.00 | measured / 462 | 23.56 | 30.14 / 87.55 / 306.22 | 1.03 / 0.05 |
| before | h264-1080p30-3l-spout-0fx-normal | 18.65 | 2.28 / 2.31 | 3.09 / 3.18 | 13.90 / 50.20 / 1.60 | 1.00 / 1.00 | measured / 1198 | 59.96 | 16.58 / 20.66 / 22.29 | 14.09 / 15.89 |
| after | h264-1080p30-3l-spout-0fx-normal | 15.34 | 2.27 / 2.30 | 3.02 / 3.08 | 11.18 / 47.82 / 1.09 | 1.00 / 1.00 | measured / 1157 | 57.97 | 16.46 / 19.64 / 32.03 | 10.40 / 10.35 |
| after | h264-1080p30-6l-display4k-0fx-normal | 10.13 | 3.66 / 3.75 | 4.89 / 4.98 | 9.00 / 116.91 / 0.00 | 1.00 / 1.00 | measured / 121 | 6.41 | 91.07 / 548.65 / 827.85 | 0.92 / 0.00 |
| after | h264-1080p30-6l-spout-0fx-normal | 12.88 | 3.23 / 3.30 | 4.49 / 4.56 | 5.82 / 137.27 / 0.00 | 1.00 / 1.00 | measured / 396 | 19.81 | 20.24 / 147.66 / 673.68 | 0.92 / 0.00 |
| after | h264-1080p30-8l-display4k-0fx-normal | 9.83 | 4.38 / 4.61 | 6.06 / 6.32 | 6.36 / 146.82 / 0.00 | 1.00 / 1.00 | measured / 74 | 3.80 | 115.52 / 807.93 / 1512.99 | 0.77 / 0.05 |
| after | h264-1080p30-8l-spout-0fx-normal | 7.60 | 3.86 / 3.98 | 5.61 / 5.73 | 2.73 / 165.73 / 0.00 | 1.00 / 1.00 | measured / 86 | 4.41 | 66.67 / 1070.31 / 2451.15 | 0.54 / 0.05 |
| after | h264-1080p30-12l-display4k-0fx-normal | 5.83 | 6.42 / 7.06 | 8.33 / 9.10 | 6.18 / 165.09 / 0.00 | 1.00 / 1.00 | measured / 19 | 1.01 | 679.63 / 2886.21 / 3016.09 | 0.53 / 0.19 |
| after | h264-1080p30-12l-spout-0fx-normal | 3.65 | 4.97 / 5.18 | 7.47 / 7.68 | 1.00 / 187.27 / 0.00 | 1.00 / 1.00 | insufficient_samples / 4 | — | — / — / — | 0.29 / 0.05 |
| after | h264-1080p30-16l-display4k-0fx-normal | 4.66 | 8.29 / 8.97 | 10.50 / 11.15 | 0.71 / 194.71 / 0.00 | 1.00 / 1.00 | insufficient_samples / 5 | — | — / — / — | 0.43 / 0.14 |
| after | h264-1080p30-16l-spout-0fx-normal | 4.19 | 6.34 / 6.81 | 9.04 / 9.51 | 1.09 / 188.27 / 0.00 | 1.00 / 1.00 | insufficient_samples / 2 | — | — / — / — | 0.29 / 0.14 |
| after | h264-1080p30-1l-display-1fx-normal | 5.92 | 1.63 / 1.65 | 2.19 / 2.24 | 6.36 / 14.09 / 0.00 | 1.00 / 1.00 | measured / 1018 | 50.98 | 19.21 / 22.79 / 25.98 | 7.02 / 7.02 |
| before | h264-1080p30-1l-display-3fx-normal | 3.51 | 1.48 / 1.50 | 1.90 / 1.95 | 4.27 / 22.45 / 0.00 | 1.00 / 1.00 | measured / 340 | 17.19 | 58.26 / 70.22 / 76.87 | 1.03 / 0.00 |
| after | h264-1080p30-1l-display-3fx-normal | 3.87 | 1.41 / 1.44 | 1.85 / 1.89 | 4.64 / 17.73 / 0.00 | 1.00 / 1.00 | measured / 412 | 20.72 | 48.28 / 58.09 / 60.54 | 1.03 / 0.00 |
| after | h264-1080p30-3l-display-1fx-normal | 5.20 | 1.82 / 1.85 | 2.49 / 2.55 | 3.36 / 53.27 / 0.00 | 1.00 / 1.00 | measured / 216 | 10.93 | 69.55 / 278.72 / 396.79 | 0.98 / 0.00 |
| before | h264-1080p30-3l-display-3fx-normal | 4.44 | 2.02 / 2.08 | 2.77 / 2.90 | 3.00 / 40.40 / 0.00 | 1.00 / 1.00 | measured / 94 | 4.82 | 190.03 / 291.68 / 433.82 | 0.97 / 0.05 |
| after | h264-1080p30-3l-display-3fx-normal | 3.82 | 1.83 / 1.85 | 2.48 / 2.54 | 3.33 / 56.92 / 0.00 | 1.00 / 1.00 | measured / 103 | 5.26 | 155.46 / 386.49 / 574.39 | 0.98 / 0.00 |
| after | h264-1080p30-6l-display-1fx-normal | 4.65 | 2.39 / 2.44 | 3.31 / 3.36 | 2.64 / 106.36 / 0.00 | 1.00 / 1.00 | measured / 87 | 4.42 | 135.88 / 543.21 / 1123.22 | 0.88 / 0.00 |
| after | h264-1080p30-6l-display-3fx-normal | 3.90 | 2.39 / 2.41 | 3.40 / 3.48 | 2.55 / 122.27 / 0.00 | 1.00 / 1.00 | measured / 47 | 2.44 | 324.40 / 790.47 / 946.59 | 0.83 / 0.00 |
| after | h264-1080p30-8l-display-1fx-normal | 4.83 | 2.77 / 2.85 | 4.09 / 4.21 | 2.36 / 139.55 / 0.00 | 1.00 / 1.00 | measured / 52 | 2.72 | 202.43 / 965.76 / 1732.26 | 0.73 / 0.00 |
| before | h264-1080p30-8l-display-3fx-normal | 6.41 | 2.96 / 3.06 | 4.36 / 4.55 | 2.22 / 107.33 / 0.00 | 1.00 / 1.00 | insufficient_samples / 9 | — | — / — / — | 0.49 / 0.20 |
| after | h264-1080p30-8l-display-3fx-normal | 3.76 | 2.73 / 2.83 | 4.03 / 4.18 | 2.00 / 143.18 / 0.00 | 1.00 / 1.00 | measured / 22 | 1.32 | 451.30 / 1928.61 / 2379.79 | 0.63 / 0.00 |
| after | h264-1080p30-12l-display-1fx-normal | 4.43 | 3.50 / 3.61 | 5.31 / 5.46 | 1.27 / 172.64 / 0.00 | 1.00 / 1.00 | measured / 13 | 0.84 | 550.04 / 2947.64 / 3159.36 | 0.54 / 0.00 |
| after | h264-1080p30-12l-display-3fx-normal | 4.14 | 3.71 / 3.95 | 5.63 / 5.95 | 2.00 / 136.82 / 0.00 | 1.00 / 1.00 | insufficient_samples / 9 | — | — / — / — | 0.53 / 0.05 |
| after | h264-1080p30-16l-display-1fx-normal | 3.92 | 4.44 / 4.67 | 6.77 / 7.10 | 1.09 / 182.82 / 0.00 | 1.00 / 1.00 | insufficient_samples / 5 | — | — / — / — | 0.39 / 0.05 |
| after | h264-1080p30-16l-display-3fx-normal | 4.40 | 4.37 / 4.67 | 6.72 / 6.98 | 1.91 / 154.45 / 0.00 | 1.00 / 1.00 | insufficient_samples / 5 | — | — / — / — | 0.39 / 0.10 |
| after | h264-1080p30-3l-display-0fx-add | 7.24 | 1.45 / 1.49 | 2.09 / 2.22 | 14.09 / 36.00 / 1.18 | 1.00 / 1.00 | measured / 1200 | 60.05 | 16.64 / 19.00 / 19.59 | 21.45 / 21.40 |
| after | h264-1080p30-3l-display-0fx-multiply | 7.42 | 1.46 / 1.50 | 2.10 / 2.21 | 13.45 / 37.09 / 1.00 | 1.00 / 1.00 | measured / 1199 | 60.02 | 16.65 / 19.05 / 20.36 | 21.54 / 21.78 |
| after | h264-1080p30-3l-display-0fx-screen | 7.25 | 1.47 / 1.49 | 2.08 / 2.20 | 13.91 / 36.00 / 0.27 | 1.00 / 1.00 | measured / 1198 | 60.07 | 16.67 / 19.05 / 19.65 | 21.64 / 21.93 |
| after | h264-1080p30-3l-display-0fx-overlay | 7.12 | 1.47 / 1.48 | 2.12 / 2.21 | 14.29 / 36.00 / 0.14 | 1.00 / 1.00 | measured / 1199 | 60.07 | 16.64 / 19.16 / 19.75 | 21.62 / 21.62 |
| after | h264-1080p30-3l-display-0fx-subtract | 7.29 | 1.46 / 1.48 | 2.07 / 2.18 | 13.82 / 34.18 / 0.27 | 1.00 / 1.00 | measured / 1199 | 60.04 | 16.67 / 19.14 / 19.57 | 19.98 / 20.08 |
| after | h264-1080p30-3l-display-0fx-difference | 7.60 | 1.47 / 1.52 | 2.11 / 2.27 | 14.00 / 35.00 / 0.27 | 1.00 / 1.00 | measured / 1199 | 60.06 | 16.67 / 19.15 / 19.89 | 21.99 / 22.23 |
| after | h264-1080p30-3l-display-0fx-lighten | 7.50 | 1.47 / 1.48 | 2.11 / 2.22 | 14.09 / 36.27 / 1.00 | 1.00 / 1.00 | measured / 1199 | 60.04 | 16.65 / 19.19 / 19.62 | 22.35 / 22.97 |
| after | h264-1080p30-3l-display-0fx-darken | 7.30 | 1.46 / 1.48 | 2.09 / 2.19 | 14.45 / 36.82 / 0.73 | 1.00 / 1.00 | measured / 1199 | 60.06 | 16.65 / 19.11 / 19.60 | 22.21 / 22.16 |
| after | h264-1080p30-3l-display-0fx-softlight | 7.29 | 1.48 / 1.50 | 2.11 / 2.23 | 15.18 / 35.64 / 0.18 | 1.00 / 1.00 | measured / 1199 | 60.04 | 16.63 / 19.02 / 19.54 | 21.70 / 21.65 |
| after | h264-1080p30-3l-display-0fx-normal-cpu | 15.67 | 1.43 / 1.44 | 1.82 / 1.83 | 1.27 / 76.91 / 0.00 | 1.00 / 1.00 | measured / 213 | 10.95 | 73.64 / 242.61 / 337.45 | 1.02 / 0.00 |
| before | h264-1080p30-8l-display-0fx-normal-cpu | 12.26 | 2.41 / 2.46 | 3.50 / 3.56 | 0.70 / 168.10 / 0.00 | 1.00 / 1.00 | measured / 34 | 1.79 | 272.37 / 2429.85 / 2785.90 | 0.62 / 0.05 |
| after | h264-1080p30-8l-display-0fx-normal-cpu | 11.86 | 2.35 / 2.42 | 3.42 / 3.54 | 0.64 / 151.82 / 0.00 | 1.00 / 1.00 | measured / 48 | 2.53 | 207.69 / 1155.60 / 2035.10 | 0.72 / 0.10 |
| after | h264-1080p30-16l-display-0fx-normal-cpu | 8.14 | 3.95 / 4.15 | 6.07 / 6.27 | 0.36 / 134.91 / 0.00 | 1.00 / 1.00 | insufficient_samples / 9 | — | — / — / — | 0.48 / 0.14 |
| after | h264-1080p30-3l-display-groups | 12.90 | 1.47 / 1.50 | 2.01 / 2.06 | 9.91 / 35.82 / 1.82 | 1.00 / 1.00 | measured / 1151 | 57.61 | 16.65 / 19.28 / 22.40 | 10.17 / 9.97 |
| before | h264-1080p30-8l-display-groups | 13.39 | 2.33 / 2.43 | 3.50 / 3.60 | 2.90 / 155.60 / 0.50 | 1.00 / 1.00 | measured / 375 | 18.82 | 22.86 / 177.25 / 804.47 | 0.67 / 0.10 |
| after | h264-1080p30-8l-display-groups | 12.17 | 2.30 / 2.39 | 3.45 / 3.58 | 4.18 / 136.45 / 0.27 | 1.00 / 1.00 | measured / 391 | 21.15 | 20.68 / 170.94 / 691.21 | 0.77 / 0.05 |
| after | h264-1080p30-16l-display-groups | 4.24 | 3.86 / 4.07 | 6.00 / 6.25 | 0.45 / 186.09 / 0.00 | 1.00 / 1.00 | insufficient_samples / 6 | — | — / — / — | 0.39 / 0.10 |

## Planned coverage still incomplete or unsupported

| Phase | Case | Status | Reports | Evidence |
| --- | --- | --- | --- | --- |
| after | h264-1080p30-1l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |
| after | h264-1080p30-3l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |
| after | h264-1080p30-6l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |
| after | h264-1080p30-8l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |
| after | h264-1080p30-12l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |
| after | h264-1080p30-16l-ndi-0fx-normal | unsupported | 0 | after-build.json declares ndiEnabled=false |

## Invalid or incomplete evidence

| Phase | Case | Status | Issues | Workload / restore / stop error |
| --- | --- | --- | --- | --- |
| after | h264-2160p30-6l-display-0fx-normal | partial | partial_display_1_native_work_pending_warmup |  |
| before | h264-2160p30-8l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-2160p30-8l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| before | h264-1080p30-12l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-2160p30-12l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| before | h264-1080p30-16l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| before | h264-2160p30-16l-display-0fx-normal | workload_failed | workload_failed_or_not_completed; instrumentation_not_valid; missing_composition_evidence; missing_monitor_fps; incomplete_process_tree_or_gpu_measurements; incomplete_exact_process_single_swapchain_presentmon; missing_expected_display_health; incomplete_display_None_cadence; missing_display_None_prepare_distribution; incomplete_display_None_native_validation; missing_display_None_observed_native_work; incomplete_display_None_decode_instrumentation | native_backend_startup_deadline |
| after | h264-2160p30-16l-display-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-1l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| after | h264-1080p30-3l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| after | h264-1080p30-6l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| after | h264-1080p30-8l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| after | h264-1080p30-12l-display4k-0fx-normal | partial | partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-12l-spout-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-12l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| after | h264-1080p30-16l-display4k-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-spout-0fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-ndi-0fx-normal | unsupported | after-build.json declares ndiEnabled=false |  |
| before | h264-1080p30-8l-display-3fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-8l-display-3fx-normal | partial | partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-12l-display-1fx-normal | partial | partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-12l-display-3fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-display-1fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-display-3fx-normal | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| before | h264-1080p30-8l-display-0fx-normal-cpu | partial | partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-display-0fx-normal-cpu | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
| after | h264-1080p30-16l-display-groups | partial | instrumentation_partial_presentmon_insufficient_samples; presentmon_insufficient_samples; partial_display_1_native_work_pending_warmup |  |
