# Generated thumbnail test media

`thumbnail-two-patterns.mp4` is a generated 96x64, two-frame, silent MPEG-4 Part 2
fixture at 2 fps. Duration: 1 second. It contains no external image or user data.
Each frame has four solid quadrants in top-left, top-right, bottom-left, bottom-right order:

- Frame 0 / 0 ms: RGB red (255,0,0), green (0,255,0), blue (0,0,255), white (255,255,255).
- Frame 1 / 500 ms: yellow (255,255,0), cyan (0,255,255), magenta (255,0,255), gray (80,80,80).

Created with local FFmpeg 8.1.2 using two generated PNGs numbered 0 and 1:

```sh
ffmpeg -nostdin -hide_banner -loglevel error -n -framerate 2 -i frames/%d.png \
  -frames:v 2 -an -c:v mpeg4 -q:v 2 -g 1 -pix_fmt yuv420p thumbnail-two-patterns.mp4
```

Size: 1344 bytes. SHA-256:
`56fcbb1725220525f53e87cccaf6eba670c4888c35aeef05c381e41193648d69`.
The tests sample quadrant centers, away from chroma/interpolation boundaries.
The exact generation command and source patterns are retained in the local QA
record; tests use this frozen clip, not an unbounded encoder subprocess.
