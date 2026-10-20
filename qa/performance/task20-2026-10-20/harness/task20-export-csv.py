#!/usr/bin/env python3
"""Export Task20's aggregate CSV with reviewed path/error normalization only."""
import argparse
import csv
import hashlib
import io
import json
import re
from collections import Counter
from pathlib import Path

# Optional newer metrics are accepted but never required or reformatted.
KNOWN_COLUMNS = set("""
phase tag output_kind output_id measurement_status cadence_status native_status
envelope_status envelope_basis partial_evidence case_cadence_status case_native_status
workload_ok instrumentation_valid background_build_sampling_available background_build_overlap
requires_stop_before_further_cases source layers fx blend cpu_requested groups composition_path
target_fps elapsed_seconds delivered_frames_delta delivered_fps target_ratio missed_intervals_delta
missed_fraction cadence_p99_target_intervals cadence_p50_ms cadence_p95_ms cadence_p99_ms
cadence_sample_count cadence_interval_count cadence_window_span_seconds_derived
cadence_rolling_reported_fps cadence_boundary prepare_p50_ms prepare_p95_ms prepare_p99_ms
prepare_sample_count prepare_boundary native_validation_state native_validation_frame_count
native_validation_average_ms native_validation_max_ms native_validation_deadline_miss_fraction
native_frame_budget_pass native_warmup_remaining native_deadline_miss_count native_width native_height
native_work_observation_mean_ms_approx native_work_status native_work_partial_reason
native_work_observation_frame_delta native_lifetime_deadline_misses_delta decode_status
decode_demand_wall_us_delta decode_demand_call_count_delta decode_demand_mean_wall_ms
decoder_session_opens_delta decoder_working_set_evictions_delta decoder_session_resets_delta
decoder_session_resets_reason decoder_failures_delta decoder_lookahead_hits_delta
decoder_last_hardware_sessions decoder_last_sessions cpu_percent_of_32_logical_mean
cpu_percent_of_32_logical_peak_interval cpu_interval_coverage process_membership_changed
cpu_accounting cpu_status memory_status gpu_provider_status working_set_bytes_mean
working_set_bytes_peak private_bytes_mean private_bytes_peak gpu_3d_percent_mean
gpu_decode_percent_mean gpu_video_codec_percent_mean gpu_video_codec_percent_peak
gpu_video_codec_count_mean gpu_video_codec_coverage gpu_video_codec_valid_samples
gpu_video_codec_boundary gpu_copy_percent_mean gpu_provider_available gpu_provider_coverage
gpu_valid_samples gpu_3d_coverage gpu_decode_coverage gpu_copy_coverage gpu_3d_count_mean
gpu_decode_count_mean gpu_copy_count_mean usage_samples usage_span_seconds presentmon_status
presentmon_process_id presentmon_identity_status presentmon_matching_rows presentmon_minimum_rows
presentmon_swapchain presentmon_interval_count presentmon_fps presentmon_p50_ms presentmon_p95_ms
presentmon_p99_ms presentmon_gpu_busy_ms_mean presentmon_in_present_api_ms_mean
presentmon_until_displayed_ms_mean program_monitor_fps preview_monitor_fps executable_sha256
fixture_sha256_before fixture_sha256_after build case_file measurement_issues cadence_reasons
native_reasons usage_reasons presentmon_reasons error restore_error stop_error
""".split())
MUTABLE = {"case_file", "usage_reasons", "presentmon_reasons", "error", "restore_error", "stop_error"}
REQUIRED = MUTABLE | {"phase", "tag"}
PATH_PATTERN = re.compile(r"\b[a-z]:[\\/]|[\\/]+Users[\\/]+|[\\/]+(?:home|root)[\\/]+|\\\\[a-z0-9_.-]+[\\/]|(?:https?|file)://", re.I)
PRIVATE_PATTERN = re.compile(r"localstorage|original[-_]?storage|raw[-_]?storage|errorui|[\"'](?:loaded|controls|recent)[\"']\s*:", re.I)
MISSING_PATTERN = re.compile(r"([^:\r\n]+): FileNotFoundError: .+", re.S)
OVERLAP_MESSAGE = "AssertionError: concurrent compiler or test workload invalidates performance comparison"


def reject_unsafe(value, where):
    if PATH_PATTERN.search(value) or PRIVATE_PATTERN.search(value):
        raise ValueError(f"{where}: unreviewed path or private-state marker")
    if "Traceback (most recent call last)" in value or value.startswith("Error: Command failed:"):
        raise ValueError(f"{where}: unreviewed command/traceback")


