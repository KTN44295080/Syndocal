#!/bin/bash
set -euo pipefail
root="$PWD"
mkdir -p inspect
unzip -q candidate/*.zip -d inspect
sdk=(inspect/*)
test "${#sdk[@]}" -eq 1
bin="${sdk[0]}/bin"
test "$(find "$bin" -maxdepth 1 -name '*.dll' | wc -l)" -eq 7
for dll in "$bin"/*.dll; do
  # Exit 1 means no matches; exit 2 (read/error) fails closed.
  set +e
  grep -aEin 'chromaprint|fftw' "$dll" > candidate/forbidden-matches.txt
  status=$?
  set -e
  if [[ $status -ne 1 ]]; then
    echo "Forbidden strings or scan error in $dll (grep=$status)" >&2
    exit 1
  fi
  printf '%s: chromaprint/fftw matches=0\n' "$(basename "$dll")"
done
sha256sum "$bin"/*.dll | tee candidate/dll-sha256.txt
docker run --rm -v "$root/${sdk[0]}":/sdk:ro -v "$root/candidate":/evidence \
  -e WINEDEBUG=-all -e WINEPREFIX=/tmp/candidate-wine \
  ghcr.io/btbn/ffmpeg-builds/win64-lgpl-shared-8.1:latest bash -c '
    set -euo pipefail
    wine /sdk/bin/ffmpeg.exe -hide_banner -buildconf > /evidence/configure.txt 2>&1
    wine /sdk/bin/ffmpeg.exe -hide_banner -L > /evidence/license.txt 2>&1
    wine /sdk/bin/ffmpeg.exe -hide_banner -muxers > /evidence/muxers.txt 2>&1
    wine /sdk/bin/ffmpeg.exe -hide_banner -decoders > /evidence/decoders.txt 2>&1
    wine /sdk/bin/ffmpeg.exe -hide_banner -hwaccels > /evidence/hwaccels.txt 2>&1
  '
cat candidate/configure.txt candidate/license.txt candidate/muxers.txt
if grep -Ei -- '--enable-(chromaprint|gpl|nonfree)' candidate/configure.txt; then exit 1; fi
if grep -Ei 'chromaprint' candidate/muxers.txt; then exit 1; fi
# ffmpeg -L prints the licence prose, not the literal "LGPL". Fail closed on GPL
# or nonfree wording, then require the Lesser GPL v2.1+/v3+ grant explicitly.
if grep -Ei 'GNU General Public License|nonfree|not legally redistributable' candidate/license.txt; then
  echo "GPL or nonfree licence wording in ffmpeg -L output" >&2
  exit 1
fi
grep -F 'under the terms of the GNU Lesser General Public License as published by' candidate/license.txt
grep -E 'either version (2\.1|3) of the License, or' candidate/license.txt
