"""Verify a downloaded corresponding-source upload set without trusting the job.

Usage: python -I verify-corresponding-source.py <dir with SHA256SUMS-corresponding-source> <dependency.Dockerfile> <rav1e.Cargo.lock>

Checks every SHA256SUMS line, joins split parts, streams the tar.xz once and
confirms: the uncompressed tar SHA-256 recorded in the manifest, the inner
MANIFEST.json equals the external manifest without its "archive" key, every
download stage mounted by the hosted Dockerfile has a non-empty tree, every
registry package of the committed rav1e lock is vendored, the FFmpeg source and
build-time sources are present, and no .git/.svn metadata remains.
"""
from __future__ import annotations

import hashlib
import json
import lzma
import pathlib
import re
import sys
import tarfile
import tomllib


class Reader:
    """File-like concatenation of split parts that hashes compressed bytes."""

    def __init__(self, paths):
        self.files = [p.open("rb") for p in paths]
        self.sha = hashlib.sha256()

    def read(self, n=-1):
        out = b""
        while self.files and (n < 0 or len(out) < n):
            chunk = self.files[0].read(1 << 20 if n < 0 else n - len(out))
            if not chunk:
                self.files.pop(0).close()
                continue
            self.sha.update(chunk)
            out += chunk
        return out


class HashingLzma:
    def __init__(self, raw):
        self.inner = lzma.LZMAFile(raw)
        self.sha = hashlib.sha256()

    def read(self, n=-1):
        data = self.inner.read(n)
        self.sha.update(data)
        return data


def main() -> None:
    folder, dockerfile, lock = (pathlib.Path(a) for a in sys.argv[1:4])
    sums = folder / "SHA256SUMS-corresponding-source"
    for line in sums.read_text(encoding="utf-8").splitlines():
        digest, name = line.split(None, 1)
        name = name.lstrip("*")
        h = hashlib.sha256()
        with (folder / name).open("rb") as f:
            for chunk in iter(lambda: f.read(1 << 20), b""):
                h.update(chunk)
        assert h.hexdigest() == digest, name
        print("OK", name)
    manifest_path = next(folder.glob("*.manifest.json"))
    external = json.loads(manifest_path.read_text(encoding="utf-8"))
    archive = external["archive"]
    parts = sorted(folder.glob(archive["name"] + ".part*")) or [folder / archive["name"]]
    raw = Reader(parts)
    stream = HashingLzma(raw)
    prefix = archive["name"].removesuffix(".tar.xz") + "/"
    stages = {pathlib.PurePosixPath(m).name.rsplit("_", 1)[0]
              for m in re.findall(r"--mount=src=\.cache/downloads/([^,]+)\.tar\.xz", dockerfile.read_text(encoding="utf-8"))}
    seen_files: dict[str, int] = {}
    vendored = set()
    inner_manifest = None
    vcs = []
    have = set()
    with tarfile.open(fileobj=stream, mode="r|") as tar:
        for m in tar:
            assert m.name.startswith(prefix) or m.name + "/" == prefix, m.name
            rel = m.name[len(prefix):]
            parts_ = rel.split("/")
            if ".git" in parts_ or ".svn" in parts_:
                vcs.append(rel)
            if rel.startswith("dependencies/") and len(parts_) >= 3 and m.isfile():
                seen_files[parts_[1]] = seen_files.get(parts_[1], 0) + 1
            if rel.startswith("build-time/50-rav1e-cargo-vendor/vendor/") and len(parts_) == 4 and m.isdir():
                vendored.add(parts_[3])
            if rel == "MANIFEST.json":
                inner_manifest = json.loads(tar.extractfile(m).read())
            for wanted in ("ffmpeg/ffmpeg-source-29e619e767.tar.gz", "dependencies/25-freetype/subprojects/dlg",
                           "dependencies/50-freetype/subprojects/dlg", "dependencies/50-kvazaar/greatest",
                           "dependencies/50-dav1d/subprojects/checkasm", "build-time/50-rav1e-cargo-vendor/Cargo.lock"):
                if rel == wanted:
                    have.add(wanted)
        while stream.read(1 << 20):
            pass
    while raw.read(1 << 20):
        pass
    failures = []
    if raw.sha.hexdigest() != archive["sha256"]:
        failures.append("archive sha256 mismatch")
    if stream.sha.hexdigest() != archive["uncompressed_tar_sha256"]:
        failures.append("uncompressed tar sha256 mismatch")
    if inner_manifest != {k: v for k, v in external.items() if k != "archive"}:
        failures.append("inner MANIFEST.json differs from external manifest")
    missing = sorted(s for s in stages if not seen_files.get(s))
    if missing:
        failures.append("missing stages: " + ", ".join(missing))
    registry = {p["name"] + "-" + p["version"] for p in tomllib.loads(lock.read_text(encoding="utf-8"))["package"]
                if p.get("source", "").startswith("registry+")}
    if registry - vendored:
        failures.append("unvendored crates: " + ", ".join(sorted(registry - vendored)))
    if vcs:
        failures.append("VCS metadata present: " + ", ".join(vcs[:5]))
    for wanted in ("ffmpeg/ffmpeg-source-29e619e767.tar.gz", "dependencies/25-freetype/subprojects/dlg",
                   "dependencies/50-freetype/subprojects/dlg", "dependencies/50-kvazaar/greatest",
                   "dependencies/50-dav1d/subprojects/checkasm", "build-time/50-rav1e-cargo-vendor/Cargo.lock"):
        if wanted not in have:
            failures.append("missing " + wanted)
    print(json.dumps({"stages_in_dockerfile": len(stages), "stages_present": sum(1 for s in stages if seen_files.get(s)),
                      "manifest_components": len(external["components"]), "registry_crates_in_lock": len(registry),
                      "vendored_crate_dirs": len(vendored), "archive_sha256": raw.sha.hexdigest(),
                      "uncompressed_tar_sha256": stream.sha.hexdigest(), "failures": failures}, indent=1))
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
