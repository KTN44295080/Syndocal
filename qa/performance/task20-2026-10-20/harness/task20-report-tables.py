#!/usr/bin/env python3
"""Render Task20 measurement CSV tables without recalculating acceptance gates.

Usage: python /tmp/task20-report-tables.py [measurements.csv] > tables.md
"""

import argparse
import csv
from collections import defaultdict
from decimal import Decimal, InvalidOperation
from pathlib import Path


LAYERS = (1, 3, 6, 8, 12, 16)
SOURCES = (
    ("h264-1080p30", "H.264 1080p"),
    ("h264-2160p30", "H.264 4K"),
    ("hap-1080p30", "HAP 1080p"),
    ("dxv-1080p30", "DXV 1080p"),
)
BLENDS = (
    ("normal", "Normal"), ("add", "Add"), ("multiply", "Multiply"),
    ("screen", "Screen"), ("overlay", "Overlay"), ("subtract", "Subtract"),
    ("difference", "Difference"), ("lighten", "Lighten"),
    ("darken", "Darken"), ("softlight", "Soft light"),
)
UNAVAILABLE = "unavailable"


def clean(value):
    return str(value or "").strip()


def boolean(value):
    value = clean(value).lower()
    if value in ("true", "1"):
        return True
    if value in ("false", "0"):
        return False
    return None


def number(value, count=False):
    """At most three decimal places; FPS always retains three places."""
    value = clean(value)
    if not value or value.lower() in ("null", "none", "nan", "n/a"):
        return UNAVAILABLE
    try:
        parsed = Decimal(value)
    except InvalidOperation:
        return value
    if not parsed.is_finite():
        return UNAVAILABLE
    if count and parsed == parsed.to_integral_value():
        return format(parsed, ".0f")
    result = format(parsed, ".3f")
    return "0.000" if result == "-0.000" else result


def metric(row, column, count=False):
    return number(row.get(column), count) if row else UNAVAILABLE


def status(row):
    """Preserve source status, and never promote incomplete evidence to pass."""
    if row is None:
        return "not_established [missing output; unavailable]"
    measurement = clean(row.get("measurement_status")) or "unavailable"
    envelope = clean(row.get("envelope_status")) or "not_established"
    invalid_pass = envelope == "pass" and (
        measurement != "complete"
        or boolean(row.get("workload_ok")) is not True
        or boolean(row.get("instrumentation_valid")) is not True
        or boolean(row.get("background_build_overlap")) is True
        or boolean(row.get("requires_stop_before_further_cases")) is True
    )
    if invalid_pass:
        return f"not_established [{measurement}; CSV envelope=pass withheld]"
    if measurement in ("missing_case", "unsupported"):
        return f"{envelope} [{measurement}; unavailable]"
    if measurement != "complete":
        return f"{envelope} [{measurement}]"
    return envelope


def sweep_cell(row):
    return f"{metric(row, 'delivered_fps')} / {status(row)}"


def escape(value):
    return str(value).replace("|", "\\|").replace("\r", " ").replace("\n", " ")


def table(headers, rows):
    lines = ["| " + " | ".join(map(escape, headers)) + " |"]
    lines.append("| " + " | ".join("---" for _ in headers) + " |")
    lines.extend("| " + " | ".join(map(escape, row)) + " |" for row in rows)
    return "\n".join(lines)


class Measurements:
    def __init__(self, rows):
        self.rows = rows
        self.index = defaultdict(list)
        seen = set()
        for row in rows:
            key = tuple(clean(row.get(k)) for k in ("phase", "tag", "output_kind", "output_id"))
            if key in seen:
                raise ValueError(f"Duplicate measurement identity: {key}; choose a canonical CSV first")
            seen.add(key)
            self.index[key[:3]].append(row)

    def get(self, phase, tag, kind="display", output_id=None):
        matches = self.index.get((phase, tag, kind), [])
        if output_id:
            exact = [row for row in matches if clean(row.get("output_id")) == output_id]
            if exact:
                return exact[0]
            # Planned missing/unsupported rows have no output ID. They supply
            # unavailability, never substitute another measured output.
            matches = [row for row in matches if not clean(row.get("output_id"))
                       and row.get("measurement_status") in ("missing_case", "unsupported")]
        if len(matches) > 1:
            raise ValueError(f"Ambiguous output identity: {(phase, tag, kind, output_id)}")
        return matches[0] if matches else None


def short_case(row):
    tag = row["tag"]
    source = next((label for token, label in SOURCES if tag.startswith(token + "-")), row.get("source") or tag)
    label = f"{source}, {clean(row.get('layers')) or '?'}L"
    if "-display4k-" in tag:
        variant = "Display 4K"
    elif "-spout-" in tag:
        variant = "Display + Spout"
    elif "-ndi-" in tag:
        variant = "Display + NDI"
    elif tag.endswith("-cpu") or boolean(row.get("cpu_requested")):
        variant = "CPU requested"
    elif tag.endswith("-groups") or boolean(row.get("groups")):
        variant = "Groups"
    elif clean(row.get("fx")) not in ("", "0", "0.0"):
        variant = f"{number(row['fx'], count=True)} FX/layer"
    elif clean(row.get("blend")) not in ("", "Normal"):
        variant = row["blend"]
    else:
        variant = "Display"
    return f"{label}, {variant}"


