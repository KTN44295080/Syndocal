#!/usr/bin/env bash
# Prepare the official Syphon.framework for macOS builds. No Windows/Linux SDK
# dependency; no upstream executable or framework binary is downloaded.
set -euo pipefail

[[ "$(uname -s)" == Darwin ]] || { echo 'Syphon can only be prepared on macOS.' >&2; exit 1; }
workspace_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
syphon_commit=f4761677a45b8034a3c2069ec0f3d2553da81fba
syphon_sha256=a3766088c46588c42b087f4e5ff7ae7bb0f83292af59a0aeb8bba69eba5311c5
syphon_root="$workspace_root/target/syphon"
syphon_frameworks="$syphon_root/frameworks"
syphon_archive="$syphon_root/Syphon-Framework-$syphon_commit.tar.gz"

mkdir -p "$syphon_root" "$syphon_frameworks"
xcrun --sdk macosx --find metal >/dev/null
if [[ ! -f "$syphon_archive" ]]; then
  curl --fail --location --retry 3 \
    "https://codeload.github.com/Syphon/Syphon-Framework/tar.gz/$syphon_commit" \
    --output "$syphon_archive"
fi
printf '%s  %s\n' "$syphon_sha256" "$syphon_archive" | shasum -a 256 -c -

# Always extract fresh input from the verified archive; edited or stale source
# directories cannot silently change the binary generated from this pin.
syphon_stage=$(mktemp -d "$syphon_root/build-input.XXXXXX")
trap 'rm -rf "$syphon_stage"' EXIT
tar -xzf "$syphon_archive" -C "$syphon_stage"
xcodebuild \
  -project "$syphon_stage/Syphon-Framework-$syphon_commit/Syphon.xcodeproj" \
  -scheme Syphon -configuration Release -sdk macosx \
  "CONFIGURATION_BUILD_DIR=$syphon_stage/products" \
  "OBJROOT=$syphon_stage/objects" \
  'ARCHS=arm64 x86_64' ONLY_ACTIVE_ARCH=NO \
  MACOSX_DEPLOYMENT_TARGET=12.0 \
  'OTHER_CFLAGS=$(inherited) -fobjc-arc-exceptions' \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO

test -f "$syphon_stage/products/Syphon.framework/Syphon"
test -f "$syphon_stage/products/Syphon.framework/Resources/default.metallib"
ditto "$syphon_stage/products/Syphon.framework" "$syphon_frameworks/Syphon.framework"
lipo -verify_arch arm64 x86_64 "$syphon_frameworks/Syphon.framework/Syphon"
otool -D "$syphon_frameworks/Syphon.framework/Syphon" | grep -F '@rpath/Syphon.framework/'
printf 'commit=%s\narchive_sha256=%s\ndeployment_target=12.0\narchitectures=arm64 x86_64\n' \
  "$syphon_commit" "$syphon_sha256" > "$syphon_root/source-receipt.txt"
echo "Prepared pinned Syphon.framework in $syphon_frameworks"
