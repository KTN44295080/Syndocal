"""Preserve executable permission when the Mac removal helper is sent from Windows."""
import hashlib
import json
import pathlib
import sys
import zipfile

root = pathlib.Path(__file__).resolve().parents[2]
version = json.loads((root / "app/package.json").read_text(encoding="utf-8"))["version"]
destination = pathlib.Path(sys.argv[1]).resolve()
if destination.name != f"Syndocal_{version}_Mac-removal.zip":
    raise ValueError("Unexpected versioned removal archive name")
source = root / "tools/distribution/Remove-Syndocal.command"
if source.is_symlink():
    raise ValueError("Redirected removal helper rejected")
script = source.read_bytes()
if b"\r" in script or not script.startswith(b"#!/"):
    raise ValueError("Expected an LF shell script with a shebang")
with zipfile.ZipFile(destination, "x", zipfile.ZIP_DEFLATED) as archive:
    entry = zipfile.ZipInfo("Remove-Syndocal.command")
    entry.create_system = 3
    entry.external_attr = 0o100755 << 16
    archive.writestr(entry, script)
with zipfile.ZipFile(destination) as archive:
    entry = archive.getinfo("Remove-Syndocal.command")
    assert entry.create_system == 3 and entry.external_attr >> 16 == 0o100755
    assert archive.read(entry) == script
digest = hashlib.sha256(destination.read_bytes()).hexdigest()
destination.with_suffix(".zip.sha256").write_text(f"{digest}  {destination.name}\n", encoding="utf-8")
print(json.dumps({"passed": True, "path": str(destination), "sha256": digest,
                  "script_sha256": hashlib.sha256(script).hexdigest(), "unix_mode": "100755"}))
