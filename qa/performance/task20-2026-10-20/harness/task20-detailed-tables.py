#!/usr/bin/env python3
"""Render detailed Task20 tables from curated CSV and the declared policy only.

Usage: task20-detailed-tables.py CSV POLICY OUTPUT [--metrics-module PATH]
The existing metrics.write_tables formatter is reused; no gates are recalculated.
"""

import argparse
import csv
import importlib.util
import json
import math
import re
from pathlib import Path


TEXT_FIELDS = set("""
phase tag output_kind measurement_status cadence_status native_status envelope_status
envelope_basis case_cadence_status case_native_status source blend composition_path
cadence_boundary prepare_boundary native_validation_state native_work_status
native_work_partial_reason decode_status decoder_session_resets_reason cpu_accounting
cpu_status memory_status gpu_provider_status gpu_video_codec_boundary presentmon_status
presentmon_identity_status presentmon_swapchain executable_sha256 fixture_sha256_before
fixture_sha256_after case_file error restore_error stop_error
""".split())
INTEGER_FIELDS = set("""
output_id layers fx cadence_sample_count cadence_interval_count prepare_sample_count
native_validation_frame_count native_warmup_remaining native_deadline_miss_count
native_width native_height gpu_video_codec_valid_samples gpu_valid_samples usage_samples
presentmon_process_id presentmon_matching_rows presentmon_minimum_rows presentmon_interval_count
""".split())
NUMBER_FIELDS = set("""
target_fps elapsed_seconds delivered_frames_delta delivered_fps target_ratio
missed_intervals_delta missed_fraction cadence_p99_target_intervals cadence_p50_ms
cadence_p95_ms cadence_p99_ms cadence_window_span_seconds_derived cadence_rolling_reported_fps
prepare_p50_ms prepare_p95_ms prepare_p99_ms native_validation_average_ms
native_validation_max_ms native_validation_deadline_miss_fraction native_work_observation_mean_ms_approx
native_work_observation_frame_delta native_lifetime_deadline_misses_delta decode_demand_wall_us_delta
decode_demand_call_count_delta decode_demand_mean_wall_ms decoder_session_opens_delta
decoder_working_set_evictions_delta decoder_session_resets_delta decoder_failures_delta
decoder_lookahead_hits_delta decoder_last_hardware_sessions decoder_last_sessions
cpu_percent_of_32_logical_mean cpu_percent_of_32_logical_peak_interval cpu_interval_coverage
working_set_bytes_mean working_set_bytes_peak private_bytes_mean private_bytes_peak
gpu_3d_percent_mean gpu_decode_percent_mean gpu_video_codec_percent_mean gpu_video_codec_percent_peak
gpu_video_codec_count_mean gpu_video_codec_coverage gpu_copy_percent_mean gpu_provider_coverage
gpu_3d_coverage gpu_decode_coverage gpu_copy_coverage gpu_3d_count_mean gpu_decode_count_mean
gpu_copy_count_mean usage_span_seconds presentmon_fps presentmon_p50_ms presentmon_p95_ms
presentmon_p99_ms presentmon_gpu_busy_ms_mean presentmon_in_present_api_ms_mean
presentmon_until_displayed_ms_mean program_monitor_fps preview_monitor_fps
""".split())
BOOLEAN_FIELDS = set("""
workload_ok instrumentation_valid background_build_sampling_available background_build_overlap
requires_stop_before_further_cases cpu_requested groups native_frame_budget_pass
process_membership_changed gpu_provider_available
""".split())
LIST_FIELDS = set("""
partial_evidence measurement_issues cadence_reasons native_reasons usage_reasons presentmon_reasons
""".split())
POLICY_FIELDS = {
    "declaredBeforeBenchmark", "targetOutputFps", "cadenceMinimumTargetRatio",
    "missedFractionMaximum", "cadenceP99MaximumTargetIntervals", "nativeBudgetRequired",
    "logicalProcessors", "warmupSeconds", "nominalObservationSeconds", "interpretation", "nativeBudget",
}
NATIVE_POLICY_FIELDS = {"frames", "averageMaxUs", "singleWorkMaxUs", "deadlineMissFractionMax"}
PRIVATE = re.compile(
    r"\b[a-z]:[\\/]|[\\/]+Users[\\/]+|[\\/]+(?:home|root)[\\/]+|"
    r"\\\\[a-z0-9_.-]+[\\/]|(?:https?|file)://|localstorage|original[-_]?storage|"
    r"raw[-_]?storage|errorui|[\"'](?:loaded|controls|recent)[\"']\s*:", re.I
)
TAG = re.compile(r"[a-z0-9][a-z0-9_-]{0,159}\Z")


