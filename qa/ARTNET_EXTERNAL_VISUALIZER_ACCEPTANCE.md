# Art-Net External Visualizer Acceptance

Date: 2026-07-23
Scope: T23 software acceptance for the formal external-visualizer product boundary. Syndocal does not ship an internal 3D visualizer.

## Verdict

**PASS for the Syndocal software path.** A native WebView2 build imported the real `Shinkan2026.dvc` show, drove a separate UDP process through Art-Net universe 0, and produced valid changing 512-channel ArtDMX frames at the engine cadence. A second-computer visualizer, physical node and fixture rig remain external acceptance; they are not relabeled as software failures or silently claimed complete.

## Reproducible receiver

`qa/harnesses/artnet-monitor.mjs` is independent of the Syndocal process. It binds UDP 6454, rejects non-ArtDMX payloads, tracks sequences per source/universe stream, renders a 512-channel browser dashboard on `127.0.0.1:6455`, and writes a JSON evidence artifact.

```powershell
node qa/harnesses/artnet-monitor.mjs --self-test
node qa/harnesses/artnet-monitor.mjs --artnet-port 6454 --http-port 6455 --duration-seconds 20 --capture-changes --max-transitions 1024 --evidence target/qa/artnet-shinkan-acceptance.json
node qa/harnesses/artnet-compare.mjs --self-test
node qa/harnesses/artnet-compare.mjs --reference target/qa/daslight-scene.json --candidate target/qa/syndocal-scene.json --universe 0 --evidence target/qa/artnet-comparison.json
```

Monitor self-test result: 4 assertions passed, including the rule that a new UDP source port starts a new ArtDMX sequence stream instead of manufacturing packet loss and that bounded full transition capture retains channel bytes. Comparator self-test result: 3 assertions passed for exact equality, a one-channel delta and malformed evidence rejection.

The monitor keeps `lastFrames` by universe as well as the most recent global frame. The comparator's equality verdict is the selected universe's final DMX frame, byte-for-byte. Transition digest counts and common prefixes are included as diagnostics; dynamic FX timing/phase needs a separately synchronized acceptance rule.

## Native real-show exercise

- Application: `target/debug/syndocal.exe`, native WebView2.
- Source: `C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc`.
- Import report: 41 fixtures, 15 profiles, 20 fixture groups, 13 scene banks, 30 cues, 22 converted tempo records, 221 beam-wait records with 0 beam-wait errors, 6 converted effects, 44 approximations, 21 skipped records and 5 unsupported records. The report keeps approximations/skips explicit.
- Route: Art-Net `127.0.0.1:6454`, universe 0, enabled from Setup > I/O > DMX.
- Operations: trigger `1.1 新宝島`; trigger dynamic `2.1 Fl-StrobeChase`; drag Scene Live speed from authored `x1` to `x4`; re-trigger the Cue and observe authored `x1` restoration.
- Output result: dynamic FX and the x4 latch produced changing ArtDMX payloads in the independent process; re-trigger restored the native Scene Live control to x1.

Final 20-second evidence (`target/qa/artnet-shinkan-acceptance.json`):

| Metric | Result |
|---|---:|
| Valid ArtDMX frames | 880 |
| Frames with changed payload | 880 |
| Rejected datagrams | 0 |
| Per-source/universe sequence discontinuities | 0 |
| Maximum inter-frame gap | 25 ms |
| Universe / payload length | 0 / 512 bytes |
| Last-frame non-zero channels / maximum value | 431 / 255 |
| Sender stream | `127.0.0.1:57600/u0` |

The measured rate is 44 frames/s over the 20-second window. This is an external-process receipt result, not a Syndocal status-label assertion.

## Daslight-to-Syndocal A/B capture status (2026-08-07)

