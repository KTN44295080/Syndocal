#!/usr/bin/env python3
"""Export selected native readout observations and reviewed screenshots only."""
import argparse
import datetime as dt
import hashlib
import json
import math
import re
import struct
import zlib
from pathlib import Path

TAGS = [
    "h264-1080p30-3l-display-0fx-normal",
    "h264-1080p30-3l-spout-0fx-normal",
    "h264-1080p30-3l-display-3fx-normal",
]
EXE = "a1e0ae6519bb3558d2b473fc9d68b9a2487ab6ac55ec8e5bbacf1bb375ef869d"
IMAGE_NAMES = ["after-" + tag + "-" + suffix + ".png"
               for tag in TAGS for suffix in ("native-display", "en", "ja")]


def safe_text(value):
    assert isinstance(value, str) and len(value) < 2000
    assert not re.search(r"[a-z]:[\\/]|(?:https?|file)://|[\\/]Users[\\/]|localStorage|recentProjects", value, re.I)
    return value


def rect(value):
    result = {k: value[k] for k in ("x", "y", "width", "height")}
    assert all(isinstance(v, (int, float)) and math.isfinite(v) and v >= 0 for v in result.values())
    assert result["width"] > 0 and result["height"] > 0
    return result


def png(path):
    assert path.is_file() and not path.is_symlink()
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    offset, chunks, width, height = 8, [], None, None
    while offset < len(data):
        assert offset + 12 <= len(data), "Truncated PNG chunk"
        length = struct.unpack(">I", data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]
        assert re.fullmatch(b"[A-Za-z]{4}", kind), "Invalid PNG chunk name"
        end = offset + 12 + length
        assert end <= len(data), "Truncated PNG payload"
        payload = data[offset + 8:end - 4]
        expected_crc = struct.unpack(">I", data[end - 4:end])[0]
        assert zlib.crc32(kind + payload) & 0xffffffff == expected_crc, "PNG CRC mismatch"
        name = kind.decode("ascii")
        assert name not in ("tEXt", "zTXt", "iTXt", "eXIf"), "PNG text/EXIF metadata requires explicit review"
        if not chunks:
            assert name == "IHDR" and length == 13, "PNG must start with IHDR"
            width, height = struct.unpack(">II", payload[:8])
            assert width > 0 and height > 0
        else:
            assert name != "IHDR", "Duplicate PNG header"
        chunks.append(name)
        offset = end
        if name == "IEND":
            assert length == 0 and "IDAT" in chunks and offset == len(data), "Invalid PNG end or trailing bytes"
            break
    assert chunks and chunks[-1] == "IEND", "PNG end missing"
    return {"name": path.name, "bytes": len(data), "width": width, "height": height,
            "sha256": hashlib.sha256(data).hexdigest(),
            "pngChunks": list(dict.fromkeys(chunks))}, data


