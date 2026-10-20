#!/usr/bin/env python3
"""Render boundary-based Task20 findings from the curated measurement CSV.

No remote access, third-party dependencies, raw error strings, or primary-cause
inference. Rerun against the refreshed CSV; failed attempts remain excluded.
"""

import argparse
import csv
import hashlib
import io
import json
import math
import re
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path


MEASURED = {"complete", "partial"}
MISSING_SUPPORTED = {"missing_case", "missing_supported"}
ESTABLISHED_RESULTS = {"established_pass", "established_miss"}
CASE_RESULTS = ("established_pass", "established_miss", "not_established", "unsupported", "missing_supported")
NUMERIC_FIELDS = (
    "target_fps", "elapsed_seconds", "delivered_frames_delta", "delivered_fps",
    "missed_fraction", "cadence_p99_ms", "cadence_sample_count",
    "cadence_window_span_seconds_derived", "prepare_p50_ms", "prepare_p95_ms",
    "prepare_p99_ms", "prepare_sample_count", "native_validation_frame_count",
    "native_validation_average_ms", "native_validation_max_ms",
    "native_validation_deadline_miss_fraction", "native_warmup_remaining",
    "native_work_observation_mean_ms_approx", "native_work_observation_frame_delta",
    "decode_demand_mean_wall_ms", "decode_demand_call_count_delta",
    "decoder_session_opens_delta", "decoder_working_set_evictions_delta",
    "decoder_session_resets_delta", "decoder_failures_delta",
    "decoder_last_hardware_sessions", "decoder_last_sessions",
    "cpu_percent_of_32_logical_mean", "cpu_interval_coverage",
    "private_bytes_peak", "working_set_bytes_peak", "gpu_3d_percent_mean",
    "gpu_video_codec_percent_mean", "gpu_video_codec_count_mean",
    "gpu_video_codec_coverage", "presentmon_matching_rows", "presentmon_fps",
    "program_monitor_fps", "preview_monitor_fps",
)
STATUS_FIELDS = (
    "measurement_status", "cadence_status", "native_status", "envelope_status",
    "native_validation_state", "native_work_status", "decode_status",
    "presentmon_status", "cpu_status", "gpu_provider_status",
)


def number(row, key):
    try:
        value = float(row.get(key, ""))
        return value if math.isfinite(value) else None
    except (TypeError, ValueError):
        return None


def fmt(value, places=4):
    if value is None:
        return "unavailable"
    rendered = f"{value:.{places}f}"
    return rendered.rstrip("0").rstrip(".") if "." in rendered else rendered


def val(row, key, places=4):
    return fmt(number(row, key), places)


def percent(row, key, places=4):
    value = number(row, key)
    return "unavailable" if value is None else f"{fmt(value * 100, places)}%"


def code_list(row, key):
    """Emit only reason codes; intentionally discard paths and prose errors."""
    try:
        items = json.loads(row.get(key) or "[]")
    except (TypeError, ValueError):
        return ["unstructured_detail_omitted"]
    if not isinstance(items, list):
        return ["unstructured_detail_omitted"]
    codes = []
    for item in items:
        if isinstance(item, str) and re.fullmatch(r"[a-z][a-z0-9_]*", item):
            codes.append(item)
        elif isinstance(item, str) and "ndiEnabled=false" in item:
            codes.append("ndi_not_built")
        else:
            codes.append("unstructured_detail_omitted")
    return sorted(set(codes))


def key(row):
    return (row["tag"], row["output_kind"], row["output_id"])


def identity(row):
    return f"{row['tag']} · {row['output_kind']} {row['output_id']}"


def measured(row):
    return (row.get("measurement_status") in MEASURED
            and row.get("envelope_status") in {"pass", "miss"}
            and row.get("workload_ok", "").lower() != "false")


def case_result(group):
    """Return one mutually exclusive result for the expected outputs of a case."""
    statuses = {r.get("measurement_status") for r in group}
    if statuses & MISSING_SUPPORTED:
        return "missing_supported"
    if "unsupported" in statuses:
        return "unsupported"
    if not all(measured(r) for r in group):
        return "not_established"
    return "established_miss" if any(r["envelope_status"] == "miss" for r in group) else "established_pass"


