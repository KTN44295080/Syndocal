#!/usr/bin/env python3
"""Write allowlisted Task20 provenance; never copy raw state, errors or payloads."""
import argparse
import datetime as dt
import hashlib
import json
import math
import re
from collections import Counter
from pathlib import Path

TAG = re.compile(r"[a-z0-9][a-z0-9_-]{0,159}")
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,239}")
SHA = re.compile(r"[0-9a-fA-F]{64}")
PRIVATE = re.compile(r"\b[a-z]:[\\/]|[\\/]+Users[\\/]+|[\\/]+(?:home|root)[\\/]+|\\\\[a-z0-9_.-]+[\\/]|(?:https?|file)://|localstorage|original[-_]?storage|raw[-_]?storage|errorui", re.I)
BUILD_FIELDS = {
    "phase": "phase", "baselineKind": "text", "description": "text", "createdAt": "date",
    "sourceHead": "commit", "mergedMain": "commit", "command": "text",
    "cargoTargetDirUnset": "bool", "ndiEnabled": "bool", "executableSha256": "sha",
    "executableBytes": "int", "sourcePatchSha256": "sha", "sourceRestoredExactly": "bool",
    "buildLog": "relative", "nativeGateLog": "relative",
}
INSTRUMENT_FIELDS = {
    "valid": "bool", "partialReasons": "codes", "backgroundBuildSamplingAvailable": "bool",
    "backgroundBuildOverlap": "bool", "usageSamples": "int", "usageSpanSeconds": "number",
    "presentRows": "int", "presentProcessId": "int", "presentIdentityObserved": "bool",
    "presentFileMissing": "bool",
}
SPEC_FIELDS = {"tag": "tag", "layers": "int", "source": "name", "output": "token",
               "fx": "int", "blend": "token", "cpu": "bool", "groups": "bool"}
NOTES = [
    "Local raw evidence remains ignored/private; this file contains selected metadata, hashes and observed restoration booleans only.",
    "Before is a controlled counterfactual on source c204074d: libav cache eight plus ISF clone; after uses cache sixteen plus ISF borrow. Eight workers are unchanged. This compares the combined change, not a historical release or isolated per-fix gains.",
    "Harness hardening differed between phases, including background compiler monitoring and partial-evidence handling. Before lacked per-sample host-build monitoring. Preserve reported phase/case harness hashes; current harness hashes do not retrospectively prove earlier bytes.",
    "Missing restoration observations remain null. A recorded stop is presence evidence, not a success claim. Final restore proof is hashed only and requires separate review.",
    "Hashes identify retained raw evidence without publishing it. Repeating the workload is possible from the harness and metadata; recomputing original aggregate metrics requires the private numeric input records.",
]


def safe_text(value, where, limit=1600):
    if not isinstance(value, str) or len(value) > limit or PRIVATE.search(value) or any(ord(c) < 32 for c in value):
        raise ValueError(f"{where}: unreviewed text")
    return value


def checked(value, kind, where):
    if value is None:
        return None
    if kind == "bool":
        if type(value) is not bool:
            raise ValueError(f"{where}: expected boolean")
    elif kind in ("number", "int"):
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError(f"{where}: invalid nonnegative number")
        if kind == "int" and (not isinstance(value, int)):
            raise ValueError(f"{where}: expected integer")
    elif kind == "codes":
        if not isinstance(value, list):
            raise ValueError(f"{where}: expected reason-code list")
        return [checked(item, "token", where) for item in value]
    else:
        safe_text(value, where)
        patterns = {"sha": SHA, "tag": TAG, "name": NAME,
                    "token": re.compile(r"[A-Za-z0-9][A-Za-z0-9_. -]{0,119}"),
                    "commit": re.compile(r"[0-9a-fA-F]{7,40}")}
        if kind in patterns and not patterns[kind].fullmatch(value):
            raise ValueError(f"{where}: invalid {kind}")
        if kind == "phase" and value not in ("before", "after"):
            raise ValueError(f"{where}: invalid phase")
        if kind == "date":
            parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
            if parsed.tzinfo is None:
                raise ValueError(f"{where}: timezone missing")
        if kind == "relative":
            if "\\" in value or value.startswith("/") or any(not NAME.fullmatch(part) or part in (".", "..") for part in value.split("/")):
                raise ValueError(f"{where}: invalid relative path")
    return value


