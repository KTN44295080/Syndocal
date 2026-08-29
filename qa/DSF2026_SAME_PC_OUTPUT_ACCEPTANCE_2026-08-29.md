# DSF2026 same-PC output acceptance — 2026-08-29

## Scope

This is the show-critical output contract for the 2026-08-30 DSF performance.
Syndocal and the Unity receiver run on the same Windows PC. Remote Art-Net and
NDI transport are deliberately outside this acceptance boundary.

The previous strict show path required a USB serial/Enttec Open DMX route. That
path is superseded for this show because the receiver consumes Art-Net and two
local Spout senders. The replacement must retain the existing output-control
lease, safety, publication, acknowledgement, and rollback boundaries; it must
not fall back to a generic unreviewed output mutation.

## Lighting contract

- Transport: Art-Net ArtDmx (`OpCode 0x5000`).
- Destination: `127.0.0.1:6454`.
- Daslight project universe: Universe 1.
- Art-Net wire universe and Syndocal internal universe: `0`.
- Payload: the completed 512-channel Syndocal DMX frame, byte values `0..255`.
- Address relation: DMX channel 1 is payload byte 0.
- Cadence: approximately 44 Hz (`22,727 us` engine tick), within the requested
  40–44 fps range.
- Channel 500 is unused and must remain zero on the wire. The strict show
  sender applies this fixed venue mask immediately before packet encoding, so
  an upstream non-zero byte cannot escape as payload byte 499.
- The strict show route must not admit an external DMX input/merge that can
  mutate Universe 0 after the completed show frame is established.

The imported patch evidence is `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`.
All 46 fixtures use Daslight Universe 1 and match the reference SDC names and
addresses 46/46 after normalization to internal Universe 0. Strongpoint is a
13-channel fixture with one dimmer and four RGB segments; the Daslight display
of twelve physical cells is not authoritative.

### Minimal lighting acceptance frame

All 512 bytes are zero except:

```text
payload[0] = 255  # DMX channel 1, first Mega PAR red
payload[4] = 255  # DMX channel 5, first Mega PAR dimmer
```

Acceptance requires one 530-byte ArtDmx packet at wire Universe 0 and visible
red output from the first Mega PAR in Unity. Daslight/Easy View must be closed
before Unity starts because the current receiver cannot share UDP port 6454.
Syndocal is a UDP sender and does not bind the receiver port.

## Video contract

Syndocal must keep exactly two local Spout senders active:

| Role | Exact sender name | Size | Off state |
| --- | --- | --- | --- |
| Background | `Syndocal Background` | 1920×1080 | continuous RGB black, opaque in Unity |
| Foreground | `Syndocal Foreground` | 1920×1080 | continuous RGB black, transparent in Unity |

Unity derives foreground transparency from RGB brightness and ignores source
alpha for that layer. Syndocal therefore does not need a new RGB-to-alpha
conversion, but both off states must be RGB byte-exact black. Stopping a
timeline or output must not destroy either show sender; black frames must keep
flowing. Sender destruction remains a separate output-route teardown operation.

## Current evidence and open gates

Static evidence already confirms the low-level ArtDmx encoder uses opcode
`0x5000`, wire Universe 0, a 512-byte payload, and `frame[0] -> payload[0]`.
The engine tick is approximately 44 Hz. The existing generic Spout worker runs
at 60 Hz and can render byte-exact RGB black.

Source checkpoint evidence now includes exact MSVC `14.44.35207` focused tests
with the Community linker pinned and first in `where.exe`: engine Art-Net
`7 passed / 0 failed`, Syndocal route `3 passed / 0 failed`, and protocol
control-plane `1 passed / 0 failed`; first-party warnings were `0`. The UDP
proof observes a 530-byte ArtDmx packet at wire U0 with 512 payload bytes,
`payload[0]=255`, `payload[4]=255`, and masked `payload[499]=0`. Independent
Terra xHigh review found one stale Setup I/O fixture and a missing publication
rollback proof; both were repaired before the focused rerun. Ox was not
callable, so this is the documented narrow review exception.

The following gates remain open until the complete Spout integration and its
native artifact are accepted:

- [x] Strict USB-serial show activation is completely replaced by exact
      `127.0.0.1:6454 / Art-Net / U0 / 512` activation.
- [x] The retired serial-show command, UI, schema, registry, tests, and
      show-only machine binding are unreachable or fail closed.
- [ ] Universe 0 input/merge cannot alter the strict completed show frame.
- [x] Channel 500 is deterministically forced to zero at the strict sender
      boundary and observed as zero on the wire.
- [x] The fixed channel 1 + channel 5 test frame is available and verified by
      a local UDP receiver test.
- [ ] Exactly two fixed 1920×1080 Spout outputs can be created or validated by
      the reviewed output-control path.
- [ ] Timeline/output stop keeps both senders alive and publishes RGB black.
- [ ] The reference show is saved to a new SDC containing the Art-Net route and
      the two fixed Spout outputs; the existing alpha9 file is not overwritten.
- [ ] Focused and full deterministic gates pass with zero first-party warnings.
- [ ] A fresh warning-free native release is built with exact MSVC 14.44,
      launched from this checkout, and verified as one responsive maximized
      Syndocal window.
- [ ] Unity physical acceptance proves the red Mega PAR frame, both exact Spout
      sender names, 1920×1080 frames, and continuous black while stopped.