def grouped_cases(rows):
    grouped = defaultdict(list)
    for row in rows:
        grouped[(row["phase"], row["tag"])].append(row)
    return grouped


def native_evidence(row):
    state = row.get("native_validation_state") or "unavailable"
    if number(row, "native_validation_frame_count") is None:
        return f"Native gate {row.get('native_status') or 'unavailable'}"
    return (f"Native {state}, n={val(row, 'native_validation_frame_count', 0)}; "
            f"validation mean/max {val(row, 'native_validation_average_ms')}/"
            f"{val(row, 'native_validation_max_ms')} ms; work misses "
            f"{percent(row, 'native_validation_deadline_miss_fraction')}")


def output_evidence(row):
    return (f"{val(row, 'delivered_fps', 6)} fps; cadence "
            f"{row.get('cadence_status') or 'unavailable'}, missed "
            f"{percent(row, 'missed_fraction')}, interval p99 "
            f"{val(row, 'cadence_p99_ms')} ms. Prepare p50/p95/p99 "
            f"{val(row, 'prepare_p50_ms')}/{val(row, 'prepare_p95_ms')}/"
            f"{val(row, 'prepare_p99_ms')} ms, n={val(row, 'prepare_sample_count', 0)}. "
            f"{native_evidence(row)}.")


def classify(row):
    """Select an observed boundary, never infer an isolated root cause."""
    target = number(row, "target_fps")
    budget = 1000 / target if target and target > 0 else None
    prep = number(row, "prepare_p95_ms")
    demand = number(row, "decode_demand_mean_wall_ms")
    demand_valid = row.get("decode_status") == "measured" and demand is not None
    cadence_fails = row.get("cadence_status") == "fail"
    native_fails = row.get("native_status") == "fail"

    if not cadence_fails and not native_fails:
        boundary = "Case-level miss; this output has no independently established failing boundary"
        approach = ("Use the failing sibling output/native gate for the case finding; "
                    "retain this output's own cadence verdict.")
    elif budget is not None and prep is not None and prep > budget:
        if demand_valid and demand > budget:
            boundary = "Synchronous decoder-demand latency and preparation exceed the frame interval"
            approach = ("Record matched demand/preparation timings, demand waits and lookahead "
                        "availability; inspect bounded reset-reason/target-gap summaries. "
                        "The demand timer includes waiting and does not measure background work.")
        else:
            boundary = "Preparation tail exceeds the frame interval"
            approach = ("Correlate per-frame demand, conversion/transfer and remaining preparation "
                        "timings. A low demand mean cannot exclude demand-tail stalls; "
                        "do not estimate other preparation by subtracting it from p95.")
    elif native_fails:
        boundary = ("Native work window fails despite preparation p95 within budget"
                    if budget is not None and prep is not None
                    else "Native work validation fails; finer localization unavailable")
        approach = ("Inspect matched preparation tails and final composition/present timings, "
                    "including possible present/vsync waits. Preparation p95 does not clear p99 "
                    "or maxima; native work does not isolate GPU execution.")
    else:
        boundary = "Delivered cadence fails; finer localization is not established"
        approach = ("Correlate health responses with native work and the exact PresentMon stream; "
                    "measure response latency, scheduling and the relevant send/present boundary.")

    evictions = number(row, "decoder_working_set_evictions_delta")
    if evictions is not None and evictions > 0:
        boundary += "; cache eviction activity also observed"
        approach += " Verify active owners, cache occupancy and batch-admission fallback."

    source = (row.get("source") or "").lower()
    path = row.get("composition_path") or "unavailable"
    if source.startswith("hap") and number(row, "layers") == 1 and "CPU" in path:
        approach += (" Profile the deliberate single-layer CPU path. HAP multi-layer cases "
                     "use a different observed composition path and are not a pure layer-count comparison.")
    if number(row, "fx") is not None and number(row, "fx") > 0:
        approach += (" Verify actual ISF execution, then compare the matched FX0 case; "
                     "the configured effect count alone does not isolate shader or copy cost.")
    if "group" in path.lower() or row.get("groups") not in (None, "", "False", "false", "0"):
        approach += (" Time group isolation and outer composition separately; "
                     "a group variant can change execution paths and semantics.")
    if row.get("output_kind") != "display":
        approach += " Respect the sender preparation/readback boundary and time SDK send separately."

    limitations = []
    if row.get("measurement_status") == "partial":
        limitations.append("Partial evidence: " + ", ".join(code_list(row, "partial_evidence")))
    if number(row, "prepare_sample_count") is not None and number(row, "prepare_sample_count") < 600:
        limitations.append("Short rolling history; percentiles are descriptive and may include warmup")
    if row.get("native_status") == "not_established":
        limitations.append("Native acceptance not established")
    return {
        "identity": identity(row), "tag": row["tag"], "output_kind": row["output_kind"],
        "output_id": row["output_id"], "boundary": boundary,
        "confidence": "observed boundary; isolated component cause unestablished",
        "evidence": output_evidence(row), "approach": approach,
        "composition_path": path, "limitations": limitations,
        "statuses": {k: row.get(k) or None for k in STATUS_FIELDS},
        "numeric_evidence": {k: number(row, k) for k in NUMERIC_FIELDS},
        "reason_codes": {k: code_list(row, k) for k in
                         ("measurement_issues", "partial_evidence", "cadence_reasons", "native_reasons")},
    }