def pick(value, fields, where):
    if not isinstance(value, dict):
        raise ValueError(f"{where}: expected object")
    return {key: checked(value[key], kind, f"{where}.{key}") for key, kind in fields.items() if key in value}


def ordinary(path, directory=False):
    if path.is_symlink() or getattr(path, "is_junction", lambda: False)():
        raise ValueError(f"{path.name}: linked evidence entry requires review")
    return path.is_dir() if directory else path.is_file()


def load(path):
    if not ordinary(path):
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except (ValueError, UnicodeError):
        raise ValueError(f"{path.name}: unreadable JSON") from None


def artifact(path):
    if not ordinary(path):
        return None
    checked(path.name, "name", "artifact name")
    digest, length = hashlib.sha256(), 0
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
            length += len(chunk)
    return {"name": path.name, "bytes": length, "sha256": digest.hexdigest()}


def hash_mapping(value, where, rust=False):
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError(f"{where}: expected hash mapping")
    result = {}
    for name, digest in value.items():
        checked(name, "relative", where)
        if rust:
            if not name.endswith(".rs") or not name.startswith(("crates/", "app/src-tauri/")):
                raise ValueError(f"{where}: unexpected source path")
        elif name != "policy.json" and Path(name).suffix not in (".mjs", ".ps1", ".py"):
            raise ValueError(f"{where}: unexpected harness extension")
        if digest is None:
            raise ValueError(f"{where}: missing digest")
        result[name] = checked(digest, "sha", where)
    return result


def entries(value):
    if value is None:
        return []
    if isinstance(value, list):
        return value
    if isinstance(value, dict):
        for key in ("cases", "tags", "caseTags", "specs"):
            if isinstance(value.get(key), list):
                return value[key]
        if "tag" in value or "spec" in value:
            return [value]
    raise ValueError("Case plan: unsupported shape")


def selected_specs(value):
    result = {}
    for entry in entries(value):
        raw = {"tag": entry} if isinstance(entry, str) else entry
        if isinstance(raw, dict) and isinstance(raw.get("spec"), dict):
            raw = raw["spec"]
        spec = pick(raw, SPEC_FIELDS, "case spec")
        if not spec.get("tag") or spec["tag"] in result:
            raise ValueError("Case plan: missing or duplicate tag")
        result[spec["tag"]] = spec
    return result


def build_record(root, phase):
    path = root / (phase + "-build.json")
    raw = load(path)
    if raw is None:
        return None
    metadata = pick(raw, BUILD_FIELDS, path.name)
    if "phase" in metadata and metadata["phase"] != phase:
        raise ValueError(f"{path.name}: phase mismatch")
    if "controlledSources" in raw:
        metadata["controlledSources"] = hash_mapping(raw["controlledSources"], path.name, rust=True)
    logs = {}
    for key in ("buildLog", "nativeGateLog"):
        if metadata.get(key):
            candidate = root / metadata[key]
            # Only explicitly named relative paths; never traverse directories.
            for parent in candidate.parents:
                if parent == root:
                    break
                ordinary(parent, directory=True)
            logs[key] = artifact(candidate)
    return {"metadata": metadata, "artifact": artifact(path), "logs": logs}


def observed_windows_off(value):
    if isinstance(value, dict) and isinstance(value.get("windows"), list):
        value = value["windows"]
    if not isinstance(value, list):
        return None
    known = True
    for window in value:
        if not isinstance(window, dict):
            known = False
            continue
        for field in ("live_open", "test_pattern_open"):
            if window.get(field) is True:
                return False
            if type(window.get(field)) is not bool:
                known = False
    return True if known else None


def restoration(report, original, original_available):
    role = report.get("restoredRole")
    role = role.get("role") if isinstance(role, dict) else None
    recovery = report.get("cleanLaunchRestore")
    recovery = recovery.get("ok") if isinstance(recovery, dict) else None
    return {
        "storageMatchesOriginal": (json.dumps(report["restoredStorage"], sort_keys=True, ensure_ascii=False)
                                   == json.dumps(original, sort_keys=True, ensure_ascii=False)
                                   if original_available and original is not None
                                   and report.get("restoredStorage") is not None else None),
        "roleStandbyObserved": role.lower() == "standby" if isinstance(role, str) else None,
        "windowsOffObserved": observed_windows_off(report.get("restoredWindows")),
        "cleanLaunchRecoveryOK": recovery if type(recovery) is bool else None,
        "stopRecorded": "stop" in report,
    }