def safe_text(value, where):
    if not isinstance(value, str) or PRIVATE.search(value):
        raise ValueError(f"{where}: uncurated path/private-state content; use the reviewed CSV exporter")
    if "Traceback (most recent call last)" in value or value.startswith("Error: Command failed:"):
        raise ValueError(f"{where}: uncurated command/traceback")
    return value


def numeric(value, where, integer=False):
    if value == "":
        return None
    if isinstance(value, bool):
        raise ValueError(f"{where}: boolean in numeric field")
    try:
        result = float(value)
    except (ValueError, TypeError):
        raise ValueError(f"{where}: expected finite numeric value") from None
    if not math.isfinite(result):
        raise ValueError(f"{where}: nonfinite numeric value")
    if integer:
        if not result.is_integer():
            raise ValueError(f"{where}: expected integer")
        return int(result)
    return result


def parse_field(key, value, where):
    if key in INTEGER_FIELDS | NUMBER_FIELDS:
        return numeric(value, where, key in INTEGER_FIELDS)
    if key in BOOLEAN_FIELDS:
        if value == "":
            return None
        if value.lower() not in ("true", "false"):
            raise ValueError(f"{where}: expected true, false or blank")
        return value.lower() == "true"
    if key in LIST_FIELDS:
        if value == "":
            return []
        try:
            result = json.loads(value)
        except json.JSONDecodeError:
            raise ValueError(f"{where}: expected JSON string list") from None
        if not isinstance(result, list) or not all(isinstance(item, str) for item in result):
            raise ValueError(f"{where}: expected JSON string list")
        return result
    return value or None


