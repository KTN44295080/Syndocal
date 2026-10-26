"""Reproduce the bounded FFmpeg dependency notice collection from retained evidence.

No network access. The evidence collectors retain source/crate archives and receipts
under target/qa/task54-release-notices. Missing or altered evidence fails closed; recorded fetch failures remain explicit.
This is an attribution collection, not a binary closure or release clearance proof.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import pathlib

RECIPE = "6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b"
BUILD = "n8.1.3-9-g29e619e767-20260930"
LIMITATIONS = [
    "Publication blocked: source-candidate collection is not exact binary dependency closure.",
    "Pinned chromaprint recipe selects -DFFT_LIB=fftw3 and -lfftw3; pinned FFTW3 COPYRIGHT grants GPL-2.0-or-later. Reconcile this with the DLL LGPL self-description before distribution.",
    "rav1e recipe runs cargo update cc without a version. The final build Cargo.lock, resolved Cargo features, target-filtered graph, link map and build image digest are not retained.",
    "All 577 registry package versions in the two pinned source Cargo.lock files are conservative candidates, including build/dev/other-platform dependencies. Cargo.lock membership is not proof of linkage. Crate archives are verified against the lockfile SHA-256 checksum.",
    "Pinned source candidates include unlinked documentation/tests/build tooling and SDK variants. Source-tree notices are deliberately overinclusive and do not assert that every described license applies to the DLLs.",
    "The original collector has no supported archive endpoint for several pinned repositories; this is a collector limitation, not proof the sources are unavailable. Missing texts are explicit in unresolved_sources. Provenance URLs do not replace notice text.",
    "Several crate archives contain license metadata but no license document. Available exact .cargo_vcs_info.json source revisions are included as supplementary source notices. Others remain explicitly unresolved.",
    "Source tags openssl-3.6.4, v1.4.363, v4.2.0 and proxy-libintl 0.5 were fetched by name; archive SHA-256 identifies fetched bytes, but historical tag resolution at the DLL build is not proved.",
    "Recipe commit is preserved, but recipe use by the binary is not independently attested. Docker tags, compiler/libgcc/libstdc++/libgomp/Rust standard-library versions and complete corresponding-source/patch preservation remain unresolved.",
    "This aggregate includes named full license/notice documents only. Per-file leading source-code notices are retained in source archives but not exhaustively reproduced here; embedded-code notice closure, including SDK headers, remains unresolved.",
    "OpenH264 is source-built in the recipe. Cisco's binary patent licence does not establish patent permission for this independently built binary; this is a separate policy decision.",
]
HEADER = """FFmpeg dependency notices - bounded pinned-source collection
======================================================================
Build: BtbN {build}, win64 LGPL shared 8.1.
Recipe: {recipe}.

PUBLICATION BLOCKED. This file reproduces available full license texts and
notices from pinned source candidates. It does not establish exact binary
closure or release clearance. Source candidates can include unlinked code.
Source distribution: licenses/FFmpeg-DEPENDENCIES.json records source
identities, hashes and unresolved items; it is not a runtime payload resource.

Collection boundaries and unresolved evidence:
{limits}