def sanitize_reasons(value, expected_file, where):
    if value == "":
        return value
    reasons = json.loads(value)
    if not isinstance(reasons, list) or any(not isinstance(item, str) for item in reasons):
        raise ValueError(f"{where}: expected JSON string list")
    cleaned = []
    for reason in reasons:
        match = MISSING_PATTERN.fullmatch(reason)
        if match:
            if match[1] != expected_file:
                raise ValueError(f"{where}: missing-file reason disagrees with case identity")
            reason = f"{expected_file}: file_missing"
        reject_unsafe(reason, where)
        cleaned.append(reason)
    return value if reasons == cleaned else json.dumps(cleaned, ensure_ascii=False, separators=(",", ":"))


def sanitize_error(value, where):
    if value == "Error: Native backend deadline":
        return "native_backend_startup_deadline"
    if (value.startswith("Error: Command failed: python ")
            and "verify-measurement.py" in value and "hostBuildProcesses" in value
            and "Traceback (most recent call last):" in value
            and value.rstrip().endswith(OVERLAP_MESSAGE)):
        return "sampled_host_compiler_overlap"
    reject_unsafe(value, where)
    return value


def read_csv(text):
    reader = csv.DictReader(io.StringIO(text, newline=""))
    columns = reader.fieldnames or []
    if len(columns) != len(set(columns)) or not REQUIRED <= set(columns):
        raise ValueError("Duplicate or missing required columns")
    if set(columns) - KNOWN_COLUMNS:
        raise ValueError("Unknown columns require explicit review: " + ", ".join(sorted(set(columns) - KNOWN_COLUMNS)))
    rows = list(reader)
    if any(None in row or any(value is None for value in row.values()) for row in rows):
        raise ValueError("Malformed CSV row width")
    return columns, rows


def export(source, destination):
    if source.resolve() == destination.resolve():
        raise ValueError("Input and output must be distinct; retain the original evidence")
    original_bytes = source.read_bytes()
    columns, original = read_csv(original_bytes.decode("utf-8-sig"))
    exported, changes = [], Counter()
    for index, row in enumerate(original, 2):
        if row["phase"] not in ("before", "after") or not re.fullmatch(r"[a-z0-9][a-z0-9_-]{0,159}", row["tag"]):
            raise ValueError(f"row {index}: invalid phase/tag")
        stem = f'{row["phase"]}-{row["tag"]}'
        clean = row.copy()
        if row["case_file"]:
            parts = [part for part in row["case_file"].replace("\\", "/").split("/") if part]
            expected = ["target", "qa", "task20d", stem + ".json"]
            if parts[-4:] != expected or ".." in parts:
                raise ValueError(f"row {index}: case_file disagrees with expected case identity/directory")
            clean["case_file"] = "/".join(expected)
        for field, suffix in (("usage_reasons", "-usage.json"), ("presentmon_reasons", "-presentmon.csv")):
            clean[field] = sanitize_reasons(row[field], stem + suffix, f"row {index} {field}")
        for field in ("error", "restore_error", "stop_error"):
            clean[field] = sanitize_error(row[field], f"row {index} {field}")
        for field in columns:
            reject_unsafe(clean[field], f"row {index} {field}")
            if clean[field] != row[field]:
                assert field in MUTABLE, "Nontext evidence changed"
                changes[field] += 1
        exported.append(clean)
    buffer = io.StringIO(newline="")
    writer = csv.DictWriter(buffer, fieldnames=columns, lineterminator="\n")
    writer.writeheader()
    writer.writerows(exported)
    result = buffer.getvalue()
    reread_columns, reread = read_csv(result)
    assert reread_columns == columns and reread == exported, "CSV roundtrip changed field strings"
    assert all(before[field] == after[field] for before, after in zip(original, reread)
               for field in columns if field not in MUTABLE), "Numeric/status/hash evidence changed"
    reject_unsafe(result, "serialized output")
    result_bytes = result.encode("utf-8")
    destination.write_bytes(result_bytes)
    print(json.dumps({"rows": len(original), "columns": len(columns), "changedCells": dict(changes),
                      "inputSha256": hashlib.sha256(original_bytes).hexdigest(),
                      "outputSha256": hashlib.sha256(result_bytes).hexdigest(),
                      "output": destination.name}, sort_keys=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_csv", type=Path)
    parser.add_argument("output_csv", type=Path)
    args = parser.parse_args()
    try:
        export(args.input_csv, args.output_csv)
    except (ValueError, OSError, UnicodeError, csv.Error) as error:
        parser.exit(1, f"Export rejected: {error}\n")
