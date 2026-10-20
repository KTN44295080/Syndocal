#!/usr/bin/env python3
"""Summarize Task 20d measurements without converting absent evidence to zero.

Usage: python task20-metrics.py TARGET_QA_TASK20D [--out-dir DIRECTORY]
Only before-*.json / after-*.json objects containing a `spec` object are cases.
The inputs are never changed. This program does not run the application.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import math
import re
import statistics
from collections import Counter, defaultdict
from pathlib import Path


LOGICAL_PROCESSORS = 32
NATIVE_MINIMUM_FRAMES = 600
PRESENTMON_MINIMUM_ROWS = 10
KINDS = ("display", "spout", "ndi")
POLICY_KEYS = (
    "cadenceMinimumTargetRatio",
    "missedFractionMaximum",
    "cadenceP99MaximumTargetIntervals",
    "nativeBudgetRequired",
)


def number(value):
    if isinstance(value, bool):
        return None
    try:
        result = float(value)
    except (TypeError, ValueError, OverflowError):
        return None
    return result if math.isfinite(result) else None


def nonnegative(value):
    value = number(value)
    return value if value is not None and value >= 0 else None


def integer(value):
    value = nonnegative(value)
    return int(value) if value is not None and value.is_integer() else None


def rounded(value, places=6):
    return round(value, places) if value is not None else None


def mean(values):
    values = [value for value in values if value is not None]
    return statistics.fmean(values) if values else None


def quantile(values, probability):
    values = sorted(value for value in values if value is not None)
    if not values:
        return None
    position = (len(values) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def load_json(path):
    try:
        with path.open(encoding="utf-8-sig") as handle:
            return json.load(handle), None
    except (OSError, ValueError) as error:
        return None, f"{path.name}: {type(error).__name__}: {error}"


def records(value):
    if isinstance(value, list):
        return [item for item in value if isinstance(item, dict)]
    return []


def unique(values):
    return list(dict.fromkeys(str(value) for value in values if value is not None))


def normal_kind(value):
    token = re.sub(r"[^a-z]", "", str(value).lower())
    return {"display": "display", "displaywindow": "display",
            "nativewindow": "display", "spout": "spout", "ndi": "ndi"}.get(token)


def expected_kinds(spec):
    output = str(spec.get("output", "")).lower()
    if output in ("display", "display4k"):
        return ["display"]
    if output in ("spout", "ndi"):
        return ["display", output]
    return []


def same_id(left, right):
    return left is not None and right is not None and str(left) == str(right)


def difference(first, last, key):
    left, right = nonnegative(first.get(key)), nonnegative(last.get(key))
    if left is None or right is None or right < left:
        return None
    return right - left


def error_present(value):
    return value not in (None, False, "", [], {})


def validated_sha256_mapping(value):
    if not isinstance(value, dict) or not value:
        return None
    if not all(isinstance(key, str) and isinstance(digest, str)
               and re.fullmatch(r"[0-9a-fA-F]{64}", digest)
               for key, digest in value.items()):
        return None
    return dict(value)


def field_samples(samples, kind, output_id):
    result = []
    for sample in samples:
        for health in records(sample.get("health")):
            if normal_kind(health.get("kind")) == kind and same_id(health.get("outputId"), output_id):
                result.append((sample, health))
    return result


def bounded_health(observations, prefix, count_key, cadence=False):
    keys = [f"{prefix}P{percentile}Ms" for percentile in (50, 95, 99)]
    for sample, health in reversed(observations):
        count = integer(health.get(count_key))
        values = [nonnegative(health.get(key)) for key in keys]
        if count is not None and count >= 1 and all(value is not None for value in values):
            rolling_fps = nonnegative(health.get("deliveredFps")) if cadence else None
            return {
                "p50Ms": values[0], "p95Ms": values[1], "p99Ms": values[2],
                "sampleCount": count,
                # The backend serializes intervals.len(), not timestamp count.
                "intervalCount": count if cadence else None,
                "rollingReportedFps": rolling_fps,
                "derivedWindowSpanSeconds": rounded(count / rolling_fps) if rolling_fps is not None and rolling_fps > 0 else None,
                "observedAt": sample.get("at"),
                "monotonicMs": nonnegative(sample.get("monotonicMs")),
                "measurementBoundary": health.get("measurementBoundary" if cadence else "renderMeasurementBoundary"),
            }
    return {"p50Ms": None, "p95Ms": None, "p99Ms": None, "sampleCount": None,
            "intervalCount": None, "rollingReportedFps": None, "derivedWindowSpanSeconds": None,
            "observedAt": None, "monotonicMs": None,
            "measurementBoundary": None}


def cadence_metrics(observations, policy, policy_errors):
    reasons = []
    timed = [(nonnegative(sample.get("monotonicMs")), health) for sample, health in observations]
    if any(timestamp is None for timestamp, _ in timed):
        reasons.append("health_sample_missing_monotonic_time")
    timed = [(timestamp, health) for timestamp, health in timed if timestamp is not None]
    result = {
        "status": "not_established", "reasons": reasons,
        "healthObservations": len(observations), "timedObservations": len(timed),
        "elapsedSeconds": None, "deliveredFramesDelta": None, "missedIntervalsDelta": None,
        "deliveredFps": None, "targetFps": None, "targetRatio": None,
        "missedFraction": None, "p99TargetIntervals": None,
        "boundedCadence": bounded_health(observations, "frameTime", "sampleCount", cadence=True),
        "boundedPrepare": bounded_health(observations, "renderTime", "renderSampleCount"),
    }
    if len(timed) < 2:
        reasons.append("fewer_than_two_timed_output_observations")
        return result
    if any(timed[index][0] <= timed[index - 1][0] for index in range(1, len(timed))):
        reasons.append("non_increasing_monotonic_time")
        return result
    elapsed = (timed[-1][0] - timed[0][0]) / 1000
    result["elapsedSeconds"] = rounded(elapsed)
    if elapsed <= 0:
        reasons.append("non_positive_output_interval")
        return result
    targets = [number(health.get("targetFps")) for _, health in timed]
    if any(value is None or value <= 0 for value in targets):
        reasons.append("missing_or_invalid_target_fps")
    elif max(targets) - min(targets) > 0.000001:
        reasons.append("target_fps_changed_during_case")
    else:
        result["targetFps"] = targets[0]
    for field, output_key in (("deliveredFrames", "deliveredFramesDelta"), ("droppedFrames", "missedIntervalsDelta")):
        values = [nonnegative(health.get(field)) for _, health in timed]
        if any(value is None for value in values):
            reasons.append(f"missing_{field}_counter")
        elif any(right < left for left, right in zip(values, values[1:])):
            reasons.append(f"reset_{field}_counter")
        else:
            result[output_key] = rounded(values[-1] - values[0])
    delivered, missed, target = result["deliveredFramesDelta"], result["missedIntervalsDelta"], result["targetFps"]
    if delivered is not None:
        result["deliveredFps"] = rounded(delivered / elapsed)
        if target is not None:
            result["targetRatio"] = rounded(delivered / elapsed / target)
    if missed is not None and delivered is not None and delivered + missed > 0:
        result["missedFraction"] = rounded(missed / (delivered + missed))
    elif missed == 0 and delivered == 0:
        reasons.append("no_output_or_missed_intervals_recorded")
    p99 = result["boundedCadence"]["p99Ms"]
    if p99 is None:
        reasons.append("missing_bounded_cadence_percentiles")
    elif target is not None:
        result["p99TargetIntervals"] = rounded(p99 * target / 1000)
    if policy_errors:
        reasons.append("invalid_or_missing_predeclared_policy")
    checks = (result["targetRatio"], result["missedFraction"], result["p99TargetIntervals"])
    if reasons or any(value is None for value in checks):
        return result
    failures = []
    # Compare the original counter ratio; avoid rounding a marginal miss into a pass.
    if delivered / elapsed / target < policy["cadenceMinimumTargetRatio"]:
        failures.append("delivered_rate_below_policy")
    if missed / (delivered + missed) > policy["missedFractionMaximum"]:
        failures.append("missed_interval_fraction_above_policy")
    if p99 * target / 1000 > policy["cadenceP99MaximumTargetIntervals"]:
        failures.append("cadence_p99_above_policy")
    result["status"] = "fail" if failures else "pass"
    result["reasons"].extend(failures)
    return result


def native_metrics(samples, output_id, minimum_frames):
    observations = []
    for sample in samples:
        for window in records(sample.get("windows")):
            performance = window.get("performance")
            if same_id(window.get("output_id"), output_id) and isinstance(performance, dict):
                observations.append((sample, performance))
    result = {
        "status": "not_established", "reasons": [], "observations": len(observations),
        "validationState": None, "validationFrameCount": None, "minimumValidationFrames": minimum_frames,
        "frameBudgetPass": None, "frameBudgetFieldPresent": False, "warmupRemaining": None, "deadlineMissCount": None,
        "averageFrameUs": None, "maxFrameUs": None, "width": None, "height": None,
        "layerSlots": None, "compressedLayerUploads": None, "lastError": None,
        "observedWorkMeanMsApprox": None, "observedWorkFramesDelta": None,
        "workStatus": "not_established", "workPartialReason": None,
        "lifetimeDeadlineMissesDelta": None, "workDeltaReasons": [],
        "decode": {"wallUsDelta": None, "callCountDelta": None, "meanDemandWallMs": None,
                   "sessionOpensDelta": None, "workingSetEvictionsDelta": None,
                   "sessionResetsDelta": None, "sessionResetsReason": None,
                   "failuresDelta": None, "lookaheadHitsDelta": None,
                   "lastHardwareSessionCount": None, "lastSessionCount": None,
                   "status": "not_established", "reasons": []},
    }
    if not observations:
        result["reasons"].append("no_matching_native_display_metrics")
        result["decode"]["sessionResetsReason"] = "no_matching_native_display_metrics"
        return result
    first, last = observations[0][1], observations[-1][1]
    mapping = {"validationState": "validation_state", "validationFrameCount": "frame_count",
               "frameBudgetPass": "frame_budget_pass", "warmupRemaining": "warmup_remaining",
               "deadlineMissCount": "deadline_miss_count", "averageFrameUs": "average_frame_us",
               "maxFrameUs": "max_frame_us", "width": "width", "height": "height",
               "layerSlots": "layer_slots", "compressedLayerUploads": "compressed_layer_uploads",
               "lastError": "last_error"}
    for destination, source in mapping.items():
        result[destination] = last.get(source)
    result["frameBudgetFieldPresent"] = "frame_budget_pass" in last
    state = str(last.get("validation_state", "")).lower()
    frames = integer(last.get("frame_count"))
    budget = last.get("frame_budget_pass")
    if state == "failed":
        result["status"] = "fail"
        result["reasons"].append("native_validation_failed")
    elif state == "passed" and frames is not None and frames >= minimum_frames and budget is True:
        result["status"] = "pass"
    elif frames is not None and frames >= minimum_frames and budget is False:
        result["status"] = "fail"
        result["reasons"].append("native_frame_budget_failed")
    else:
        result["reasons"].append("native_validation_not_complete")
    if error_present(last.get("last_error")):
        result["status"] = "fail"
        result["reasons"].append("native_last_error")
    if len(observations) < 2:
        result["workDeltaReasons"].append("fewer_than_two_native_observations")
        result["decode"]["reasons"].append("fewer_than_two_native_observations")
        result["decode"]["sessionResetsReason"] = "fewer_than_two_native_observations"
        return result
    counts = [nonnegative(performance.get("lifetime_frame_count")) for _, performance in observations]
    averages = [nonnegative(performance.get("lifetime_average_frame_us")) for _, performance in observations]
    if any(value is None for value in counts + averages):
        result["workDeltaReasons"].append("missing_lifetime_frame_instrumentation")
    elif any(right < left for left, right in zip(counts, counts[1:])):
        result["workDeltaReasons"].append("lifetime_frame_counter_reset")
    else:
        frame_delta = counts[-1] - counts[0]
        result["observedWorkFramesDelta"] = rounded(frame_delta)
        if frame_delta > 0:
            work_delta_us = averages[-1] * counts[-1] - averages[0] * counts[0]
            if work_delta_us >= 0:
                result["observedWorkMeanMsApprox"] = rounded(work_delta_us / frame_delta / 1000)
            else:
                result["workDeltaReasons"].append("negative_rounded_work_delta")
        else:
            result["workDeltaReasons"].append("no_native_frames_during_observation")
    if result["observedWorkMeanMsApprox"] is not None:
        result["workStatus"] = "measured"
    elif (result["workDeltaReasons"] == ["no_native_frames_during_observation"]
          and all(value == 0 for value in counts + averages)):
        warmup_counts = [integer(performance.get("warmup_remaining")) for _, performance in observations]
        pending_uninitialized = all(
            str(performance.get("validation_state", "")).lower() == "pending"
            and integer(performance.get("frame_count")) == 0
            and "frame_budget_pass" in performance and performance["frame_budget_pass"] is None
            and remaining is not None and remaining > 0
            for (_, performance), remaining in zip(observations, warmup_counts))
        if pending_uninitialized and all(right <= left for left, right in zip(warmup_counts, warmup_counts[1:])):
            result["workStatus"] = "pending_warmup"
            result["workPartialReason"] = "native lifetime work counters have not started during unfinished warmup; full-work mean is unavailable"
    result["lifetimeDeadlineMissesDelta"] = difference(first, last, "lifetime_deadline_miss_count")
    diagnostics = [performance.get("decoder_diagnostics") for _, performance in observations]
    decode = result["decode"]
    if any(not isinstance(item, dict) for item in diagnostics):
        decode["reasons"].append("missing_decoder_diagnostics")
        decode["sessionResetsReason"] = "missing_decoder_diagnostics"
        return result
    decode_fields = {
        "wallUsDelta": "decode_wall_us", "callCountDelta": "decode_call_count",
        "sessionOpensDelta": "libav_session_open_count",
        "workingSetEvictionsDelta": "libav_working_set_eviction_count",
        "failuresDelta": "decode_failures", "lookaheadHitsDelta": "libav_lookahead_hit_count",
    }
    for destination, source in decode_fields.items():
        values = [nonnegative(item.get(source)) for item in diagnostics]
        if any(value is None for value in values):
            decode["reasons"].append(f"missing_{source}")
        elif any(right < left for left, right in zip(values, values[1:])):
            decode["reasons"].append(f"reset_{source}")
        else:
            decode[destination] = rounded(values[-1] - values[0])
    # Optional diagnostic: in-place demux rewind/decoder flush. Keep its
    # availability separate from required decode evidence and from session
    # opens/working-set evictions; resets alone do not establish cache churn.
    session_resets = [integer(item.get("libav_session_reset_count")) for item in diagnostics]
    if any(value is None for value in session_resets):
        decode["sessionResetsReason"] = "missing_or_invalid_libav_session_reset_count"
    elif any(right < left for left, right in zip(session_resets, session_resets[1:])):
        decode["sessionResetsReason"] = "nonmonotonic_libav_session_reset_count"
    else:
        decode["sessionResetsDelta"] = session_resets[-1] - session_resets[0]
    decode["lastHardwareSessionCount"] = nonnegative(diagnostics[-1].get("libav_hardware_session_count"))
    decode["lastSessionCount"] = nonnegative(diagnostics[-1].get("libav_session_count"))
    if decode["wallUsDelta"] is not None and decode["callCountDelta"] is not None:
        if decode["callCountDelta"] > 0:
            decode["meanDemandWallMs"] = rounded(decode["wallUsDelta"] / decode["callCountDelta"] / 1000)
            decode["status"] = "measured"
        elif decode["wallUsDelta"] == 0:
            decode["status"] = "no_synchronous_decode_calls"
        else:
            decode["reasons"].append("decode_wall_without_calls")
    if decode["reasons"]:
        decode["status"] = "not_established"
    return result


def usage_metrics(path, instrumentation):
    value, read_error = load_json(path)
    samples = records(value if isinstance(value, list) else (value or {}).get("samples")) if isinstance(value, (list, dict)) else []
    result = {
        "file": str(path), "status": "not_established", "reasons": [], "sampleCount": len(samples),
        "cpuStatus": "not_established", "memoryStatus": "not_established", "gpuProviderStatus": "not_established",
        "spanSeconds": None, "cpuPercentOf32LogicalMean": None, "cpuPercentOf32LogicalPeakInterval": None,
        "cpuValidIntervals": 0, "cpuIntervalCoverage": None, "processMembershipChanged": False,
        "cpuAccounting": "sum of retained cumulative CPU readings keyed by PID plus process start time; exited processes remain accounted for",
        "workingSetBytesMean": None, "workingSetBytesPeak": None,
        "privateBytesMean": None, "privateBytesPeak": None,
        "gpuProviderAvailable": instrumentation.get("gpuProviderAvailable"),
        "gpuValidSamples": 0, "gpuProviderCoverage": None,
        "gpu3dPercentMean": None, "gpuDecodePercentMean": None, "gpuCopyPercentMean": None,
        "gpu3dPercentPeak": None, "gpuDecodePercentPeak": None, "gpuCopyPercentPeak": None,
        "gpu3dCountMean": None, "gpuDecodeCountMean": None, "gpuCopyCountMean": None,
        "gpu3dValidSamples": 0, "gpuDecodeValidSamples": 0, "gpuCopyValidSamples": 0,
        "gpu3dCoverage": None, "gpuDecodeCoverage": None, "gpuCopyCoverage": None,
        "gpuVideoCodecPercentMean": None, "gpuVideoCodecPercentPeak": None,
        "gpuVideoCodecCountMean": None, "gpuVideoCodecValidSamples": 0, "gpuVideoCodecCoverage": None,
        "gpuVideoCodecBoundary": "per-sample sum of exact reported engtype_Video Codec Engine instances; can exceed 100%; not device-wide utilization or a renamed decode counter",
        "gpuEngineCountMean": None, "gpuErrors": [],
    }
    if read_error:
        result["reasons"].append(read_error)
    if len(samples) < 2:
        result["reasons"].append("fewer_than_two_usage_samples")
        return result
    times = [nonnegative(sample.get("atSeconds")) for sample in samples]
    if any(value is None for value in times) or any(right <= left for left, right in zip(times, times[1:])):
        result["reasons"].append("missing_or_non_increasing_usage_time")
    else:
        result["spanSeconds"] = rounded(times[-1] - times[0])
    cpu_intervals = []
    for index, (left, right) in enumerate(zip(samples, samples[1:])):
        left_time, right_time = times[index:index + 2]
        delta = difference(left, right, "totalCpuSeconds")
        if left_time is not None and right_time is not None and right_time > left_time and delta is not None:
            cpu_intervals.append((delta, right_time - left_time))
    if cpu_intervals:
        result["cpuValidIntervals"] = len(cpu_intervals)
        result["cpuIntervalCoverage"] = rounded(len(cpu_intervals) / (len(samples) - 1))
        result["cpuPercentOf32LogicalMean"] = rounded(sum(item[0] for item in cpu_intervals) / sum(item[1] for item in cpu_intervals) / LOGICAL_PROCESSORS * 100)
        result["cpuPercentOf32LogicalPeakInterval"] = rounded(max(cpu / elapsed / LOGICAL_PROCESSORS * 100 for cpu, elapsed in cpu_intervals))
    if len(cpu_intervals) != len(samples) - 1:
        result["reasons"].append("incomplete_or_reset_process_tree_cpu_counters")
    elif result["spanSeconds"] is not None and result["spanSeconds"] > 0:
        result["cpuStatus"] = "measured"
    memberships = [tuple(sorted(str(pid) for pid in sample["processIds"]))
                   for sample in samples if isinstance(sample.get("processIds"), list)]
    result["processMembershipChanged"] = len(set(memberships)) > 1
    for source in ("workingSetBytes", "privateBytes"):
        values = [nonnegative(sample.get(source)) for sample in samples]
        result[source + "Mean"] = rounded(mean(values))
        result[source + "Peak"] = max((value for value in values if value is not None), default=None)
        if any(value is None for value in values):
            result["reasons"].append(f"incomplete_{source}_samples")
    if not any(reason.startswith("incomplete_workingSetBytes") or reason.startswith("incomplete_privateBytes") for reason in result["reasons"]):
        result["memoryStatus"] = "measured"
    valid_gpu = [sample for sample in samples if not error_present(sample.get("gpuError"))
                 and nonnegative(sample.get("gpuEngineCount")) is not None
                 and nonnegative(sample.get("gpuEngineCount")) > 0]
    result["gpuValidSamples"] = len(valid_gpu)
    result["gpuProviderCoverage"] = rounded(len(valid_gpu) / len(samples))
    result["gpuErrors"] = unique(sample.get("gpuError") for sample in samples if error_present(sample.get("gpuError")))
    result["gpuEngineCountMean"] = rounded(mean([nonnegative(sample.get("gpuEngineCount")) for sample in valid_gpu]))
    for category in ("gpu3d", "gpuDecode", "gpuCopy"):
        counts = [nonnegative(sample.get(category + "Count")) for sample in valid_gpu]
        result[category + "CountMean"] = rounded(mean(counts))
        # A successful provider sample can have no matching instance of a
        # particular engine type. Such a category is unobserved, not 0% busy.
        category_samples = [sample for sample in valid_gpu
                            if nonnegative(sample.get(category + "Count")) is not None
                            and nonnegative(sample.get(category + "Count")) > 0
                            and nonnegative(sample.get(category + "Percent")) is not None]
        values = [nonnegative(sample.get(category + "Percent")) for sample in category_samples]
        result[category + "ValidSamples"] = len(category_samples)
        result[category + "Coverage"] = rounded(len(category_samples) / len(samples))
        result[category + "PercentMean"] = rounded(mean(values))
        result[category + "PercentPeak"] = max(values, default=None)
    codec_counts, codec_values = [], []
    for sample in valid_gpu:
        engines = sample.get("gpuEngines")
        if not isinstance(engines, list):
            continue  # Missing raw instance evidence is not a zero-instance sample.
        matching = [engine for engine in records(engines)
                    if str(engine.get("Name", "")).rsplit("_engtype_", 1)[-1] == "Video Codec Engine"
                    and "_engtype_" in str(engine.get("Name", ""))]
        codec_counts.append(len(matching))
        percentages = [nonnegative(engine.get("UtilizationPercentage")) for engine in matching]
        if percentages and all(value is not None for value in percentages):
            codec_values.append(sum(percentages))
    result["gpuVideoCodecCountMean"] = rounded(mean(codec_counts))
    result["gpuVideoCodecValidSamples"] = len(codec_values)
    result["gpuVideoCodecCoverage"] = rounded(len(codec_values) / len(samples))
    result["gpuVideoCodecPercentMean"] = rounded(mean(codec_values))
    result["gpuVideoCodecPercentPeak"] = max(codec_values, default=None)
    if instrumentation.get("gpuProviderAvailable") is not True:
        result["reasons"].append("gpu_provider_not_established")
    if len(valid_gpu) != len(samples):
        result["reasons"].append("incomplete_gpu_provider_coverage")
    elif instrumentation.get("gpuProviderAvailable") is True:
        result["gpuProviderStatus"] = "measured"
    if not result["reasons"]:
        result["status"] = "measured"
    return result


def presentmon_metrics(path, report):
    instrumentation = report.get("instrumentation") if isinstance(report.get("instrumentation"), dict) else {}
    launch = report.get("launch") if isinstance(report.get("launch"), dict) else {}
    pid = integer(launch.get("pid"))
    declared_pid = integer(instrumentation.get("presentProcessId"))
    result = {"file": str(path), "status": "not_established", "reasons": [],
              "processId": pid, "declaredProcessId": declared_pid,
              "measurementBoundary": instrumentation.get("presentMonBoundary"),
              "minimumRows": PRESENTMON_MINIMUM_ROWS, "identityStatus": "unobserved",
              "allRows": 0, "matchingRows": 0, "streams": [], "observed": None, "selected": None,
              "fileMissing": False, "readError": None}
    if pid is None:
        result["reasons"].append("missing_exact_native_launch_pid")
    if declared_pid is not None and pid is not None and declared_pid != pid:
        result["reasons"].append("presentmon_instrumentation_pid_mismatch")
    try:
        with path.open(newline="", encoding="utf-8-sig") as handle:
            rows = list(csv.DictReader(handle))
    except FileNotFoundError as error:
        result["fileMissing"] = True
        result["readError"] = f"{path.name}: {type(error).__name__}: {error}"
        result["allRows"] = result["matchingRows"] = None
        result["reasons"].extend(["presentmon_file_missing", "presentmon_insufficient_samples"])
        return result
    except (OSError, ValueError, csv.Error) as error:
        result["reasons"].append(f"{path.name}: {type(error).__name__}: {error}")
        return result
    result["allRows"] = len(rows)
    groups = defaultdict(list)
    for row in rows:
        row_pid = integer(row.get("ProcessID"))
        application = str(row.get("Application", "")).replace("\\", "/").rsplit("/", 1)[-1].lower()
        swapchain = str(row.get("SwapChainAddress", "")).strip()
        if pid is not None and row_pid == pid and application == "syndocal.exe":
            result["matchingRows"] += 1
            groups[swapchain].append(row)
    if result["allRows"] != result["matchingRows"]:
        result["reasons"].append("unexpected_presentmon_pid_or_application_rows")
    for swapchain, stream_rows in sorted(groups.items()):
        intervals = [number(row.get("MsBetweenPresents")) for row in stream_rows]
        intervals = [value for value in intervals if value is not None and value > 0]
        stream = {"swapChainAddress": swapchain or None, "rowCount": len(stream_rows),
                  "finiteNonnegativeTimingRows": sum(nonnegative(row.get("MsBetweenPresents")) is not None for row in stream_rows),
                  "intervalCount": len(intervals), "spanSeconds": rounded(sum(intervals) / 1000) if intervals else None,
                  "fps": rounded(1000 / statistics.fmean(intervals)) if intervals else None,
                  "p50Ms": rounded(quantile(intervals, 0.5)), "p95Ms": rounded(quantile(intervals, 0.95)),
                  "p99Ms": rounded(quantile(intervals, 0.99)),
                  "presentModes": unique(row.get("PresentMode") for row in stream_rows)}
        for source, output in (("MsGPUBusy", "gpuBusyMsMean"), ("MsInPresentAPI", "inPresentApiMsMean"), ("MsUntilDisplayed", "untilDisplayedMsMean")):
            stream[output] = rounded(mean([nonnegative(row.get(source)) for row in stream_rows]))
        result["streams"].append(stream)
    if result["allRows"] == 0 and not result["reasons"]:
        # No rows provides no independent PresentMon identity evidence. It does
        # not contradict the app's separately verified output-counter evidence.
        result["status"] = "insufficient_samples"
        result["reasons"].append("presentmon_insufficient_samples")
        return result
    if len(result["streams"]) != 1:
        result["reasons"].append("presentmon_requires_exactly_one_native_swapchain")
    else:
        selected = result["streams"][0]
        result["observed"] = selected
        if selected["swapChainAddress"] is None:
            result["reasons"].append("missing_native_swapchain_identity")
        if not result["reasons"]:
            result["identityStatus"] = "verified"
        if selected["finiteNonnegativeTimingRows"] != selected["rowCount"]:
            result["reasons"].append("non_finite_or_negative_presentmon_timing")
        if not result["reasons"]:
            if selected["rowCount"] < PRESENTMON_MINIMUM_ROWS or selected["intervalCount"] < 2:
                result["status"] = "insufficient_samples"
                result["reasons"].append("presentmon_insufficient_samples")
            else:
                result["status"] = "measured"
                result["selected"] = selected
    return result


def monitor_metrics(report):
    value = report.get("monitor")
    if not isinstance(value, dict):
        value = report.get("monitorRates")
    if not isinstance(value, dict):
        value = {}
    return {"elapsedMs": nonnegative(value.get("elapsedMs")),
            "programFps": nonnegative(value.get("programFps")),
            "previewFps": nonnegative(value.get("previewFps"))}


def verdict(statuses):
    if not statuses or "not_established" in statuses:
        return "fail" if "fail" in statuses else "not_established"
    return "fail" if "fail" in statuses else "pass"


def summarize_case(path, report, policy, policy_errors, minimum_frames):
    spec = report["spec"]
    phase = "before" if path.name.startswith("before-") else "after"
    tag = str(spec.get("tag", path.stem[len(phase) + 1:]))
    samples = records(report.get("samples"))
    instrumentation = report.get("instrumentation") if isinstance(report.get("instrumentation"), dict) else {}
    composition = report.get("compositionEvidence") if isinstance(report.get("compositionEvidence"), dict) else {}
    result = {
        "caseFile": str(path), "phase": phase, "tag": tag, "spec": spec,
        "build": report.get("build"), "executableSha256": report.get("executableSha256"),
        "harnessSha256": validated_sha256_mapping(report.get("harnessSha256")),
        "fixtureSha256Before": report.get("fixtureSha256Before"), "fixtureSha256After": report.get("fixtureSha256After"),
        "ok": report.get("ok"), "error": report.get("error"), "restoreError": report.get("restoreError"),
        "stopError": report.get("stopError"),
        "requiresStopBeforeFurtherCases": error_present(report.get("restoreError")) or error_present(report.get("stopError")),
        "instrumentation": instrumentation, "compositionEvidence": composition,
        "monitor": monitor_metrics(report), "sampleCount": len(samples),
        "startedAt": samples[0].get("at") if samples else None,
        "endedAt": samples[-1].get("at") if samples else None,
        "measurementStatus": "incomplete", "measurementIssues": [],
        "cadenceStatus": "not_established", "nativeStatus": "not_established", "envelopeStatus": "not_established",
        "envelopeBasis": None, "partialEvidence": [],
        "outputs": [],
    }
    issues = result["measurementIssues"]
    native_work_partial_issues = []
    if report.get("ok") is not True:
        issues.append("workload_failed_or_not_completed")
    if result["requiresStopBeforeFurtherCases"]:
        issues.append("restore_or_stop_failed")
    if instrumentation.get("valid") is not True:
        issues.append("instrumentation_not_valid")
    if not report.get("executableSha256"):
        issues.append("missing_executable_hash")
    if not report.get("fixtureSha256Before") or not report.get("fixtureSha256After"):
        issues.append("missing_fixture_hash_evidence")
    elif report["fixtureSha256Before"] != report["fixtureSha256After"]:
        issues.append("fixture_changed_during_case")
    if not composition.get("path"):
        issues.append("missing_composition_evidence")
    if result["monitor"]["programFps"] is None or result["monitor"]["previewFps"] is None:
        issues.append("missing_monitor_fps")
    if policy_errors:
        issues.append("invalid_or_missing_predeclared_policy")
    result["usage"] = usage_metrics(path.with_name(path.stem + "-usage.json"), instrumentation)
    result["presentMon"] = presentmon_metrics(path.with_name(path.stem + "-presentmon.csv"), report)
    # Explicit low-sample or absent-capture markers permit partial corroboration.
    # Identity contradictions, corrupt timing, provider failures and missing
    # workload proof remain blocking issues; they are never softened here.
    partial_reasons = instrumentation.get("partialReasons")
    declared_presentmon_partial = partial_reasons == ["presentmon_insufficient_samples"]
    missing_markers = {"presentmon_file_missing", "presentmon_insufficient_samples"}
    declared_presentmon_missing = (isinstance(partial_reasons, list) and len(partial_reasons) == 2
                                  and all(isinstance(reason, str) for reason in partial_reasons)
                                  and set(partial_reasons) == missing_markers)
    partial_presentmon_missing = (instrumentation.get("valid") is False and declared_presentmon_missing
                                  and result["presentMon"]["fileMissing"] is True
                                  and result["presentMon"]["status"] == "not_established"
                                  and result["presentMon"]["identityStatus"] == "unobserved"
                                  and set(result["presentMon"]["reasons"]) == missing_markers)
    partial_presentmon = (partial_presentmon_missing
                         or (instrumentation.get("valid") is False and declared_presentmon_partial
                             and result["presentMon"]["status"] == "insufficient_samples"
                             and result["presentMon"]["reasons"] == ["presentmon_insufficient_samples"]))
    if partial_presentmon:
        issues.remove("instrumentation_not_valid")
        issues.append("instrumentation_partial_presentmon_insufficient_samples")
    if result["usage"]["status"] != "measured":
        issues.append("incomplete_process_tree_or_gpu_measurements")
    if result["presentMon"]["status"] == "insufficient_samples":
        issues.append("presentmon_insufficient_samples")
    elif partial_presentmon_missing:
        issues.extend(["presentmon_file_missing", "presentmon_insufficient_samples"])
    elif result["presentMon"]["status"] != "measured":
        issues.append("incomplete_exact_process_single_swapchain_presentmon")
    kinds = expected_kinds(spec)
    if not kinds:
        issues.append("unknown_expected_output_configuration")
        kinds = ["unknown"]
    for kind in kinds:
        output_ids = []
        for sample in samples:
            for health in records(sample.get("health")):
                output_id = health.get("outputId")
                if normal_kind(health.get("kind")) == kind and output_id is not None and str(output_id) not in [str(item) for item in output_ids]:
                    output_ids.append(output_id)
        if not output_ids and kind == "display":
            output_ids = list(dict.fromkeys(window.get("output_id") for sample in samples
                                           for window in records(sample.get("windows")) if window.get("output_id") is not None))
        for output_id in output_ids or [None]:
            observations = field_samples(samples, kind, output_id)
            cadence = cadence_metrics(observations, policy, policy_errors)
            native = native_metrics(samples, output_id, minimum_frames) if kind == "display" else None
            output = {"kind": kind, "outputId": output_id, "cadence": cadence, "native": native}
            result["outputs"].append(output)
            if not observations:
                issues.append(f"missing_expected_{kind}_health")
            elif len(observations) != len(samples):
                issues.append(f"expected_{kind}_{output_id}_not_present_once_in_every_sample")
            if cadence["status"] == "not_established":
                issues.append(f"incomplete_{kind}_{output_id}_cadence")
            if cadence["boundedPrepare"]["sampleCount"] is None:
                issues.append(f"missing_{kind}_{output_id}_prepare_distribution")
            if native:
                # A fully measured low-FPS run can still be sampling the native
                # 600-frame gate. That is a native verdict, not missing evidence.
                native_count = integer(native["validationFrameCount"])
                native_state = str(native["validationState"] or "").lower()
                pending_budget = (native["frameBudgetPass"] is None
                                  and native["frameBudgetFieldPresent"]
                                  and native_state in ("pending", "sampling")
                                  and native_count is not None and native_count < minimum_frames)
                if (native["observations"] == 0 or not native["validationState"]
                        or native_count is None
                        or not (isinstance(native["frameBudgetPass"], bool) or pending_budget)):
                    issues.append(f"incomplete_display_{output_id}_native_validation")
                if native["observedWorkMeanMsApprox"] is None:
                    if (native["workStatus"] == "pending_warmup"
                            and cadence["deliveredFramesDelta"] is not None and cadence["deliveredFramesDelta"] > 0
                            and native["decode"]["status"] in ("measured", "no_synchronous_decode_calls")):
                        partial_issue = f"partial_display_{output_id}_native_work_pending_warmup"
                        issues.append(partial_issue)
                        native_work_partial_issues.append(partial_issue)
                    else:
                        issues.append(f"missing_display_{output_id}_observed_native_work")
                if native["decode"]["status"] == "not_established":
                    issues.append(f"incomplete_display_{output_id}_decode_instrumentation")
    result["cadenceStatus"] = verdict([output["cadence"]["status"] for output in result["outputs"]])
    result["nativeStatus"] = verdict([output["native"]["status"] for output in result["outputs"] if output["native"]])
    partial_issues = set(native_work_partial_issues)
    if partial_presentmon:
        partial_issues.update({"instrumentation_partial_presentmon_insufficient_samples", "presentmon_insufficient_samples"})
    if partial_presentmon_missing:
        partial_issues.add("presentmon_file_missing")
    allowed_partial_evidence = partial_presentmon or bool(native_work_partial_issues)
    result["partialEvidence"] = [issue for issue in issues if issue in partial_issues]
    other_issues = [issue for issue in issues if issue not in partial_issues]
    if report.get("ok") is not True:
        result["measurementStatus"] = "workload_failed"
    elif result["requiresStopBeforeFurtherCases"]:
        result["measurementStatus"] = "restore_or_stop_failed"
    elif (instrumentation.get("valid") is not True and not partial_presentmon) or "fixture_changed_during_case" in issues:
        result["measurementStatus"] = "invalid"
    elif allowed_partial_evidence and not other_issues:
        result["measurementStatus"] = "partial"
    elif not issues:
        result["measurementStatus"] = "complete"
    result["measurementIssues"] = unique(issues)
    # A failed workload is never a successful benchmark. Cadence/native verdicts
    # remain inspectable, but cannot promote invalid evidence to an envelope pass.
    # Insufficient native validation frames alone do not make measurement
    # evidence incomplete. Missing evidence still blocks an envelope verdict.
    blocking_issues = list(issues)
    if report.get("ok") is True and instrumentation.get("valid") is True and not blocking_issues:
        if result["cadenceStatus"] == "fail" or result["nativeStatus"] == "fail":
            result["envelopeStatus"] = "miss"
            result["envelopeBasis"] = "complete_measurement_failure"
        elif result["measurementStatus"] == "complete" and result["cadenceStatus"] == "pass" and (result["nativeStatus"] == "pass" or policy.get("nativeBudgetRequired") is False):
            result["envelopeStatus"] = "pass"
            result["envelopeBasis"] = "complete_measurement_all_required_gates_passed"
    elif (allowed_partial_evidence and (instrumentation.get("valid") is True or partial_presentmon)
          and report.get("ok") is True and not other_issues
          and (result["cadenceStatus"] == "fail" or result["nativeStatus"] == "fail")):
        # A declared corroborating PresentMon shortage/absent file or pending
        # native warmup counter cannot erase a verified output miss. Neither
        # form of partial evidence can ever establish an envelope pass.
        result["envelopeStatus"] = "miss"
        result["envelopeBasis"] = "verified_output_failure_with_partial_evidence"
    return result


def case_specs(value, lookup=None):
    lookup = lookup or {}
    if isinstance(value, dict):
        value = value.get("cases", value.get("tags", []))
    result = []
    if isinstance(value, list):
        for item in value:
            if isinstance(item, str):
                result.append(dict(lookup.get(item, {"tag": item})))
            elif isinstance(item, dict):
                spec = item.get("spec", item)
                if isinstance(spec, dict) and spec.get("tag") is not None:
                    result.append(spec)
    return result


def coverage_metrics(root, cases, all_specs):
    lookup = {str(spec["tag"]): spec for spec in all_specs}
    before_value, before_error = load_json(root / "before-cases.json")
    before_specs = case_specs(before_value, lookup)
    build, build_error = load_json(root / "after-build.json")
    build = build if isinstance(build, dict) else {}
    ndi_unsupported = build.get("ndiEnabled") is False
    plans = {"before": before_specs, "after": all_specs}
    found = defaultdict(list)
    for case in cases:
        found[(case["phase"], case["tag"])].append(case)
    rows = []
    for phase, specs in plans.items():
        for spec in specs:
            key = (phase, str(spec["tag"]))
            reports = found.get(key, [])
            unsupported = phase == "after" and ndi_unsupported and str(spec.get("output", "")).lower() == "ndi"
            if unsupported:
                status = "unsupported"
            elif not reports:
                status = "missing_case"
            elif any(case["measurementStatus"] == "complete" for case in reports):
                status = "measured"
            else:
                status = "reported_incomplete"
            rows.append({"phase": phase, "tag": str(spec["tag"]), "spec": spec, "status": status,
                         "reason": "after-build.json declares ndiEnabled=false" if unsupported else None,
                         "reportCount": len(reports), "reportFiles": [case["caseFile"] for case in reports]})
    return {"beforePlanFile": str(root / "before-cases.json"), "beforePlanError": before_error,
            "afterBuildFile": str(root / "after-build.json"), "afterBuildError": build_error,
            "ndiExplicitlyUnsupported": ndi_unsupported, "cases": rows,
            "missingSupportedCases": [row for row in rows if row["status"] == "missing_case"],
            "unsupportedCases": [row for row in rows if row["status"] == "unsupported"],
            "duplicateReports": [{"phase": key[0], "tag": key[1], "files": [case["caseFile"] for case in reports]}
                                 for key, reports in found.items() if len(reports) > 1]}


def flattened_row(case, output):
    spec, cadence = case["spec"], output["cadence"]
    bounded, prepare = cadence["boundedCadence"], cadence["boundedPrepare"]
    native = output["native"] or {}
    decode, usage = native.get("decode", {}), case["usage"]
    pm = case["presentMon"]["selected"] or {}
    build_sampling_available = case["instrumentation"].get("backgroundBuildSamplingAvailable")
    build_overlap = (case["instrumentation"].get("backgroundBuildOverlap")
                     if build_sampling_available is True else None)
    validation_count = integer(native.get("validationFrameCount"))
    validation_average_us = nonnegative(native.get("averageFrameUs"))
    validation_max_us = nonnegative(native.get("maxFrameUs"))
    validation_misses = nonnegative(native.get("deadlineMissCount"))
    has_validation_frames = validation_count is not None and validation_count > 0
    validation_average_ms = (rounded(validation_average_us / 1000)
                             if has_validation_frames and validation_average_us is not None else None)
    validation_max_ms = (rounded(validation_max_us / 1000)
                         if has_validation_frames and validation_max_us is not None else None)
    validation_miss_fraction = (rounded(validation_misses / validation_count)
                                if has_validation_frames and validation_misses is not None else None)
    return {
        "phase": case["phase"], "tag": case["tag"], "output_kind": output["kind"], "output_id": output["outputId"],
        "measurement_status": case["measurementStatus"], "cadence_status": cadence["status"],
        "native_status": native.get("status", "not_applicable"), "envelope_status": case["envelopeStatus"],
        "envelope_basis": case["envelopeBasis"],
        "partial_evidence": case["partialEvidence"],
        "case_cadence_status": case["cadenceStatus"], "case_native_status": case["nativeStatus"],
        "workload_ok": case["ok"], "instrumentation_valid": case["instrumentation"].get("valid"),
        "background_build_sampling_available": build_sampling_available, "background_build_overlap": build_overlap,
        "requires_stop_before_further_cases": case["requiresStopBeforeFurtherCases"],
        "source": spec.get("source"), "layers": spec.get("layers"), "fx": spec.get("fx"),
        "blend": spec.get("blend"), "cpu_requested": spec.get("cpu"), "groups": spec.get("groups"),
        "composition_path": case["compositionEvidence"].get("path"),
        "target_fps": cadence["targetFps"], "elapsed_seconds": cadence["elapsedSeconds"],
        "delivered_frames_delta": cadence["deliveredFramesDelta"], "delivered_fps": cadence["deliveredFps"],
        "target_ratio": cadence["targetRatio"], "missed_intervals_delta": cadence["missedIntervalsDelta"],
        "missed_fraction": cadence["missedFraction"], "cadence_p99_target_intervals": cadence["p99TargetIntervals"],
        "cadence_p50_ms": bounded["p50Ms"], "cadence_p95_ms": bounded["p95Ms"], "cadence_p99_ms": bounded["p99Ms"],
        "cadence_sample_count": bounded["sampleCount"], "cadence_interval_count": bounded["intervalCount"],
        "cadence_window_span_seconds_derived": bounded["derivedWindowSpanSeconds"],
        "cadence_rolling_reported_fps": bounded["rollingReportedFps"],
        "cadence_boundary": bounded["measurementBoundary"],
        "prepare_p50_ms": prepare["p50Ms"], "prepare_p95_ms": prepare["p95Ms"], "prepare_p99_ms": prepare["p99Ms"],
        "prepare_sample_count": prepare["sampleCount"], "prepare_boundary": prepare["measurementBoundary"],
        "native_validation_state": native.get("validationState"), "native_validation_frame_count": native.get("validationFrameCount"),
        "native_validation_average_ms": validation_average_ms, "native_validation_max_ms": validation_max_ms,
        "native_validation_deadline_miss_fraction": validation_miss_fraction,
        "native_frame_budget_pass": native.get("frameBudgetPass"), "native_warmup_remaining": native.get("warmupRemaining"),
        "native_deadline_miss_count": native.get("deadlineMissCount"), "native_width": native.get("width"), "native_height": native.get("height"),
        "native_work_observation_mean_ms_approx": native.get("observedWorkMeanMsApprox"),
        "native_work_status": native.get("workStatus"), "native_work_partial_reason": native.get("workPartialReason"),
        "native_work_observation_frame_delta": native.get("observedWorkFramesDelta"),
        "native_lifetime_deadline_misses_delta": native.get("lifetimeDeadlineMissesDelta"),
        "decode_status": decode.get("status"), "decode_demand_wall_us_delta": decode.get("wallUsDelta"),
        "decode_demand_call_count_delta": decode.get("callCountDelta"), "decode_demand_mean_wall_ms": decode.get("meanDemandWallMs"),
        "decoder_session_opens_delta": decode.get("sessionOpensDelta"), "decoder_working_set_evictions_delta": decode.get("workingSetEvictionsDelta"),
        "decoder_session_resets_delta": decode.get("sessionResetsDelta"), "decoder_session_resets_reason": decode.get("sessionResetsReason"),
        "decoder_failures_delta": decode.get("failuresDelta"), "decoder_lookahead_hits_delta": decode.get("lookaheadHitsDelta"),
        "decoder_last_hardware_sessions": decode.get("lastHardwareSessionCount"), "decoder_last_sessions": decode.get("lastSessionCount"),
        "cpu_percent_of_32_logical_mean": usage["cpuPercentOf32LogicalMean"],
        "cpu_percent_of_32_logical_peak_interval": usage["cpuPercentOf32LogicalPeakInterval"],
        "cpu_interval_coverage": usage["cpuIntervalCoverage"], "process_membership_changed": usage["processMembershipChanged"],
        "cpu_accounting": usage["cpuAccounting"],
        "cpu_status": usage["cpuStatus"], "memory_status": usage["memoryStatus"], "gpu_provider_status": usage["gpuProviderStatus"],
        "working_set_bytes_mean": usage["workingSetBytesMean"], "working_set_bytes_peak": usage["workingSetBytesPeak"],
        "private_bytes_mean": usage["privateBytesMean"], "private_bytes_peak": usage["privateBytesPeak"],
        "gpu_3d_percent_mean": usage["gpu3dPercentMean"], "gpu_decode_percent_mean": usage["gpuDecodePercentMean"],
        "gpu_video_codec_percent_mean": usage["gpuVideoCodecPercentMean"], "gpu_video_codec_percent_peak": usage["gpuVideoCodecPercentPeak"],
        "gpu_video_codec_count_mean": usage["gpuVideoCodecCountMean"], "gpu_video_codec_coverage": usage["gpuVideoCodecCoverage"],
        "gpu_video_codec_valid_samples": usage["gpuVideoCodecValidSamples"], "gpu_video_codec_boundary": usage["gpuVideoCodecBoundary"],
        "gpu_copy_percent_mean": usage["gpuCopyPercentMean"], "gpu_provider_available": usage["gpuProviderAvailable"],
        "gpu_provider_coverage": usage["gpuProviderCoverage"], "gpu_valid_samples": usage["gpuValidSamples"],
        "gpu_3d_coverage": usage["gpu3dCoverage"], "gpu_decode_coverage": usage["gpuDecodeCoverage"], "gpu_copy_coverage": usage["gpuCopyCoverage"],
        "gpu_3d_count_mean": usage["gpu3dCountMean"], "gpu_decode_count_mean": usage["gpuDecodeCountMean"], "gpu_copy_count_mean": usage["gpuCopyCountMean"],
        "usage_samples": usage["sampleCount"], "usage_span_seconds": usage["spanSeconds"],
        "presentmon_status": case["presentMon"]["status"], "presentmon_process_id": case["presentMon"]["processId"],
        "presentmon_identity_status": case["presentMon"]["identityStatus"], "presentmon_matching_rows": case["presentMon"]["matchingRows"],
        "presentmon_minimum_rows": case["presentMon"]["minimumRows"],
        "presentmon_swapchain": pm.get("swapChainAddress"), "presentmon_interval_count": pm.get("intervalCount"),
        "presentmon_fps": pm.get("fps"), "presentmon_p50_ms": pm.get("p50Ms"), "presentmon_p95_ms": pm.get("p95Ms"),
        "presentmon_p99_ms": pm.get("p99Ms"), "presentmon_gpu_busy_ms_mean": pm.get("gpuBusyMsMean"),
        "presentmon_in_present_api_ms_mean": pm.get("inPresentApiMsMean"), "presentmon_until_displayed_ms_mean": pm.get("untilDisplayedMsMean"),
        "program_monitor_fps": case["monitor"]["programFps"], "preview_monitor_fps": case["monitor"]["previewFps"],
        "executable_sha256": case["executableSha256"], "fixture_sha256_before": case["fixtureSha256Before"],
        "fixture_sha256_after": case["fixtureSha256After"], "build": case["build"], "case_file": case["caseFile"],
        "measurement_issues": case["measurementIssues"], "cadence_reasons": cadence["reasons"], "native_reasons": native.get("reasons"),
        "usage_reasons": usage["reasons"], "presentmon_reasons": case["presentMon"]["reasons"],
        "error": case["error"], "restore_error": case["restoreError"], "stop_error": case["stopError"],
    }


def missing_row(coverage):
    spec = coverage["spec"]
    return [{"phase": coverage["phase"], "tag": coverage["tag"], "output_kind": kind,
             "output_id": None, "measurement_status": coverage["status"], "cadence_status": "not_established",
             "native_status": "not_established" if kind == "display" else "not_applicable", "envelope_status": "not_established",
             "case_cadence_status": "not_established", "case_native_status": "not_established",
             "source": spec.get("source"), "layers": spec.get("layers"), "fx": spec.get("fx"),
             "blend": spec.get("blend"), "cpu_requested": spec.get("cpu"), "groups": spec.get("groups"),
             "measurement_issues": [coverage.get("reason") or coverage["status"]]}
            for kind in expected_kinds(spec) or ["unknown"]]


def csv_cell(value):
    if isinstance(value, (dict, list)):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    return value


def md_cell(value, places=2):
    if value is None or value == "":
        return "—"
    if isinstance(value, float):
        value = f"{value:.{places}f}"
    if isinstance(value, list):
        value = "; ".join(str(item) for item in value)
    return str(value).replace("|", "\\|").replace("\r", " ").replace("\n", " ")


def markdown_table(headers, rows):
    return "\n".join(["| " + " | ".join(headers) + " |", "| " + " | ".join("---" for _ in headers) + " |"] +
                     ["| " + " | ".join(md_cell(value) for value in row) + " |" for row in rows])


def write_tables(path, summary, rows):
    sections = ["# Task 20 measurement tables", "",
                "`—` means unavailable or not applicable; it is not zero. These tables are generated from case evidence.", "",
                "## Measurement boundaries", "",
                "Delivered FPS uses per-output delivered-counter deltas divided by monotonic elapsed time. "
                "Monotonic timestamps are recorded before the asynchronous health query; no per-query completion timestamp or completion-latency bound is captured. "
                "Endpoint query-latency differences can therefore bias counter-delta FPS. "
                "It counts output submissions; a 30 FPS source does not provide 60 unique source images. "
                "Cadence percentiles are the last bounded health window. Its sampleCount is already the interval count "
                "(up to 599), so nothing is subtracted. "
                "Prepare percentiles are a separate duration window. Native work is an approximation from differences "
                "of rounded lifetime average × lifetime frame count; it includes preparation/present submission and possible "
                "vsync wait, and excludes pacing. Decode wall time is synchronous batch demand time; background lookahead is excluded. "
                "Session resets count in-place demux rewinds/decoder flushes. Session opens and working-set evictions are separate counters; "
                "reset counts alone do not establish cache churn.", "",
                "Native validation requires 600 frames, state `passed`, and `frame_budget_pass=true`. "
                "Native validation average/max and deadline-miss fraction describe that 600-frame or still-sampling native window, "
                "separately from the approximate observation mean derived from lifetime counters. An empty validation window has unavailable statistics. "
                "Cadence and native budget are reported independently. A low-output-rate sampling case can still be a cadence miss. "
                "Envelope pass requires complete measurement and every expected output passing cadence and native Display validation. "
                "A verified output miss remains a miss when the only gaps are an explicitly declared missing PresentMon capture or insufficient samples "
                "and/or unstarted native lifetime-work counters during verified pending warmup; "
                "its measurement validity stays partial and it can never pass the envelope. PresentMon complete statistics require at least 10 exact-process rows. "
                "A zero-row trace has unobserved PresentMon identity; no PresentMon FPS or percentiles are established. "
                "A missing trace remains PresentMon not_established with unobserved identity and unavailable row counts and timings; "
                "an absent-file partial verdict requires both missing-file and insufficient-sample markers and cannot establish an envelope pass. "
                "Pending-warmup full native work remains unavailable, not zero; output cadence, preparation and decode evidence retain their own boundaries.", "",
                "## Declared policy", "", "```json", json.dumps(summary["policy"], indent=2, ensure_ascii=False), "```", ""]
    if summary["policyErrors"]:
        sections.extend(["Policy errors: " + md_cell(summary["policyErrors"]), ""])
    sections.extend(["## Delivery and verdicts — one row per expected output", "", markdown_table(
        ["Phase", "Case", "Output", "Validity", "Cadence", "Native", "Envelope", "FPS / target", "Missed", "Cadence p50 / p95 / p99 ms", "Intervals"],
        [[row.get("phase"), row.get("tag"), str(row.get("output_kind")) + (":" + str(row["output_id"]) if row.get("output_id") is not None else ""),
          row.get("measurement_status"), row.get("cadence_status"), row.get("native_status"), row.get("envelope_status"),
          md_cell(row.get("delivered_fps")) + " / " + md_cell(row.get("target_fps")), row.get("missed_intervals_delta"),
          " / ".join(md_cell(row.get(key)) for key in ("cadence_p50_ms", "cadence_p95_ms", "cadence_p99_ms")), row.get("cadence_interval_count")]
         for row in rows]), ""])
    measured_rows = [row for row in rows if row.get("case_file")]
    sections.extend(["## Prepare, native work and synchronous decode", "", markdown_table(
        ["Phase", "Case", "Output", "Prepare p50 / p95 / p99 ms", "Prepare n", "Native state / n", "Native validation avg / max ms", "Native validation miss fraction", "Observed native work ms ≈", "Decode wall ms / calls", "Opens / evictions / resets"],
        [[row.get("phase"), row.get("tag"), row.get("output_kind"),
          " / ".join(md_cell(row.get(key)) for key in ("prepare_p50_ms", "prepare_p95_ms", "prepare_p99_ms")), row.get("prepare_sample_count"),
          md_cell(row.get("native_validation_state")) + " / " + md_cell(row.get("native_validation_frame_count")),
          " / ".join(md_cell(row.get(key)) for key in ("native_validation_average_ms", "native_validation_max_ms")),
          md_cell(row.get("native_validation_deadline_miss_fraction"), places=4), row.get("native_work_observation_mean_ms_approx"),
          md_cell(row.get("decode_demand_wall_us_delta") / 1000 if row.get("decode_demand_wall_us_delta") is not None else None) + " / " + md_cell(row.get("decode_demand_call_count_delta")),
          " / ".join(md_cell(row.get(key)) for key in ("decoder_session_opens_delta", "decoder_working_set_evictions_delta", "decoder_session_resets_delta"))]
         for row in measured_rows]), ""])
    display_rows = [row for row in measured_rows if row.get("output_kind") == "display"]
    sections.extend(["## Process tree, GPU, PresentMon and monitors", "",
                     "CPU is normalized to 32 logical processors; cumulative readings are retained by PID and process start time, "
                     "including exited processes. Membership changes are a note rather than an invalidation. Memory is the sampled sum over the process tree. "
                     "GPU percentages sum reported matching engine instances and are not a device-wide utilization limit. "
                     "The video-codec metric is the sum of instances named exactly `Video Codec Engine`, can exceed 100%, "
                     "and is kept separate from the decode counter; an unavailable decode counter remains unavailable. "
                     "An engine category with no matching instances is unobserved, not zero; provider and per-category coverage are recorded separately. "
                     "PresentMon uses the exact native PID and a single swapchain. Background build overlap is recorded only when in-sample "
                     "monitoring is declared available; otherwise overlap is unknown. Process/PresentMon/monitor metrics repeat across outputs only in CSV.", "",
                     markdown_table(
        ["Phase", "Case", "CPU %", "WS mean / peak GiB", "Private mean / peak GiB", "GPU 3D / video-codec / copy %", "GPU provider / video-codec coverage", "PM status / rows", "PM FPS", "PM p50 / p95 / p99 ms", "Monitor PGM / PVW FPS"],
        [[row.get("phase"), row.get("tag"), row.get("cpu_percent_of_32_logical_mean"),
          " / ".join(md_cell(row.get(key) / 1024 ** 3 if row.get(key) is not None else None) for key in ("working_set_bytes_mean", "working_set_bytes_peak")),
          " / ".join(md_cell(row.get(key) / 1024 ** 3 if row.get(key) is not None else None) for key in ("private_bytes_mean", "private_bytes_peak")),
          " / ".join(md_cell(row.get(key)) for key in ("gpu_3d_percent_mean", "gpu_video_codec_percent_mean", "gpu_copy_percent_mean")),
          md_cell(row.get("gpu_provider_coverage")) + " / " + md_cell(row.get("gpu_video_codec_coverage")),
          md_cell(row.get("presentmon_status")) + " / " + md_cell(row.get("presentmon_matching_rows")), row.get("presentmon_fps"),
          " / ".join(md_cell(row.get(key)) for key in ("presentmon_p50_ms", "presentmon_p95_ms", "presentmon_p99_ms")),
          md_cell(row.get("program_monitor_fps")) + " / " + md_cell(row.get("preview_monitor_fps"))]
         for row in display_rows]), ""])
    coverage_rows = [row for row in summary["coverage"]["cases"] if row["status"] != "measured"]
    sections.extend(["## Planned coverage still incomplete or unsupported", "", markdown_table(
        ["Phase", "Case", "Status", "Reports", "Evidence"],
        [[row["phase"], row["tag"], row["status"], row["reportCount"], row["reason"]] for row in coverage_rows]), ""])
    issue_cases = [case for case in summary["cases"] if case["measurementIssues"]]
    if issue_cases:
        sections.extend(["## Invalid or incomplete evidence", "", markdown_table(
            ["Phase", "Case", "Status", "Issues", "Workload / restore / stop error"],
            [[case["phase"], case["tag"], case["measurementStatus"], case["measurementIssues"],
              [str(case[key]) for key in ("error", "restoreError", "stopError") if error_present(case[key])]]
             for case in issue_cases]), ""])
    if summary["inputErrors"]:
        sections.extend(["## Unreadable inputs", "", "\n".join("- " + md_cell(error) for error in summary["inputErrors"]), ""])
    path.write_text("\n".join(sections), encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    root, output = args.root.resolve(), (args.out_dir or args.root).resolve()
    if not root.is_dir():
        parser.error(f"Input directory does not exist: {root}")
    policy, policy_error = load_json(root / "policy.json")
    policy = policy if isinstance(policy, dict) else {}
    policy_errors = [policy_error] if policy_error else []
    for key in POLICY_KEYS:
        if key == "nativeBudgetRequired":
            if not isinstance(policy.get(key), bool):
                policy_errors.append(f"policy.{key} must be boolean")
        elif number(policy.get(key)) is None:
            policy_errors.append(f"policy.{key} must be a finite number")
        else:
            policy[key] = number(policy[key])
    if not policy_errors:
        if not 0 < policy["cadenceMinimumTargetRatio"] <= 1:
            policy_errors.append("cadenceMinimumTargetRatio must be in (0, 1]")
        if not 0 <= policy["missedFractionMaximum"] < 1:
            policy_errors.append("missedFractionMaximum must be in [0, 1)")
        if policy["cadenceP99MaximumTargetIntervals"] <= 0:
            policy_errors.append("cadenceP99MaximumTargetIntervals must be positive")
    minimum_frames = NATIVE_MINIMUM_FRAMES
    declared_minimum = integer(policy.get("nativeMinimumFrames"))
    if declared_minimum is not None:
        minimum_frames = max(NATIVE_MINIMUM_FRAMES, declared_minimum)
    raw_specs, specs_error = load_json(root / "cases.json")
    specs = case_specs(raw_specs)
    order = {str(spec["tag"]): index for index, spec in enumerate(specs)}
    cases, input_errors = [], [specs_error] if specs_error else []
    paths = sorted(set(root.glob("before-*.json")) | set(root.glob("after-*.json")))
    for path in paths:
        if path.name.endswith("-usage.json") or path.name in ("before-cases.json", "after-build.json", "before-build.json"):
            continue
        value, error = load_json(path)
        if error:
            input_errors.append(error)
        elif isinstance(value, dict) and isinstance(value.get("spec"), dict):
            try:
                cases.append(summarize_case(path, value, policy, policy_errors, minimum_frames))
            except (KeyError, TypeError, ValueError, ArithmeticError) as error:
                input_errors.append(f"{path.name}: summary rejected malformed evidence: {type(error).__name__}: {error}")
    cases.sort(key=lambda case: (order.get(case["tag"], len(order)), 0 if case["phase"] == "before" else 1, case["caseFile"]))
    coverage = coverage_metrics(root, cases, specs)
    summary = {
        "schemaVersion": 1, "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "root": str(root), "policyFile": str(root / "policy.json"), "policy": policy,
        "policyErrors": policy_errors, "inputErrors": input_errors,
        "method": {
            "primaryFps": "delta(deliveredFrames) / delta(monotonicMs) * 1000 per output",
            "timestampBoundary": "monotonicMs is recorded before the asynchronous health query; no per-query completion timestamp or completion-latency bound is captured, so endpoint latency differences can bias counter-delta FPS",
            "cadenceDistribution": "last bounded health window; backend sampleCount is intervals.len(), up to 599, retained without subtraction",
            "missedIntervals": "delta(droppedFrames); this is the telemetry high-water missed-interval estimate",
            "missedFraction": "delta(droppedFrames) / (delta(deliveredFrames) + delta(droppedFrames))",
            "nativeWork": "approximate delta(lifetime_average_frame_us * lifetime_frame_count) / delta(lifetime_frame_count) / 1000; rounded cumulative averages",
            "nativeWorkBoundary": "preparation/present submission + possible vsync wait; pacing excluded",
            "nativePass": f"validation_state=passed AND frame_count>={minimum_frames} AND frame_budget_pass=true",
            "nativeValidationWindow": "native average_frame_us/1000, max_frame_us/1000 and deadline_miss_count/frame_count use the 600-frame or still-sampling native validation window, distinct from the approximate lifetime-counter observation mean; zero/missing frame count gives unavailable window statistics",
            "decodeBoundary": "synchronous batch demand wall time; background lookahead excluded",
            "decoderSessionResets": "optional delta(libav_session_reset_count): in-place demux rewind/decoder flush, separate from session opens and working-set evictions; resets alone do not establish cache churn; unavailable/nonmonotonic values remain null with a separate reason",
            "cpu": f"delta of retained cumulative CPU readings keyed by PID plus process start time, including exited processes / elapsed seconds / {LOGICAL_PROCESSORS} * 100",
            "memory": "arithmetic sample mean and peak of summed process-tree working set and private bytes",
            "gpu": "provider valid when gpuError absent and gpuEngineCount>0; each engine category is averaged only when its count>0 and percentage is finite; provider and per-category coverage reported separately; absent categories stay null",
            "gpuVideoCodec": "per-valid-provider-sample sum of UtilizationPercentage for exact raw _engtype_Video Codec Engine instances; separate from decode, not clamped to 100, not device-wide utilization; category coverage includes only observed finite sums",
            "backgroundBuildOverlap": "copied from instrumentation only when backgroundBuildSamplingAvailable=true; unavailable monitoring means overlap unknown, including before measurements without in-sample host-build monitoring",
            "presentMon": "complete statistics require at least 10 exact launch PID/syndocal.exe rows, exactly one swapchain, finite nonnegative timings and at least 2 positive intervals; FPS=1000/mean positive MsBetweenPresents; insufficient samples or an explicitly declared absent CSV can retain a verified miss as partial evidence, never an envelope pass; absent CSV remains not_established with unobserved identity and null counts/timings",
            "sourceCadence": "delivered counters count submissions, not unique source images; 30 FPS media can be submitted at 60 FPS",
            "percentiles": "PresentMon linear interpolation over sorted positive intervals; health percentiles retained as reported",
        },
        "counts": {"caseReports": len(cases), "measurementStatus": dict(Counter(case["measurementStatus"] for case in cases)),
                   "envelopeStatus": dict(Counter(case["envelopeStatus"] for case in cases))},
        "cases": cases, "coverage": coverage,
    }
    rows = [flattened_row(case, one_output) for case in cases for one_output in case["outputs"]]
    for item in coverage["cases"]:
        if item["status"] in ("missing_case", "unsupported") and item["reportCount"] == 0:
            rows.extend(missing_row(item))
    rows.sort(key=lambda row: (order.get(row["tag"], len(order)), 0 if row["phase"] == "before" else 1,
                              KINDS.index(row["output_kind"]) if row["output_kind"] in KINDS else 99,
                              str(row.get("output_id", "")), str(row.get("case_file", ""))))
    output.mkdir(parents=True, exist_ok=True)
    (output / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2, allow_nan=False) + "\n", encoding="utf-8", newline="\n")
    columns = list(dict.fromkeys(key for row in rows for key in row))
    if not columns:
        columns = ["phase", "tag", "output_kind", "measurement_status", "cadence_status", "native_status", "envelope_status"]
    with (output / "measurements.csv").open("w", encoding="utf-8-sig", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        writer.writerows({key: csv_cell(value) for key, value in row.items()} for row in rows)
    write_tables(output / "tables.md", summary, rows)
    print(json.dumps({"caseReports": len(cases), "deliveryRows": len(rows), "envelopeStatus": summary["counts"]["envelopeStatus"],
                      "missingSupported": len(coverage["missingSupportedCases"]), "unsupported": len(coverage["unsupportedCases"]),
                      "inputErrors": len(input_errors), "policyErrors": len(policy_errors), "outputDirectory": str(output)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