def case_record(root, phase, spec, build, original, original_available):
    tag, stem = spec["tag"], phase + "-" + spec["tag"]
    path = root / (stem + ".json")
    report = load(path)
    unsupported = spec.get("output") == "ndi" and (build or {}).get("metadata", {}).get("ndiEnabled") is False
    result = {"phase": phase, "tag": tag, "recordStatus": "unsupported" if unsupported else "missing",
              "unsupportedReason": "ndi_not_built" if unsupported else None,
              "reportPresent": report is not None,
              "artifacts": {"report": artifact(path), "usage": artifact(root / (stem + "-usage.json")),
                            "presentMon": artifact(root / (stem + "-presentmon.csv"))}}
    if report is None:
        return result
    if not isinstance(report, dict):
        raise ValueError(f"{path.name}: expected object")
    report_spec = report.get("spec")
    if not isinstance(report_spec, dict) or report_spec.get("tag") != tag:
        raise ValueError(f"{path.name}: case identity mismatch")
    selected = pick(report_spec, SPEC_FIELDS, path.name)
    if any(key in selected and selected[key] != value for key, value in spec.items()):
        raise ValueError(f"{path.name}: report workload differs from plan")
    result["ok"] = checked(report.get("ok"), "bool", path.name)
    if not unsupported:
        result["recordStatus"] = "reported" if result["ok"] is True else "reported_not_ok"
    for key in ("executableSha256", "fixtureSha256Before", "fixtureSha256After"):
        result[key] = checked(report.get(key), "sha", f"{path.name}.{key}")
    result["harnessSha256"] = hash_mapping(report.get("harnessSha256"), path.name)
    samples = report.get("samples")
    samples = samples if isinstance(samples, list) else []
    for key, sample in (("sampleStartAt", samples[0] if samples else None), ("sampleEndAt", samples[-1] if samples else None)):
        result[key] = checked(sample.get("at") if isinstance(sample, dict) else None, "date", path.name)
    raw_instrument = report.get("instrumentation")
    result["instrumentation"] = pick(raw_instrument, INSTRUMENT_FIELDS, path.name) if isinstance(raw_instrument, dict) else {}
    result["restoration"] = restoration(report, original, original_available)
    result["errorFlags"] = {key: report.get(key) not in (None, False, "", [], {}) for key in ("error", "restoreError", "stopError")}
    return result


def duration(value, where):
    if value in (None, "N/A"):
        return None
    if isinstance(value, str):
        safe_text(value, where, 40)
        if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)?", value):
            raise ValueError(f"{where}: invalid duration")
        return value
    return checked(value, "number", where)


def media_records(root):
    value = load(root / "media-manifest.json")
    if value is None:
        return None
    if not isinstance(value, list):
        raise ValueError("media-manifest.json: expected array")
    result = []
    for raw in value:
        item = pick(raw, {"name": "name", "bytes": "int", "sha256": "sha"}, "media entry")
        probe = raw.get("ffprobe") if isinstance(raw.get("ffprobe"), dict) else {}
        streams = []
        for stream in probe.get("streams", []):
            if not isinstance(stream, dict) or stream.get("codec_type") not in (None, "video"):
                continue
            clean = pick(stream, {"codec_name": "token", "width": "int", "height": "int", "pix_fmt": "token"}, "ffprobe stream")
            for key in ("r_frame_rate", "avg_frame_rate"):
                if stream.get(key) is not None:
                    rate = safe_text(stream[key], key, 40)
                    if not re.fullmatch(r"[0-9]+/[0-9]+", rate):
                        raise ValueError(f"ffprobe {key}: invalid rational")
                    clean[key] = rate
            if "duration" in stream:
                clean["duration"] = duration(stream["duration"], "stream duration")
            streams.append(clean)
        format_data = probe.get("format") if isinstance(probe.get("format"), dict) else {}
        item["ffprobe"] = {"streams": streams, "duration": duration(format_data.get("duration"), "format duration")}
        result.append(item)
    return result


