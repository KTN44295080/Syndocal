"""Write MANIFEST.json for the corresponding-source tree and cross-check the hosted build log.

Called by pack-corresponding-source.sh. Fails closed when a stage mounted by the
build Dockerfile is missing, or when a commit printed by run 40366564270's
download step does not match the commit resolved now.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import stat

NOTICE = re.compile(r"(?i)^(copying|copyright|licen[sc]e|notice|patents|unlicense)([.\-_]|$)")
HEX40 = re.compile(r"^[0-9a-f]{40}$")
IMAGES = {
    "base": "ghcr.io/btbn/ffmpeg-builds/base@sha256:91eaaaf32f3f98464e99faf1062fab8ab99a23d0b499c72b51a02ec9137b06a0",
    "target_base_win64": "ghcr.io/btbn/ffmpeg-builds/base-win64@sha256:8a3d189e5b615c89e1a9fe2be705fa17173d8f6ea0a955b25c7c28041cde36c7",
    "buildkit": "moby/buildkit:buildx-stable-1 (tag only; digest not printed by run 40366564270, unresolved)",
    "dependency_image_id": "sha256:22662e2994102ad02d2b6e21eb331024310eb336899910fb454bd4260287210a (local image, not pushed)",
    "runner": "GitHub-hosted ubuntu-24.04, runner image ubuntu24/20261012.334",
}
NOT_PRESERVED = [
    "GCC 16.2.0 (crosstool-NG 1.29.0.7_b1a94f6) runtime libraries statically linked by the toolchain (libgcc, libstdc++, libgomp; GCC Runtime Library Exception) and binutils: identified by version and by the digest-pinned base-win64 image only; their sources are not included.",
    "Rust standard library of rustc 1.99.1 (58cf8f9c3) linked into rav1e/librsvg: identified by version only; source not included.",
    "The two digest-pinned base images are not reconstructed; BtbN's images/ recipes are included in recipe/ but the images were not proven to be built from that revision.",
    "Dependency stages downloaded by download.sh but not mounted by the generated Dockerfile (listed under downloaded_not_built) are not included because they are not built.",
]


def sha_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def tree_digest(root: pathlib.Path) -> dict:
    """sha256 over sorted '<type> <exec> <sha256|link target> <relpath>' lines."""
    lines, files, size = [], 0, 0
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames + [d for d in dirnames if os.path.islink(os.path.join(dirpath, d))]):
            p = pathlib.Path(dirpath) / name
            rel = p.relative_to(root).as_posix()
            st = os.lstat(p)
            if stat.S_ISLNK(st.st_mode):
                lines.append("l - " + os.readlink(p) + " " + rel)
            elif stat.S_ISREG(st.st_mode):
                files += 1
                size += st.st_size
                lines.append("f " + ("x" if st.st_mode & 0o111 else "-") + " " + sha_file(p) + " " + rel)
    lines.sort(key=lambda s: s.split(" ", 3)[3])
    return {"algorithm": "sha256 of sorted '<f|l> <x|-> <sha256|target> <path>' lines",
            "sha256": hashlib.sha256("\n".join(lines).encode("utf-8")).hexdigest(),
            "files": files, "bytes": size}


def licence_files(root: pathlib.Path) -> list[dict]:
    out = []
    if not root.is_dir():
        return out
    for p in sorted(root.iterdir()):
        if p.is_file() and not p.is_symlink() and NOTICE.search(p.name):
            out.append({"path": p.name, "sha256": sha_file(p), "bytes": p.stat().st_size})
    return out


def script_vars(path: pathlib.Path) -> dict:
    return dict(re.findall(r'^SCRIPT_([A-Z0-9_]+)="([^"]*)"', path.read_text(encoding="utf-8"), re.M))


def log_segments(log: pathlib.Path) -> dict:
    segs, cur = {}, None
    tgt = re.compile(r"^\+ TGT=/dldir/(\S+)_([0-9a-f]{64})\.tar\.xz$")
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        m = tgt.match(line)
        if m:
            cur = segs.setdefault(m.group(1) + "_" + m.group(2), {"head": [], "submodules": [], "svn": [], "downloads": []})
            continue
        if line.startswith("+ ./generate.sh") or line.startswith("+ bash ./build.sh") or line.startswith("#1 "):
            cur = None
        if cur is None:
            continue
        if (m := re.search(r"HEAD is now at ([0-9a-f]{7,40}) ", line)):
            cur["head"].append(m.group(1))
        if (m := re.search(r"Submodule path '([^']+)': checked out '([0-9a-f]{40})'", line)):
            cur["submodules"].append({"path": m.group(1), "commit": m.group(2)})
        if (m := re.search(r"Checked out revision (\d+)\.", line)):
            cur["svn"].append(m.group(1))
        if (m := re.search(r"Downloading (\S+) source from (\S+)", line)):
            cur["downloads"].append({"name": m.group(1), "url": m.group(2)})
        if (m := re.search(r"saved \[(\d+)/(\d+)\]", line)) and "opus_data-" in line:
            cur["downloads"].append({"name": re.search(r"'([^']+)'", line).group(1), "bytes": int(m.group(1))})
    return segs


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tree", type=pathlib.Path, required=True)
    ap.add_argument("--build-log", type=pathlib.Path, required=True)
    ap.add_argument("--licenses", type=pathlib.Path, required=True)
    ap.add_argument("--output", type=pathlib.Path, required=True)
    a = ap.parse_args()
    tree = a.tree
    recipe = next((tree / "recipe").iterdir())
    licences = json.loads(a.licenses.read_text(encoding="utf-8"))
    caches = [l.split("\t") for l in (tree / "receipts/download-caches.tsv").read_text(encoding="utf-8").splitlines() if l]
    idents = [l.split("\t") for l in (tree / "receipts/git-identities.tsv").read_text(encoding="utf-8").splitlines() if l]
    segs = log_segments(a.build_log)
    failures, components = [], []
    mounted = set()
    for stage, script, cache, cache_bytes, cache_sha in caches:
        mounted.add(cache.removesuffix(".tar.xz"))
        root = tree / "dependencies" / stage
        if not root.is_dir() or not any(root.iterdir()):
            failures.append(stage + ": missing or empty source tree")
            continue
        if stage not in licences:
            failures.append(stage + ": no licence summary")
        vars_ = script_vars(recipe / script)
        own = [{"path": p.removeprefix("dependencies/" + stage).lstrip("/") or ".", "commit": c, "remote": r}
               for p, c, r in idents if p == "dependencies/" + stage or p.startswith("dependencies/" + stage + "/")]
        resolved = {i["commit"] for i in own}
        pinned = {v for k, v in vars_.items() if k.startswith("COMMIT") and HEX40.match(v)}
        refs = []
        for k, v in sorted(vars_.items()):
            if not k.startswith(("COMMIT", "REV")):
                continue
            suffix = k.removeprefix("COMMIT").removeprefix("REV")
            repo = vars_.get("REPO" + suffix) or vars_.get("REPO")
            entry = {"var": "SCRIPT_" + k, "repo": repo, "requested": v}
            if k.startswith("REV"):
                entry["kind"] = "svn-revision (fixed)"
            elif HEX40.match(v):
                entry["kind"] = "commit (fixed)"
            else:
                entry["kind"] = "named ref (moving)"
                matches = sorted({i["commit"] for i in own if i["remote"] == repo})
                entry["resolved_commit"] = matches[0] if len(matches) == 1 else None
                if entry["resolved_commit"] is None:
                    failures.append(stage + ": could not resolve " + v)
            refs.append(entry)
        seg = segs.get(cache.removesuffix(".tar.xz"))
        evidence = {"segment_found": seg is not None}
        if seg is None:
            failures.append(stage + ": no download segment in hosted build log")
        else:
            evidence.update(seg)
            candidates = resolved | pinned
            bad = [h for h in seg["head"] if not any(c.startswith(h) for c in candidates)]
            bad += [s["commit"] for s in seg["submodules"] if s["commit"] not in resolved]
            if vars_.get("REV") and seg["svn"] and seg["svn"] != [vars_["REV"]]:
                bad += seg["svn"]
            evidence["mismatches"] = bad
            if bad:
                failures.append(stage + ": build-log commit mismatch " + ", ".join(bad))
            moving = [r for r in refs if r["kind"].startswith("named")]
            evidence["status"] = ("matched; moving refs proved only by abbreviated build-log commit prefix" if moving
                                  else "matched")
        components.append({
            "stage": stage, "script": script, "licence_summary": licences.get(stage),
            "download_cache": {"name": cache, "bytes": int(cache_bytes), "sha256": cache_sha,
                               "note": "original cache bytes are not deterministic; the extracted tree is preserved"},
            "refs": refs, "resolved_vcs": own, "build_log_evidence": evidence,
            "licence_files": {i["path"]: licence_files(root / i["path"]) for i in own} or {".": licence_files(root)},
            "tree": tree_digest(root),
        })
    downloaded_not_built = sorted(k.rsplit("_", 1)[0] for k in segs if k not in mounted)
    vendor = tree / "build-time/50-rav1e-cargo-vendor"
    crates = sorted(p.name for p in (vendor / "vendor").iterdir() if p.is_dir())
    for crate in ("cc-1.10.0", "find-msvc-tools-0.1.15", "shlex-2.0.1"):
        if crate not in crates:
            failures.append("rav1e vendor lacks " + crate)
    build_time = [
        {"stage": "25-freetype", "path": "dependencies/25-freetype/subprojects/dlg", "commit": "395ccad2c1e0daae535c4d20bb0a3f2424648e17",
         "evidence": "build log: Submodule path 'subprojects/dlg': checked out (freetype autogen.sh)"},
        {"stage": "50-freetype", "path": "dependencies/50-freetype/subprojects/dlg", "commit": "395ccad2c1e0daae535c4d20bb0a3f2424648e17",
         "evidence": "build log: Submodule path 'subprojects/dlg': checked out (freetype autogen.sh)"},
        {"stage": "50-kvazaar", "path": "dependencies/50-kvazaar/greatest", "commit": "60e25ce7658ced49f03b52ac14c553cf6ea33de4",
         "evidence": "build log: Submodule path 'greatest': checked out (test library, not linked)"},
        {"stage": "50-dav1d", "path": "dependencies/50-dav1d/subprojects/checkasm", "requested": "v1.2.0 (meson wrap-git tag)",
         "commit": next((c for p, c, r in idents if p == "dependencies/50-dav1d/subprojects/checkasm"), None),
         "evidence": "build log shows only 'Project version: 1.2.0'; the commit used at build time is unresolved (test-only subproject, not linked)"},
        {"stage": "50-rav1e", "path": "build-time/50-rav1e-cargo-vendor", "lock_sha256": sha_file(vendor / "Cargo.lock"),
         "cargo": (vendor / "cargo-version.txt").read_text(encoding="utf-8").strip(), "crates": len(crates),
         "evidence": "cargo cinstall --locked fetched these crates during the build; vendored from the identical committed lock (checksums enforced by cargo)"},
    ]
    manifest = {
        "schema_version": 1,
        "subject": {
            "release_tag": "ffmpeg-sdk-lgpl-n8.1.3-r40366564270",
            "sdk_zip": "ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip",
            "sdk_zip_sha256": "81587fa070a9fa2da3d8ff3e63beb9654729782bc8e9899ee951baf5fc5d9f1a",
            "build_run": "https://github.com/KTN44295080/Syndocal/actions/runs/40366564270",
            "build_commit": "7d0af757f73231fc0c6006d6c21c72c14cb4a31a",
            "variant": "win64 lgpl-shared 8.1",
        },
        "ffmpeg_source": {"path": "ffmpeg/ffmpeg-source-29e619e767.tar.gz", "commit": "29e619e767cde9045a75c29bc9a8278ae7b3a98b",
                          "sha256": sha_file(tree / "ffmpeg/ffmpeg-source-29e619e767.tar.gz"), "licence_summary": "LGPL-3.0-or-later as configured (--enable-version3, no --enable-gpl/--enable-nonfree)"},
        "recipe": {"path": "recipe/" + recipe.name, "upstream": "https://github.com/BtbN/FFmpeg-Builds",
                   "commit": "6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b",
                   "patch_sha256": sha_file(tree / "syndocal" / ("build-controls-7d0af757f73231fc0c6006d6c21c72c14cb4a31a") / "tools/ffmpeg-sdk/0001-lgpl-without-chromaprint-fftw.patch"),
                   "rav1e_lock_sha256": sha_file(recipe / "util/rav1e.Cargo.lock"),
                   "tree": tree_digest(recipe), "licence_summary": "MIT (BtbN FFmpeg-Builds LICENSE)"},
        "dockerfile": {"hosted_sha256": sha_file(tree / "receipts/hosted-dependency.Dockerfile"),
                       "regenerated_sha256": sha_file(tree / "receipts/regenerated-dependency.Dockerfile"),
                       "identical": sha_file(tree / "receipts/hosted-dependency.Dockerfile") == sha_file(tree / "receipts/regenerated-dependency.Dockerfile")},
        "images": IMAGES,
        "toolchain": (tree / "receipts/hosted-toolchain.txt").read_text(encoding="utf-8").strip().splitlines(),
        "component_count": len(components),
        "components": components,
        "build_time_sources": build_time,
        "downloaded_not_built": downloaded_not_built,
        "not_preserved": NOT_PRESERVED,
        "reproducibility": "Inputs are fixed or verified against run 40366564270; byte-identical rebuilds of the SDK have not been demonstrated.",
        "failures": failures,
    }
    if not manifest["dockerfile"]["identical"]:
        failures.append("regenerated Dockerfile differs from hosted Dockerfile")
    a.output.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"components": len(components), "downloaded_not_built": downloaded_not_built,
                      "rav1e_crates": len(crates), "failures": failures}, indent=1))
    if failures:
        raise SystemExit("corresponding-source manifest failures: " + "; ".join(failures))


if __name__ == "__main__":
    main()