The following full source license/notice documents retain their wording.
Line endings, tabs, trailing horizontal whitespace and RST underline indentation
are normalized for packaging. Identical complete texts are reproduced once and
referenced by their document number within this file.
Headings, source identities and hashes are editorial provenance; source hashes
refer to the original bytes before whitespace normalization.
"""

def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def decode(data: bytes) -> str:
    try:
        return data.decode("utf-8-sig")
    except UnicodeDecodeError:
        return data.decode("latin-1")

def build(evidence: pathlib.Path) -> tuple[bytes, bytes]:
    sources = []
    for name in ("source-receipts.json", "extra-source-receipts.json"):
        sources.extend(json.loads((evidence / name).read_text(encoding="utf-8")))
    # Linux-only recipes are explicitly excluded by their ffbuild_enabled guards.
    excluded = [s for s in sources if s["name"] in ("libpciaccess", "libdrm")]
    sources = [s for s in sources if s["name"] not in ("libpciaccess", "libdrm")]
    # The recipe's pinned gnulib mirror was fetched successfully as a supplement.
    sources = [s for s in sources if s["name"] != "libiconv-loader"]
    raw_receipt = evidence / "missing-source-receipts.json"
    if raw_receipt.exists():
        resolved = json.loads(raw_receipt.read_text(encoding="utf-8-sig"))
        resolved_names = {s["name"] for s in resolved}
        sources = [s for s in sources if s["name"] not in resolved_names] + resolved
    crates = json.loads((evidence / "crate-receipts.json").read_text(encoding="utf-8"))
    intro = HEADER.format(build=BUILD, recipe=RECIPE, limits="\n".join("- " + x for x in LIMITATIONS))
    sections = [intro]
    shared_texts = {}
    manifest = {"schema_version": 1, "build": BUILD, "recipe_commit": RECIPE,
                "closure_status": "incomplete-publication-blocked", "limitations": LIMITATIONS,
                "sources": [], "excluded_linux_only_recipes": excluded, "registry_crate_candidates": [], "unresolved_sources": [],
                "crate_archives_without_license_documents": [], "sources_without_license_documents": [], "notice_count": 0}
    for kind, items in (("source", sources), ("crate", crates)):
        for source in sorted(items, key=lambda x: (x["name"], x.get("version", ""))):
            entry = dict(source)
            entry.pop("files", None)
            entry["notices"] = []
            name = source["name"] + ("-" + source["version"] if kind == "crate" else "")
            target = "sources" if kind == "source" else "registry_crate_candidates"
            manifest[target].append(entry)
            if "error" in source:
                manifest["unresolved_sources"].append(entry.copy())
                continue
            if not source.get("raw_only"):
                archive = evidence / ("crates" if kind == "crate" else "sources") / (name + (".crate" if kind == "crate" else "-" + source["ref"] + ".tar.gz"))
                raw = archive.read_bytes()
                if sha(raw) != source["archive_sha256"] or len(raw) != source["archive_bytes"]:
                    raise ValueError("Archive identity drift: " + str(archive))
            else:
                entry["raw_source_documents"] = source["files"]
            files = source["files"]
            root = evidence / ("extracted-crates" if kind == "crate" else "extracted") / name
            identity = (source["repo"] + " @ " + source["ref"]) if kind == "source" else (source["archive_url"] + " (Cargo.lock SHA-256 verified)")
            seen = set()
            def append(path: str, blob: bytes, form: str) -> None:
                digest = sha(blob)
                if digest in seen:
                    next(n for n in entry["notices"] if n["sha256"] == digest)["paths"].append(path)
                    return
                seen.add(digest)
                entry["notices"].append({"paths": [path], "sha256": digest, "bytes": len(blob), "form": form})
                heading = "\n" + "=" * 78 + "\n" + name + " / " + path + "\nSource: " + identity + "\nSHA-256 (source bytes): " + digest + "\n" + "-" * 78 + "\n"
                rendered = "\n".join(line.expandtabs(8).rstrip(" \r") for line in decode(blob).split("\n")).rstrip()
                # Upstream RST underlines can look like Git merge markers.
                rendered = "\n".join(" " + line if line == "=======" else line for line in rendered.split("\n"))
                rendered_digest = sha(rendered.encode("utf-8"))
                if rendered_digest in shared_texts:
                    document_id = shared_texts[rendered_digest]
                    sections.append(heading + "Identical complete text is reproduced above as document " + str(document_id) + ".\n")
                else:
                    document_id = len(shared_texts) + 1
                    shared_texts[rendered_digest] = document_id
                    sections.append(heading + "Document " + str(document_id) + " (complete text):\n" + rendered + "\n")
                entry["notices"][-1]["document_id"] = document_id
            for f in sorted(files, key=lambda x: x["path"]):
                if not f["notice"]:
                    continue
                doc_path = (evidence / f["local_path"]) if source.get("raw_only") else (root / f["path"])
                if not doc_path.resolve().is_relative_to(evidence.resolve()):
                    raise ValueError("Notice path escapes evidence root: " + str(doc_path))
                blob = doc_path.read_bytes()
                if sha(blob) != f["sha256"] or len(blob) != f["bytes"]:
                    raise ValueError("Notice identity drift: " + name + "/" + f["path"])
                append(f["path"], blob, "full-source-document")
            if kind == "crate" and not seen:
                manifest["crate_archives_without_license_documents"].append({"name": name, "declared_license": source.get("declared_license"), "supplement": "See matching crate-upstream source candidate when available; unresolved otherwise."})
            if kind == "source" and not seen:
                manifest["sources_without_license_documents"].append(name)
            manifest["notice_count"] += len(entry["notices"])
    text = "".join(sections).encode("utf-8")
    manifest["unique_complete_text_count"] = len(shared_texts)
    manifest["output"] = {"path": "licenses/FFmpeg-DEPENDENCIES.txt", "bytes": len(text), "sha256": sha(text)}
    metadata = (json.dumps(manifest, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    return text, metadata

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=pathlib.Path, default=pathlib.Path("target/qa/task54-release-notices"))
    parser.add_argument("--check", action="store_true", help="Verify existing outputs without writing")
    args = parser.parse_args()
    text, metadata = build(args.evidence_root)
    for name, data in (("FFmpeg-DEPENDENCIES.txt", text), ("FFmpeg-DEPENDENCIES.json", metadata)):
        dest = pathlib.Path("licenses") / name
        if args.check:
            if dest.read_bytes() != data:
                raise SystemExit("Generated output drift: " + str(dest))
        else:
            dest.write_bytes(data)
    info = json.loads(metadata)
    print(json.dumps({"check": args.check, "sources": len(info["sources"]), "crates": len(info["registry_crate_candidates"]), "notices": info["notice_count"], "unresolved_sources": len(info["unresolved_sources"]), "output": info["output"]}))

if __name__ == "__main__":
    main()
