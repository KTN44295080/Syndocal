# RDM software revalidation — 2026-09-12

This checkpoint records bounded current-main parser, codec, and serial-framing
evidence for `COV-RDM-001`. It does not claim physical RDM/TOD operation or
timeout/cancellation acceptance against devices.

## Source and scope

- Source base: `6b88ec8a353f6233f6f6dd606ecad84036c7aa50` (`main`)
- `origin/main` matched before the checkpoint; no product source was changed.
- The checks exercise existing RDM and Art-Net/Enttec framing contracts only.

## Focused evidence

| Check | Result |
| --- | --- |
| `cargo test -p io --release --locked rdm -- --test-threads=1` | PASS — 14 passed, 0 failed, 0 ignored; 169 filtered out |

The run covered RDM round-trip encoding, length/checksum/command corruption
rejection, response-type handling, collision-safe discovery decoding, device
info parsing, Art-Net RDM transaction identity, ACK timer/overflow handling,
Enttec framing, oversize/bad-delimiter rejection, and discovery packet range
encoding.

The native Rust run used `vcvars64.bat -vcvars_ver=14.44` and the exact
Build Tools x64 linker returned first by `where.exe link.exe`:
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.

## Remaining boundary

`COV-RDM-001` remains `In progress`. No physical RDM device discovery,
timeout/cancellation capture, replacement-mapping ownership drill, or TOD
hardware evidence was performed. The parser/codec result cannot be promoted
to physical RDM/TOD acceptance.