def cell(value):
    return str(value).replace("|", "\\|").replace("\r", " ").replace("\n", " ")


def table(headers, rows):
    rows = list(rows)
    if not rows:
        return "No rows established in this snapshot.\n"
    return "\n".join([
        "| " + " | ".join(map(cell, headers)) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
        *("| " + " | ".join(map(cell, row)) + " |" for row in rows),
    ]) + "\n"


def pairs(rows):
    valid_cases = {k for k, group in grouped_cases(rows).items()
                   if case_result(group) in ESTABLISHED_RESULTS}
    before = {key(r): r for r in rows if r["phase"] == "before"
              and (r["phase"], r["tag"]) in valid_cases and measured(r)}
    return [(before[key(r)], r) for r in rows
            if r["phase"] == "after" and (r["phase"], r["tag"]) in valid_cases
            and measured(r) and key(r) in before]


def counter_tuple(row):
    return "/".join(val(row, k, 0) for k in
                    ("decoder_session_opens_delta", "decoder_working_set_evictions_delta",
                     "decoder_session_resets_delta"))


def session_tuple(row):
    return "/".join(val(row, k, 0) for k in
                    ("decoder_last_hardware_sessions", "decoder_last_sessions"))


def controls_table(matched):
    result = []
    for before, after in matched:
        layers = number(after, "layers")
        if number(after, "fx") != 0 or layers is None or layers > 8:
            continue
        b, a = number(before, "delivered_fps"), number(after, "delivered_fps")
        ratio = a / b if b and a is not None else None
        path_same = before.get("composition_path") == after.get("composition_path")
        churn_zero = all(number(r, k) == 0 for r in (before, after) for k in
                         ("decoder_session_opens_delta", "decoder_working_set_evictions_delta"))
        flags = []
        if ratio is not None and abs(ratio - 1) >= .10:
            flags.append("≥10% FPS difference: repeat before attributing")
        if before.get("envelope_status") != after.get("envelope_status"):
            flags.append("envelope verdict changed")
        if not path_same:
            flags.append("observed composition path changed")
        if churn_zero and path_same:
            flags.append("same observed path; zero opens/evictions in both intervals")
        else:
            flags.append("check retention/admission and actual executed paths")
        result.append([
            identity(after), f"{fmt(b, 6)} → {fmt(a, 6)} ({fmt(ratio, 3)}×)",
            f"{val(before, 'prepare_p95_ms')} → {val(after, 'prepare_p95_ms')}",
            f"{counter_tuple(before)} → {counter_tuple(after)}", "; ".join(flags),
        ])
    return result