def load_rows(path):
    allowed = TEXT_FIELDS | INTEGER_FIELDS | NUMBER_FIELDS | BOOLEAN_FIELDS | LIST_FIELDS
    required = {"phase", "tag", "output_kind", "output_id", "measurement_status", "envelope_status", "measurement_issues"}
    rows, identities = [], set()
    with path.open(encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        if not required.issubset(reader.fieldnames or []):
            raise ValueError("CSV is missing required identity/status columns")
        if len(reader.fieldnames) != len(set(reader.fieldnames)):
            raise ValueError("CSV contains duplicate column names")
        for index, original in enumerate(reader, 1):
            if None in original or any(value is None for value in original.values()):
                raise ValueError(f"CSV row {index}: malformed column count")
            for key, value in original.items():
                safe_text(value, f"CSV row {index} field {key}")
            row = {key: parse_field(key, original.get(key, ""), f"CSV row {index} field {key}") for key in allowed}
            if row["phase"] not in ("before", "after") or not TAG.fullmatch(row["tag"] or ""):
                raise ValueError(f"CSV row {index}: invalid phase/tag")
            if row["case_file"] and row["case_file"] != f"target/qa/task20d/{row['phase']}-{row['tag']}.json":
                raise ValueError(f"CSV row {index}: unexpected curated case reference")
            identity = tuple(row[key] for key in ("phase", "tag", "output_kind", "output_id"))
            if identity in identities:
                raise ValueError(f"CSV row {index}: duplicate output identity")
            identities.add(identity)
            if row["envelope_status"] == "pass" and (
                row["measurement_status"] != "complete" or row["workload_ok"] is not True
                or row["instrumentation_valid"] is not True or row["background_build_overlap"] is True
                or row["requires_stop_before_further_cases"] is True
            ):
                raise ValueError(f"CSV row {index}: contradictory pass status; classifications must be fixed upstream")
            rows.append(row)
    return rows


def load_policy(path):
    policy = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(policy, dict) or set(policy) != POLICY_FIELDS:
        raise ValueError("Policy fields do not match the reviewed Task20 policy")
    if not isinstance(policy["nativeBudget"], dict) or set(policy["nativeBudget"]) != NATIVE_POLICY_FIELDS:
        raise ValueError("Native policy fields do not match the reviewed policy")
    for key in ("declaredBeforeBenchmark", "nativeBudgetRequired"):
        if not isinstance(policy[key], bool):
            raise ValueError(f"Policy {key}: expected boolean")
    safe_text(policy["interpretation"], "Policy interpretation")
    for key in POLICY_FIELDS - {"declaredBeforeBenchmark", "nativeBudgetRequired", "interpretation", "nativeBudget"}:
        numeric(policy[key], f"Policy {key}")
    for key in NATIVE_POLICY_FIELDS:
        numeric(policy["nativeBudget"][key], f"Native policy {key}", key == "frames")
    if not (0 < policy["cadenceMinimumTargetRatio"] <= 1 and 0 <= policy["missedFractionMaximum"] < 1
            and policy["cadenceP99MaximumTargetIntervals"] > 0):
        raise ValueError("Policy cadence thresholds are outside valid ranges")
    return policy


def unique(values):
    return list(dict.fromkeys(values))


def synthetic_summary(policy, rows):
    cases = {}
    for row in rows:
        key = (row["phase"], row["tag"])
        entry = cases.setdefault(key, {"phase": key[0], "tag": key[1],
            "measurementStatus": row["measurement_status"], "measurementIssues": [],
            "error": [], "restoreError": [], "stopError": [], "hasCaseReference": False})
        if entry["measurementStatus"] != row["measurement_status"]:
            raise ValueError("One case has inconsistent measurement statuses across outputs")
        entry["measurementIssues"].extend(row["measurement_issues"])
        entry["hasCaseReference"] |= bool(row["case_file"])
        for source, destination in (("error", "error"), ("restore_error", "restoreError"), ("stop_error", "stopError")):
            error = row[source]
            if error not in (None, "", "null", "false", "False", "[]", "{}"):
                entry[destination].append(error)
    clean_cases, coverage = [], []
    for entry in cases.values():
        clean = {key: entry[key] for key in ("phase", "tag", "measurementStatus")}
        clean["measurementIssues"] = unique(entry["measurementIssues"])
        for key in ("error", "restoreError", "stopError"):
            clean[key] = "; ".join(unique(entry[key])) or None
        clean_cases.append(clean)
        if entry["measurementStatus"] in ("missing_case", "unsupported"):
            coverage.append({"phase": entry["phase"], "tag": entry["tag"], "status": entry["measurementStatus"],
                             "reportCount": int(entry["hasCaseReference"]),
                             "reason": clean["measurementIssues"] or [entry["measurementStatus"]]})
    return {"policy": policy, "policyErrors": [], "inputErrors": [], "cases": clean_cases,
            "coverage": {"cases": coverage}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("policy", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--metrics-module", type=Path, default=Path(__file__).with_name("task20-metrics.py"))
    args = parser.parse_args()
    try:
        if args.output.resolve() in {p.resolve() for p in (args.csv, args.policy, args.metrics_module)}:
            raise ValueError("Output must not replace an input or the metrics module")
        rows = load_rows(args.csv)
        summary = synthetic_summary(load_policy(args.policy), rows)
        spec = importlib.util.spec_from_file_location("task20_metrics_table_formatter", args.metrics_module)
        if spec is None or spec.loader is None:
            raise ValueError("Metrics formatter module cannot be loaded")
        metrics = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(metrics)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        metrics.write_tables(args.output, summary, rows)
        safe_text(args.output.read_text(encoding="utf-8"), "Rendered table document")
        print(json.dumps({"rows": len(rows), "cases": len(summary["cases"]),
                          "coverageMarkers": len(summary["coverage"]["cases"]), "output": args.output.name}))
    except (ValueError, OSError, UnicodeError, csv.Error, json.JSONDecodeError) as error:
        message = str(error) if isinstance(error, ValueError) else type(error).__name__
        parser.exit(1, "Detailed table generation rejected: " + message + "\n")


if __name__ == "__main__":
    main()
