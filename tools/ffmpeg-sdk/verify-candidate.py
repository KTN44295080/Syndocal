"""Read-only SDK acceptance; run each DLL set in a separate Python process."""

import argparse
import ctypes as c
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def probe(bin_dir):
    directory = os.add_dll_directory(str(bin_dir))
    codec = c.CDLL(str(bin_dir / "avcodec-62.dll"))
    util = c.CDLL(str(bin_dir / "avutil-60.dll"))
    fmt = c.CDLL(str(bin_dir / "avformat-62.dll"))

    class Codec(c.Structure):
        _fields_ = [("name", c.c_char_p), ("long_name", c.c_char_p),
                    ("type", c.c_int), ("id", c.c_int)]

    class Hardware(c.Structure):
        _fields_ = [("pix_fmt", c.c_int), ("methods", c.c_int),
                    ("device_type", c.c_int)]

    class OutputFormat(c.Structure):
        _fields_ = [("name", c.c_char_p)]

    codec.av_codec_iterate.argtypes = [c.POINTER(c.c_void_p)]
    codec.av_codec_iterate.restype = c.POINTER(Codec)
    codec.av_codec_is_decoder.argtypes = [c.POINTER(Codec)]
    codec.avcodec_get_hw_config.argtypes = [c.POINTER(Codec), c.c_int]
    codec.avcodec_get_hw_config.restype = c.POINTER(Hardware)
    util.av_hwdevice_get_type_name.argtypes = [c.c_int]
    util.av_hwdevice_get_type_name.restype = c.c_char_p
    state, decoders, hardware = c.c_void_p(), [], {}
    while True:
        entry = codec.av_codec_iterate(c.byref(state))
        if not entry:
            break
        if not codec.av_codec_is_decoder(entry):
            continue
        name = entry.contents.name.decode()
        decoders.append(name)
        configs = []
        for index in range(100):
            item = codec.avcodec_get_hw_config(entry, index)
            if not item:
                break
            kind = util.av_hwdevice_get_type_name(item.contents.device_type)
            configs.append(dict(device=kind.decode() if kind else None,
                                methods=item.contents.methods,
                                pix_fmt=item.contents.pix_fmt))
        if configs:
            hardware[name] = configs
    fmt.av_muxer_iterate.argtypes = [c.POINTER(c.c_void_p)]
    fmt.av_muxer_iterate.restype = c.POINTER(OutputFormat)
    state, muxers = c.c_void_p(), []
    while True:
        entry = fmt.av_muxer_iterate(c.byref(state))
        if not entry:
            break
        muxers.append(entry.contents.name.decode())

    def string(lib, symbol):
        function = getattr(lib, symbol)
        function.restype = c.c_char_p
        return function().decode()

    result = dict(version=string(util, "av_version_info"),
                  configure=string(codec, "avcodec_configuration"),
                  avformat_configure=string(fmt, "avformat_configuration"),
                  license=string(util, "avutil_license"),
                  avformat_license=string(fmt, "avformat_license"),
                  decoders=sorted(decoders), hardware=hardware, muxers=sorted(muxers))
    directory.close()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe-bin", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--old", type=Path)
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--fixtures", type=Path)
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    if args.probe_bin:
        print(json.dumps(probe(args.probe_bin.resolve()), indent=2))
        return
    assert all([args.candidate, args.old, args.inventory, args.fixtures, args.evidence])
    evidence = args.evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
    observed = {}
    for label, sdk in [("old", args.old.resolve()), ("candidate", args.candidate.resolve())]:
        records = []
        for path in sorted((sdk / "bin").glob("*.dll")):
            data = path.read_bytes()
            offset = struct.unpack_from("<I", data, 60)[0]
            record = dict(filename=path.name, byte_size=len(data), sha256=digest(path),
                          pe_identity=dict(signature=data[offset:offset + 4].decode("ascii"),
                                           machine=struct.unpack_from("<H", data, offset + 4)[0],
                                           characteristics=struct.unpack_from("<H", data, offset + 22)[0],
                                           optional_magic=struct.unpack_from("<H", data, offset + 24)[0]),
                          forbidden_ascii_matches=len(re.findall(b"chromaprint|fftw", data.lower())),
                          forbidden_utf16_matches=sum(data.lower().count(x.encode("utf-16-le"))
                                                      for x in ["chromaprint", "fftw"]))
            records.append(record)
        assert len(records) == 7, (label, "seven DLLs required")
        expected = {x["filename"]: x for x in inventory["runtime_dlls"]}
        assert {x["filename"] for x in records} == set(expected)
        if label == "old":
            for record in records:
                for key in ["byte_size", "sha256", "pe_identity"]:
                    assert record[key] == expected[record["filename"]][key], (record, key)
        else:
            for record in records:
                assert record["pe_identity"]["signature"] == "PE\0\0"
                assert record["pe_identity"]["machine"] == 34404
                assert record["pe_identity"]["optional_magic"] == 523
                assert record["forbidden_ascii_matches"] == record["forbidden_utf16_matches"] == 0
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()),
                                 "--probe-bin", str(sdk / "bin")], capture_output=True, check=True)
        observed[label] = json.loads(result.stdout)
        observed[label]["runtime_dlls"] = records
        env = dict(os.environ)
        env["PATH"] = str(sdk / "bin") + os.pathsep + env["PATH"]
        for option in ["buildconf", "L", "muxers", "decoders", "hwaccels"]:
            result = subprocess.run([str(sdk / "bin/ffmpeg.exe"), "-hide_banner", "-" + option],
                                    env=env, capture_output=True, check=True)
            (evidence / f"{label}-{option}.txt").write_bytes(result.stdout + result.stderr)
        (evidence / f"{label}-probe.json").write_text(json.dumps(observed[label], indent=2), encoding="utf-8")
    old, new = observed["old"], observed["candidate"]
    assert old["configure"] == inventory["ffmpeg_build"]["configure"]
    assert new["configure"] == new["avformat_configure"]
    assert not re.search(r"--enable-(chromaprint|gpl|nonfree)\b", new["configure"])
    assert new["license"] == new["avformat_license"] == "LGPL version 3 or later"
    assert "chromaprint" not in new["muxers"]
    removed = sorted(set(old["decoders"]) - set(new["decoders"]))
    added = sorted(set(new["decoders"]) - set(old["decoders"]))
    assert not removed and not added, (removed, added)
    assert old["hardware"] == new["hardware"], "per-decoder hardware configurations differ"
    for required in ["h264", "hevc", "hap", "dxv", "prores", "vp9", "av1", "libdav1d"]:
        assert required in new["decoders"], required
    hw = [(evidence / f"{label}-hwaccels.txt").read_text(encoding="utf-8").split("Hardware acceleration methods:")[-1].split()
          for label in ["old", "candidate"]]
    assert hw[0] == hw[1], hw
    assert all(x in hw[1] for x in ["d3d11va", "dxva2", "d3d12va", "vulkan"])
    differences = dict(decoder_count=len(new["decoders"]), decoders_removed=removed, decoders_added=added,
                       hwaccels=hw[1], per_decoder_hardware_equal=True,
                       configure_removed=sorted(set(old["configure"].split()) - set(new["configure"].split())),
                       configure_added=sorted(set(new["configure"].split()) - set(old["configure"].split())),
                       muxers_removed=sorted(set(old["muxers"]) - set(new["muxers"])),
                       muxers_added=sorted(set(new["muxers"]) - set(old["muxers"])))
    assert differences["configure_removed"] == ["--enable-chromaprint"]
    assert not differences["configure_added"]
    assert differences["muxers_removed"] == ["chromaprint"] and not differences["muxers_added"]
    (evidence / "capability-diff.json").write_text(json.dumps(differences, indent=2), encoding="utf-8")
    matrix = json.loads((args.fixtures / "results.json").read_text(encoding="utf-8"))
    assert len(matrix) == 11
    results = []
    for case in matrix:
        fixtures = [p for p in args.fixtures.iterdir() if p.stem == case["case"] and p.suffix in [".mov", ".mp4", ".webm"]]
        assert len(fixtures) == 1
        fixture = fixtures[0].resolve()
        assert digest(fixture) == case["fixture_sha256"]
        frames = []
        for label, sdk in [("old", args.old.resolve()), ("candidate", args.candidate.resolve())]:
            output = evidence / f'{case["case"]}.{label}.framecrc'
            cmd = [str(sdk / "bin/ffmpeg.exe"), "-hide_banner", "-v", "error", "-threads", "1",
                   "-i", str(fixture), "-map", "0:v:0", "-f", "framecrc", "-y", str(output)]
            result = subprocess.run(cmd, capture_output=True)
            assert result.returncode == 0, (cmd, result.stderr.decode(errors="replace"))
            frames.append([x for x in output.read_text().splitlines() if x and not x.startswith("#")])
        assert len(frames[0]) == case["frames"] and frames[0] == frames[1], case["case"]
        results.append(dict(case=case["case"], fixture_sha256=digest(fixture), old_exit=0,
                            candidate_exit=0, frames=len(frames[0]), identical_frame_records=True))
    (evidence / "framecrc-results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(json.dumps(dict(capabilities=differences, framecrc_cases=len(results), status="PASS"), indent=2))


if __name__ == "__main__":
    main()