def missing_cases(after):
    result = []
    for (_, tag), group in grouped_cases(after).items():
        result_kind = case_result(group)
        if result_kind in ESTABLISHED_RESULTS:
            continue
        statuses = sorted({r["measurement_status"] for r in group})
        codes = sorted({code for r in group for k in
                        ("measurement_issues", "native_reasons", "presentmon_reasons")
                        for code in code_list(r, k)})
        if result_kind == "unsupported":
            action = "NotBuilt; no cadence or throughput conclusion"
        elif result_kind == "missing_supported":
            action = "Missing supported case; no recorded attempt in this CSV; coverage pending"
        elif "workload_failed" in statuses:
            action = "Canonical attempt recorded; workload not established; numeric performance conclusions excluded"
        else:
            action = "Canonical attempt recorded; envelope not established; retain the reason codes"
        result.append({
            "tag": tag, "outputs": [f"{r['output_kind']} {r['output_id']}" for r in group],
            "case_result": result_kind,
            "measurement_statuses": statuses, "reason_codes": codes,
            "envelope_status": "not_established", "action": action,
        })
    return result


def render(rows, source_hash):
    after = [r for r in rows if r["phase"] == "after"]
    groups = grouped_cases(after)
    results = {tag: case_result(group) for (_, tag), group in groups.items()}
    result_counts = {result: sum(value == result for value in results.values()) for result in CASE_RESULTS}
    established = [r for r in after if results[r["tag"]] in ESTABLISHED_RESULTS and measured(r)]
    miss_rows = [r for r in established if r["envelope_status"] == "miss"]
    findings = [classify(r) for r in miss_rows]
    unavailable = missing_cases(after)
    matched = pairs(rows)
    counts = {
        "after_cases": len(results),
        "supported_cases": len(results) - result_counts["unsupported"],
        "attempted_supported_cases": sum(result_counts[k] for k in
                                         ("established_pass", "established_miss", "not_established")),
        "established_cases": result_counts["established_pass"] + result_counts["established_miss"],
        "established_pass_cases": result_counts["established_pass"],
        "established_miss_cases": result_counts["established_miss"],
        "not_established_cases": result_counts["not_established"],
        "miss_output_rows": len(findings),
        "missing_supported_cases": result_counts["missing_supported"],
        "pending_cases": result_counts["missing_supported"],
        "unsupported_cases": result_counts["unsupported"],
        "failed_workload_cases": sum(results[tag] == "not_established"
                                     and any(r["measurement_status"] == "workload_failed" for r in group)
                                     for (_, tag), group in groups.items()),
    }
    interim = counts["missing_supported_cases"] > 0
    snapshot_label = ("Interim coverage snapshot" if interim
                      else "Curated matrix snapshot — all listed supported cases attempted")
    sections = [
        "# Task20 per-case observed boundaries\n",
        f"**{snapshot_label}**; CSV SHA-256 `{source_hash}`. "
        f"{counts['established_cases']} established after cases: "
        f"{counts['established_pass_cases']} envelope passes and {counts['established_miss_cases']} misses. "
        f"{counts['not_established_cases']} attempted cases remain not established, "
        f"{counts['missing_supported_cases']} supported cases have no recorded attempt, "
        f"and {counts['unsupported_cases']} cases are unsupported. "
        "Case counts use distinct tags; output rows are not additional cases.\n",
        "## Evidence boundaries\n",
        "The labels identify observed timing boundaries, not isolated causes or a serial order of components. "
        "Verdicts are retained from the curated CSV. Counter-delta delivered FPS uses monotonic timestamps "
        "taken **before awaiting health responses**; response-end latency was not recorded, so unequal "
        "response latency can affect the nominal measurement interval. Corroborate with native counters "
        "and the exact PID/swap-chain PresentMon stream; tiny or absent traces remain partial evidence. "
        "Health measures submission/accepted-return cadence, not physical scanout or receiver delivery; "
        "30 fps media does not become 60 unique source images.\n",
        "Preparation histories contain at most 600 samples and cadence histories at most 599 intervals; "
        "rolling windows can include warmup and differ from the observed counter interval. Preparation "
        "includes synchronous demand but excludes final Display GPU composition/present and pacing. "
        "Sender preparation uses its separate readback/send boundary. Native validation mean/max/miss "
        "fraction belong to its own frame window; unfinished validation is not acceptance. Approximate "
        "native observation means reconstructed from rounded lifetime counters are a separate measure. "
        "Decoder-demand mean includes synchronous waits and excludes background lookahead. "
        "Never subtract a mean from p95 to assign component cost.\n",
        "## Every established after envelope miss\n",
        table(["Case / output", "First observed boundary", "Numerical evidence", "Proposed approach / limits"],
              [[f["identity"], f["boundary"], f["evidence"],
                f["approach"] + (" " + "; ".join(f["limitations"]) if f["limitations"] else "")]
               for f in findings]),
        "## Decoder and resource context for the misses\n",
        "HW/total are last observed **instrumented native decoder** session counts, not process-wide "
        "session totals. Zero libav sessions for HAP does not mean no decoding or no GPU use. "
        "Open/eviction/reset values are observation deltas; a matching-session reset rewinds/flushes "
        "in place and is not a fresh open or eviction. Reset counts alone do not establish why a frame "
        "stalled. CPU is process-tree time normalized to 32 logical processors; low aggregate CPU "
        "does not exclude a busy thread, worker pool, or waits. GPU numbers below are raw engine-instance "
        "sums, not device-wide utilization percentages; Video Codec Engine is not a decode counter. "
        "They may exceed 100. Missing categories remain unavailable. Monitor FPS counts image paints.\n",
        table(["Case / output", "Observed composition", "Demand mean ms / calls", "HW / total; Δopen / evict / reset",
               "CPU % of 32", "GPU raw 3D / Video Codec; codec instances / coverage", "Program / Preview paint fps"],
              [[identity(r), r.get("composition_path") or "unavailable",
                f"{val(r, 'decode_demand_mean_wall_ms', 6)} / {val(r, 'decode_demand_call_count_delta', 0)}",
                f"{session_tuple(r)}; {counter_tuple(r)}", val(r, "cpu_percent_of_32_logical_mean"),
                f"{val(r, 'gpu_3d_percent_mean')} / {val(r, 'gpu_video_codec_percent_mean')}; "
                f"{val(r, 'gpu_video_codec_count_mean')} / {percent(r, 'gpu_video_codec_coverage')}",
                f"{val(r, 'program_monitor_fps')} / {val(r, 'preview_monitor_fps')}"] for r in miss_rows]),
        "## Matched cases above the former cache capacity\n",
        "With verified no-ISF execution, these pairs target capacity/admission in principle. "
        "Eliminated eviction activity is directly observed; the change in FPS is a single-pair estimate "
        "subject to window, response-latency and background-load differences. Private-byte peaks are "
        "allocation/commit measurements, not resident RAM or proof of memory pressure.\n",
    ]
    cache_pairs = [(b, a) for b, a in matched if number(a, "fx") == 0
                   and number(a, "layers") is not None and number(a, "layers") > 8
                   and a["output_kind"] == "display"]
    sections.append(table(
        ["Case", "FPS before → after", "Δopen / eviction before → after", "HW / total before → after",
         "Private-byte peak GiB before → after", "Remaining observed boundary"],
        [[identity(a), f"{val(b, 'delivered_fps', 6)} → {val(a, 'delivered_fps', 6)}",
          f"{val(b, 'decoder_session_opens_delta', 0)}/{val(b, 'decoder_working_set_evictions_delta', 0)} → "
          f"{val(a, 'decoder_session_opens_delta', 0)}/{val(a, 'decoder_working_set_evictions_delta', 0)}",
          f"{session_tuple(b)} → {session_tuple(a)}",
          f"{fmt(number(b, 'private_bytes_peak') / 2**30 if number(b, 'private_bytes_peak') is not None else None)} → "
          f"{fmt(number(a, 'private_bytes_peak') / 2**30 if number(a, 'private_bytes_peak') is not None else None)}",
          classify(a)["boundary"] if a["envelope_status"] == "miss" else "No envelope miss established"]
         for b, a in cache_pairs]))
    sections.extend([
        "## Matched ≤8-layer no-FX controls\n",
        "These comparisons flag attribution uncertainty. The ≥10% FPS flag is a descriptive review aid, "
        "not a statistical significance test or acceptance policy. Where both intervals have zero "
        "opens/evictions and the same observed composition path, a large change still needs a mechanism "
        "consistent with actual cache admission and executed code; it cannot automatically be credited "
        "to capacity 16 or ISF borrowing. Repeat matched cases and account for asymmetric before/after "
        "background-build monitoring. An unchanged aggregate counter does not prove identical hidden state.\n",
        table(["Matched output", "FPS before → after", "Prepare p95 ms before → after",
               "Δopen / evict / reset before → after", "Interpretation"], controls_table(matched)),
        "## HAP path comparison\n",
        "The source-confirmed single-layer eligibility rule uses CPU composition; multi-layer cases "
        "with observed GPU buffers use a different path. A one-layer native miss alongside multi-layer "
        "passes is therefore nonmonotonic across these cases, not evidence of a monotonic layer ceiling "
        "or a codec-only cause. Profile the single-layer path before proposing a policy change.\n",
        table(["Case", "Composition", "FPS / cadence", "Prepare p95 ms", "Native window"],
              [[identity(r), r.get("composition_path") or "unavailable",
                f"{val(r, 'delivered_fps', 6)} / {r.get('cadence_status')}", val(r, "prepare_p95_ms"), native_evidence(r)]
               for r in established if (r.get("source") or "").lower().startswith("hap")]),
        "## Unsupported, pending, or otherwise not established\n",
        "No performance numbers from these rows are used above. Attempted workloads with failed or "
        "unestablished measurements remain explicit outcomes of the canonical matrix; they do not "
        "imply that coverage is pending or that another rerun is promised. Only missing_case or "
        "missing_supported markers make this report interim. "
        "Raw error/restore/stop messages and Windows paths are intentionally omitted; only reason codes appear.\n",
        table(["Case", "Expected outputs", "Case result", "Measurement status", "Reason codes", "Interpretation"],
              [[u["tag"], ", ".join(u["outputs"]), u["case_result"], ", ".join(u["measurement_statuses"]),
                ", ".join(u["reason_codes"]) or "none supplied", u["action"]] for u in unavailable]),
    ])
    payload = {
        "schema_version": 2, "generated_utc": datetime.now(timezone.utc).isoformat(),
        "csv_sha256": source_hash, "interim": interim, "counts": counts,
        "coverage_complete": not interim,
        "case_result_counts": result_counts,
        "cases": [{"tag": tag, "case_result": results[tag],
                   "outputs": [f"{r['output_kind']} {r['output_id']}" for r in group]}
                  for (_, tag), group in groups.items()],
        "findings": findings, "not_established": unavailable,
    }
    return "\n".join(sections), payload


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--csv", type=Path, default=Path("/tmp/task20-measurements-current.csv"))
    parser.add_argument("--markdown", type=Path, default=Path("/tmp/task20-bottleneck-cases.md"))
    parser.add_argument("--json", type=Path, default=Path("/tmp/task20-bottleneck-cases.json"))
    args = parser.parse_args()
    raw = args.csv.read_bytes()
    reader = csv.DictReader(io.StringIO(raw.decode("utf-8-sig"), newline=""))
    required = {"phase", "tag", "output_kind", "output_id", "measurement_status", "envelope_status"}
    absent = required - set(reader.fieldnames or [])
    if absent:
        raise SystemExit("Missing required CSV columns: " + ", ".join(sorted(absent)))
    rows = list(reader)
    seen = Counter((r["phase"], *key(r)) for r in rows)
    if any(count > 1 for count in seen.values()):
        raise SystemExit("Duplicate phase/case/output rows: supply a curated CSV with one retained attempt per key")
    markdown, payload = render(rows, hashlib.sha256(raw).hexdigest())
    args.markdown.write_text(markdown, encoding="utf-8")
    args.json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"markdown": str(args.markdown), "json": str(args.json),
                      "interim": payload["interim"], **payload["counts"]}, ensure_ascii=False))


if __name__ == "__main__":
    main()