def archive_records(root, known_tags):
    result = []
    validation = root / "validation"
    if not ordinary(validation, directory=True):
        return result
    for name in ("initial-sparse-presentmon", "concurrent-build-attempts", "shutdown-timeout-attempts", "presentmon-missing-attempts", "lease-renewal-attempts", "native-denial-attempts"):
        directory = validation / name
        if not ordinary(directory, directory=True):
            continue
        candidates = [directory / "manifest.json"]
        for child in sorted(directory.iterdir()):
            if ordinary(child, directory=True):
                checked(child.name, "name", "archive directory")
                candidates.append(child / "manifest.json")
        for candidate in candidates:
            raw = load(candidate)
            if raw is None:
                continue
            tags = set()
            if isinstance(raw, dict):
                for key in ("tag", "caseTag", "tags", "caseTags", "cases"):
                    value = raw.get(key)
                    for entry in value if isinstance(value, list) else [value]:
                        if isinstance(entry, dict):
                            entry = entry.get("tag")
                        if isinstance(entry, str) and entry in known_tags:
                            tags.add(entry)
            result.append({"archive": candidate.parent.relative_to(root).as_posix(),
                           "artifact": artifact(candidate), "caseTags": sorted(tags)})
    return result


def generate(root, destination, png_names):
    root = root.resolve()
    if not root.is_dir():
        raise ValueError("Raw directory does not exist")
    if destination.resolve().parent == root and destination.exists():
        raise ValueError("Output would overwrite an existing raw entry; choose a separate filename/directory")
    specs = selected_specs(load(root / "cases.json"))
    before = selected_specs(load(root / "before-cases.json"))
    for tag, spec in before.items():
        if tag not in specs or any(key in specs[tag] and specs[tag][key] != value for key, value in spec.items()):
            raise ValueError("Before plan disagrees with master plan")
    builds = {phase: build_record(root, phase) for phase in ("before", "after")}
    original_path = root / "original-storage.json"
    original_available = ordinary(original_path)
    original = load(original_path) if original_available else None
    records = [case_record(root, phase, specs[tag], builds[phase], original, original_available)
               for phase, tags in (("before", before), ("after", specs)) for tag in tags]
    direct = [path for path in sorted(root.iterdir()) if path != destination.resolve()
              and not path.is_symlink() and not getattr(path, "is_junction", lambda: False)()
              and path.is_file()]
    pngs = [root / checked(name, "name", "selected PNG") for name in png_names] if png_names else [path for path in direct if path.suffix.lower() == ".png"]
    if any(path.suffix.lower() != ".png" or not ordinary(path) for path in pngs):
        raise ValueError("Selected PNG is absent or invalid")
    result = {
        "schemaVersion": 1, "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "notes": NOTES, "plan": {"cases": list(specs.values()), "beforeTags": list(before)},
        "builds": builds, "cases": records,
        "coverage": {phase: dict(Counter(record["recordStatus"] for record in records if record["phase"] == phase)) for phase in ("before", "after")},
        "inputs": {name: artifact(root / relative) for name, relative in (("cases.json", "cases.json"), ("before-cases.json", "before-cases.json"), ("policy.json", "policy.json"), ("media-manifest.json", "media-manifest.json"), ("before-source.diff", "validation/before-source.diff"), ("restore-proof.json", "restore-proof.json"))},
        "media": media_records(root),
        "currentHarness": [artifact(path) for path in direct if path.suffix in (".mjs", ".ps1", ".py")],
        "runnerLogs": [artifact(path) for path in direct if path.suffix == ".log"],
        "launchRecords": [artifact(path) for path in direct if path.suffix == ".json" and "launch" in path.stem.lower()],
        "screenshots": [artifact(path) for path in pngs],
        "archives": archive_records(root, set(specs)),
    }
    # Inspect selected strings rather than silently erasing an unexpected value.
    def audit(value):
        if isinstance(value, dict):
            for key, child in value.items():
                safe_text(key, "output key")
                audit(child)
        elif isinstance(value, list):
            for child in value:
                audit(child)
        elif isinstance(value, str):
            safe_text(value, "output string")
    audit(result)
    destination.write_text(json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"cases": len(records), "coverage": result["coverage"], "harnessFiles": len(result["currentHarness"]),
                      "archiveManifests": len(result["archives"]), "output": destination.name}, sort_keys=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_dir", type=Path)
    parser.add_argument("output_json", type=Path)
    parser.add_argument("--png", action="append", default=[], help="Select a direct PNG basename; defaults to all direct PNGs (hashes only)")
    args = parser.parse_args()
    try:
        generate(args.raw_dir, args.output_json, args.png)
    except (ValueError, OSError, UnicodeError) as error:
        # Do not echo OS exception filenames or raw JSON content.
        message = str(error) if isinstance(error, ValueError) else type(error).__name__
        parser.exit(1, "Provenance rejected: " + message + "\n")