def reviewed_images(raw_dir, manifest_path):
    assert manifest_path.is_file() and not manifest_path.is_symlink()
    manifest_bytes = manifest_path.read_bytes()
    entries = json.loads(manifest_bytes.decode("utf-8-sig"))
    assert isinstance(entries, list) and len(entries) == len(IMAGE_NAMES), "Manifest must identify exactly nine PNGs"
    reviewed = {}
    for entry in entries:
        assert isinstance(entry, dict) and set(entry) == {"name", "sha256"}
        name, digest = entry["name"], entry["sha256"]
        assert isinstance(name, str) and name in IMAGE_NAMES and name not in reviewed
        assert isinstance(digest, str) and re.fullmatch(r"[0-9a-fA-F]{64}", digest)
        reviewed[name] = digest.lower()
    assert set(reviewed) == set(IMAGE_NAMES)
    metadata, contents = {}, {}
    for name in IMAGE_NAMES:
        image, data = png(raw_dir / name)
        assert image["sha256"] == reviewed[name], "PNG bytes differ from reviewed manifest"
        metadata[name], contents[name] = image, data
    return metadata, contents, hashlib.sha256(manifest_bytes).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_dir", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--reviewed-manifest", required=True, type=Path)
    args = parser.parse_args()
    assert args.destination.is_dir()
    image_metadata, image_contents, manifest_sha256 = reviewed_images(args.raw_dir, args.reviewed_manifest)
    records = []
    images = args.destination / "images"
    for tag in TAGS:
        stem = "after-" + tag
        raw = json.loads((args.raw_dir / (stem + ".json")).read_text(encoding="utf-8-sig"))
        assert raw["ok"] and not raw.get("restoreError") and not raw.get("stopError")
        assert raw["executableSha256"] == EXE and raw["spec"]["tag"] == tag
        capture = raw["nativeDisplayCapture"]
        assert all(capture[k] is True for k in ("primaryOnly", "fullyVisibleClient", "afterMeasurement"))
        native = image_metadata[stem + "-native-display.png"]
        assert (native["width"], native["height"]) == (1920, 1080)
        assert native["sha256"] == capture["sha256"]
        nonblack = capture["nonBlackSampleFraction"]
        assert type(nonblack) in (int, float) and math.isfinite(nonblack) and .01 < nonblack <= 1
        locales = []
        assert [p["locale"] for p in raw["localeProof"]] == ["en", "ja"]
        for proof in raw["localeProof"]:
            assert isinstance(proof["text"], str) and proof["text"].strip()
            bounds = rect(proof["bounds"])
            viewport = {k: proof["viewport"][k] for k in ("width", "height")}
            assert viewport == {"width": 1280, "height": 752}
            assert bounds["x"] + bounds["width"] <= viewport["width"] + 1
            assert bounds["y"] + bounds["height"] <= viewport["height"] + 1
            leaves = []
            assert isinstance(proof["leaves"], list) and proof["leaves"]
            for leaf in proof["leaves"]:
                assert isinstance(leaf["text"], str) and leaf["text"].strip()
                box = rect(leaf["bounds"])
                assert leaf["fontPx"] == 14
                assert box["x"] >= bounds["x"] - 1 and box["y"] >= bounds["y"] - 1
                assert box["x"] + box["width"] <= bounds["x"] + bounds["width"] + 1
                assert box["y"] + box["height"] <= bounds["y"] + bounds["height"] + 1
                leaves.append({"text": safe_text(leaf["text"]), "fontPx": 14,
                               "color": safe_text(leaf["color"]), "bounds": box})
            screenshot = image_metadata[stem + "-" + proof["locale"] + ".png"]
            assert (screenshot["width"], screenshot["height"]) == (2560, 1504)
            if raw["spec"]["fx"] == 3:
                expected = "Frames missed" if proof["locale"] == "en" else "フレーム欠落"
                assert expected in proof["text"]
            locales.append({"locale": proof["locale"], "text": safe_text(proof["text"]),
                            "viewport": viewport, "bounds": bounds, "leaves": leaves,
                            "allLeavesWithinReadout": True, "screenshot": screenshot})
        records.append({"tag": tag, "executableSha256": EXE, "locales": locales,
                        "nativeDisplay": {**native, "primaryOnly": True, "fullyVisibleClient": True,
                                          "afterMeasurement": True,
                                          "nonBlackSampleFraction": capture["nonBlackSampleFraction"]}})
    result = {"schemaVersion": 1, "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
              "review": "Main-checkout agent visually inspected all nine images. These are post-observation UI/native-image checks, not timed FPS samples or physical scanout proof.",
              "reviewedImageManifestSha256": manifest_sha256,
              "cases": records}
    encoded = json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    images.mkdir(exist_ok=True)
    for name, data in image_contents.items():
        # Copy the exact preflighted bytes, never reopen a mutable source path.
        (images / name).write_bytes(data)
    (args.destination / "ui-validation.json").write_text(encoded, encoding="utf-8", newline="\n")
    print(json.dumps({"cases": len(records), "images": len(records) * 3, "fontPx": 14, "allLeavesWithinReadout": True}))


if __name__ == "__main__":
    main()