def pair_metric(before, after, column, count=False):
    return f"{metric(before, column, count)} → {metric(after, column, count)}"


def paired_table(data):
    rows = []
    before_rows = [row for row in data.rows if row.get("phase") == "before"]
    for before in before_rows:
        after = data.get("after", before["tag"], before["output_kind"], clean(before.get("output_id")))
        output = before["output_kind"].capitalize()
        if before.get("output_id"):
            output += f" #{before['output_id']}"
        churn = lambda row: f"{metric(row, 'decoder_session_opens_delta', True)}/{metric(row, 'decoder_working_set_evictions_delta', True)}"
        rows.append([
            short_case(before), output,
            pair_metric(before, after, "delivered_fps"),
            pair_metric(before, after, "missed_intervals_delta", True),
            pair_metric(before, after, "prepare_p95_ms"),
            pair_metric(before, after, "decode_demand_mean_wall_ms"),
            f"{churn(before)} → {churn(after)}",
            f"{status(before)} → {status(after)}",
        ])
    headers = ["Case", "Output", "FPS: before → after", "Δmissed: before → after",
               "Prepare p95 (ms): before → after", "Decode demand mean (ms/call): before → after",
               "Δopens/Δevictions: before → after", "Envelope: before → after"]
    return table(headers, rows), len(before_rows), len({r["tag"] for r in before_rows})


def render(data):
    blocks = [
        "## After: source codec sweep",
        "Cells show delivered output FPS / envelope status. Display is 1080p with normal blend and no FX unless stated. "
        "FPS has three decimal places; missing metrics stay unavailable. Status tokens come from the CSV. "
        "A workload_failed or partial row is never promoted to pass; diagnostic measurements can remain visible beside its restricted status.",
    ]
    headers = ["Source (30 fps)"] + [f"{layers}L" for layers in LAYERS]
    rows = []
    for token, label in SOURCES:
        rows.append([label] + [sweep_cell(data.get("after", f"{token}-{n}l-display-0fx-normal")) for n in LAYERS])
    blocks.append(table(headers, rows))

    blocks += ["## After: output and FX variants", "All variants use H.264 1080p sources and normal blend. "
               "The two Display + Spout rows are separate outputs from the same cases. "
               "NDI rows retain the build's unsupported classification."]
    variants = [
        ("Display 4K", "display4k-0fx-normal", "display"),
        ("Display + Spout: Display", "spout-0fx-normal", "display"),
        ("Display + Spout: Spout", "spout-0fx-normal", "spout"),
        ("Display, 1 FX/layer", "display-1fx-normal", "display"),
        ("Display, 3 FX/layer", "display-3fx-normal", "display"),
        ("Display + NDI: Display", "ndi-0fx-normal", "display"),
        ("Display + NDI: NDI", "ndi-0fx-normal", "ndi"),
    ]
    rows = [[label] + [sweep_cell(data.get("after", f"h264-1080p30-{n}l-{suffix}", kind)) for n in LAYERS]
            for label, suffix, kind in variants]
    blocks.append(table(["Variant / output"] + [f"{n}L" for n in LAYERS], rows))

    blocks += ["## After: blend modes, CPU requests and groups", "H.264 1080p sources; Display 1080p. "
               "Native and cadence columns retain the individual CSV gate statuses; neither overrides the envelope status."]
    cases = [(f"Blend: {label}", 3, f"h264-1080p30-3l-display-0fx-{token}") for token, label in BLENDS]
    cases += [("CPU requested", n, f"h264-1080p30-{n}l-display-0fx-normal-cpu") for n in (3, 8, 16)]
    cases += [("Groups", n, f"h264-1080p30-{n}l-display-groups") for n in (3, 8, 16)]
    rows = []
    for label, layers, tag in cases:
        row = data.get("after", tag)
        rows.append([label, layers, metric(row, "delivered_fps"), metric(row, "prepare_p95_ms"),
                     clean(row.get("native_status")) if row else "not_established",
                     clean(row.get("cadence_status")) if row else "not_established", status(row)])
    blocks.append(table(["Case", "Layers", "Output FPS", "Prepare p95 (ms)", "Native work", "Cadence", "Envelope"], rows))

    paired, output_count, case_count = paired_table(data)
    blocks += ["## Exact before → after pairs",
               f"All {case_count} before case tags / {output_count} output rows are included. "
               "Pairs match the exact case tag, output kind and measured output ID; an ID-less planned row only records unavailability. "
               "Display and Spout are separate rows. Arrows mean the before value followed by the after value, not an arithmetic difference. "
               "Δmissed, Δopens and Δevictions are each run's observation-period counter deltas. "
               "Decode demand is mean wall time per demand call, not GPU decode utilization.", paired]
    return "\n\n".join(blocks) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv_path", nargs="?", type=Path, default=Path("/tmp/task20-measurements-snapshot.csv"))
    args = parser.parse_args()
    with args.csv_path.open(encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        required = {"phase", "tag", "output_kind", "output_id", "measurement_status", "envelope_status"}
        if not required.issubset(reader.fieldnames or []):
            parser.error("CSV is missing identity/status columns: " + ", ".join(sorted(required - set(reader.fieldnames or []))))
        data = Measurements(list(reader))
    print(render(data), end="")


if __name__ == "__main__":
    main()
