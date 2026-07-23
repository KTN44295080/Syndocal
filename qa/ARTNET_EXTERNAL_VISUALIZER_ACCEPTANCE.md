# Art-Net External Visualizer Acceptance

Date: 2026-07-23
Scope: T23 software acceptance for the formal external-visualizer product boundary. Syndocal does not ship an internal 3D visualizer.

## Verdict

**PASS for the Syndocal software path.** A native WebView2 build imported the real `Shinkan2026.dvc` show, drove a separate UDP process through Art-Net universe 0, and produced valid changing 512-channel ArtDMX frames at the engine cadence. A second-computer visualizer, physical node and fixture rig remain external acceptance; they are not relabeled as software failures or silently claimed complete.

## Reproducible receiver

`qa/harnesses/artnet-monitor.mjs` is independent of the Syndocal process. It binds UDP 6454, rejects non-ArtDMX payloads, tracks sequences per source/universe stream, renders a 512-channel browser dashboard on `127.0.0.1:6455`, and writes a JSON evidence artifact.

```powershell
node qa/harnesses/artnet-monitor.mjs --self-test
node qa/harnesses/artnet-monitor.mjs --artnet-port 6454 --http-port 6455 --duration-seconds 20 --evidence target/qa/artnet-shinkan-acceptance.json
```

Self-test result: 3 assertions passed, including the rule that a new UDP source port starts a new ArtDMX sequence stream instead of manufacturing packet loss.

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

## IP-path diagnosis

The same-PC physical-NIC target `192.168.1.34:6454` reported successful sends in Syndocal but did not loop packets back to either the independent receiver or TouchDesigner. Retargeting the same route to `127.0.0.1:6454` immediately produced continuous valid frames. This isolates the local failure to the host's same-NIC self-return/bind/firewall path, not ArtDMX encoding or the Syndocal send loop.

For a real external visualizer, use the receiving computer or node's LAN address, bind its Art-Net input to the intended adapter/universe, and permit UDP 6454 on that receiver. Do not use the Syndocal computer's own LAN address as proof of cross-machine delivery.

## Commercial visualizer boundary on this host

- ChamSys MagicVis could not be used because its installed copy reported that the MagicQ installation was corrupted and requested reinstallation.
- TouchDesigner 2023.12000 exposed the host NIC address but did not receive the same-PC physical-NIC self-target. Security/firewall settings were not changed.
- The independent receiver proves Syndocal's packet boundary. Visual rendering in a commercial external application and physical LAN delivery remain explicit environment/hardware follow-ups.