The comparison path is executable without a Daslight Art-Net licence. The
supported `DMX LEVELS` window exposes the internal 512-channel values; a captured
730x601 frame can be converted into comparator-compatible JSON with
`qa/harnesses/daslight-dmx-levels-reference.ps1`. Exact bytes are transcribed
from the visible numbers and independently checked against the visible blue bar
set and approximate bar height. This is a final-frame comparison path, not a
44 Hz packet-stream capture.

- Daslight source: `C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc` (344,765 bytes, SHA-256 `ED202B33878E42124D8DEDA2088239C9E8A1B7C65EFDC56821B6317FF9BE3099`).
- Daslight virtual device: `Codex Capture`, active, Art-Net 4, `192.168.1.255`, mask `255.255.255.0`, port 6454, Art-Net universe 0 mapped to software universe 1.
- Directed capture while switching `Shin`/`Unr`: 30 seconds, ArtDMX 0, rejected non-ArtDMX 1.
- Broadcast capture with `Use Broadcast` enabled while switching `Unr`/`Shin`: 20 seconds, ArtDMX 0, rejected non-ArtDMX 1.
- Hardware Manager reported no connected device. Daslight listed a cached `DVC GOLD` serial `1021943`, but its connectivity remained disabled.

Daslight's manual states that Art-Net output needs a connected compatible SUT device with a valid Art-Net licence. The virtual device and universe mapping can therefore be configured without a reference ArtDMX stream being licensed for output. A licensed interface remains necessary for Daslight packet cadence/sequence comparison, but it is no longer required for stable visible final-frame byte comparison. Until the same named scene and time point pass `artnet-compare.mjs`, do not label the import/output byte-identical to Daslight.

Official source: [Daslight 5 v1.4 manual](https://eu-litterature.n-g.co/Release/daslight_5_manual_en.pdf).

The same current release build produced independent Syndocal evidence after Daslight released UDP 6454:

| Scene | Window | ArtDMX frames | Changed frames | Rejected | Sequence gaps | Max gap | U / bytes | Last non-zero | Last digest |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `Shin` | 12 s | 529 | 1 | 0 | 0 | 24 ms | 0 / 512 | 283 | `e7817420` |
| `Unr` | 10 s | 440 | 347 | 0 | 0 | 24 ms | 0 / 512 | 289 | `5f647702` |

The comparator returned 0 for the real `Shin` evidence compared with itself. As a negative control, `Shin` versus `Unr` returned 1 and reported 26 exact channel differences; the first was channel 72, `26 -> 255`. This proves that the comparison gate detects payload differences instead of merely checking packet presence.

Run the two product captures sequentially. Windows allowed Daslight and the monitor to share UDP 6454 for the Daslight attempt, but leaving Daslight bound while Syndocal sent to `127.0.0.1` caused the unicast to be consumed by the competing socket. Closing Daslight before the Syndocal capture removed that host-only ambiguity and immediately restored valid ArtDMX receipt.

## IP-path diagnosis

The same-PC physical-NIC target `192.168.1.34:6454` reported successful sends in Syndocal but did not loop packets back to either the independent receiver or TouchDesigner. Retargeting the same route to `127.0.0.1:6454` immediately produced continuous valid frames. This isolates the local failure to the host's same-NIC self-return/bind/firewall path, not ArtDMX encoding or the Syndocal send loop.

For a real external visualizer, use the receiving computer or node's LAN address, bind its Art-Net input to the intended adapter/universe, and permit UDP 6454 on that receiver. Do not use the Syndocal computer's own LAN address as proof of cross-machine delivery.

## Commercial visualizer boundary on this host

- ChamSys MagicVis could not be used because its installed copy reported that the MagicQ installation was corrupted and requested reinstallation.
- TouchDesigner 2023.12000 exposed the host NIC address but did not receive the same-PC physical-NIC self-target. Security/firewall settings were not changed.
- The independent receiver proves Syndocal's packet boundary. Visual rendering in a commercial external application and physical LAN delivery remain explicit environment/hardware follow-ups.
