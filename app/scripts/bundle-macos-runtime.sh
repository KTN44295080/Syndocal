#!/usr/bin/env bash
set -eo pipefail

app_path=${1:?usage: bundle-macos-runtime.sh APP_PATH DMG_DIR}
dmg_dir=${2:?usage: bundle-macos-runtime.sh APP_PATH DMG_DIR}
ffmpeg_prefix=${FFMPEG_PREFIX:?FFMPEG_PREFIX must point to the LGPL FFmpeg installation}
workspace_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
executable="$app_path/Contents/MacOS/syndocal"
frameworks_dir="$app_path/Contents/Frameworks"

test -x "$executable"
mkdir -p "$frameworks_dir"
echo "Bundling FFmpeg runtime from $ffmpeg_prefix into $app_path"

runtime_libraries=()
append_runtime_library() {
  local candidate=$1
  if [[ "$candidate" != "$ffmpeg_prefix"/lib/* ]]; then
    return 0
  fi
  local existing
  for existing in "${runtime_libraries[@]}"; do
    if [[ "$existing" == "$candidate" ]]; then
      return 0
    fi
  done
  runtime_libraries+=("$candidate")
}

while IFS= read -r dependency; do
  append_runtime_library "$dependency"
done < <(otool -L "$executable" | tail -n +2 | awk '{print $1}')

library_index=0
while [[ $library_index -lt ${#runtime_libraries[@]} ]]; do
  source=${runtime_libraries[$library_index]}
  destination="$frameworks_dir/$(basename "$source")"
  cp -Lf "$source" "$destination"
  while IFS= read -r dependency; do
    append_runtime_library "$dependency"
  done < <(otool -L "$destination" | tail -n +2 | awk '{print $1}')
  library_index=$((library_index + 1))
done

if [[ ${#runtime_libraries[@]} -eq 0 ]]; then
  echo "No FFmpeg runtime libraries were found in $ffmpeg_prefix/lib" >&2
  exit 1
fi

bundled_libraries=()
while IFS= read -r library; do
  bundled_libraries+=("$library")
done < <(find "$frameworks_dir" -maxdepth 1 -type f -name '*.dylib' -print | sort)
for library in "${bundled_libraries[@]}"; do
  install_name_tool -id "@rpath/$(basename "$library")" "$library"
done

targets=("$executable" "${bundled_libraries[@]}")
for target in "${targets[@]}"; do
  while IFS= read -r dependency; do
    [[ "$dependency" == "$ffmpeg_prefix"/lib/* ]] || continue
    install_name_tool -change "$dependency" "@rpath/$(basename "$dependency")" "$target"
  done < <(otool -L "$target" | tail -n +2 | awk '{print $1}')

  if ! otool -l "$target" | grep -A2 LC_RPATH | grep -q '@executable_path/../Frameworks'; then
    install_name_tool -add_rpath '@executable_path/../Frameworks' "$target"
  fi
done

if otool -L "${targets[@]}" | grep -F "$ffmpeg_prefix" \
  || otool -L "${targets[@]}" | grep -E '/opt/homebrew/|/Users/runner/'; then
  echo 'The packaged application still contains a build-machine library path.' >&2
  exit 1
fi

codesign --force --deep --sign - "$app_path"
codesign --verify --deep --strict "$app_path"

mkdir -p "$dmg_dir"
dmg_path="$dmg_dir/Syndocal_1.2.0-alpha.78_$(uname -m).dmg"
dmg_stage=$(mktemp -d)
trap 'rm -rf "$dmg_stage"' EXIT
ditto "$app_path" "$dmg_stage/Syndocal.app"
ln -s /Applications "$dmg_stage/Applications"
cp "$workspace_root/tools/distribution/Remove-Syndocal.command" "$dmg_stage/Remove-Syndocal.command"
chmod 755 "$dmg_stage/Remove-Syndocal.command"
# Use logical file lengths: allocated blocks can undercount compressed or cloned files.
dmg_payload_bytes=$(find "$dmg_stage" -type f -exec stat -f '%z' {} + | awk '{bytes += $1} END {printf "%.0f\n", bytes}')
[[ "$dmg_payload_bytes" =~ ^[0-9]+$ ]] || { echo 'Invalid DMG payload size' >&2; exit 1; }
dmg_payload_mb=$(((dmg_payload_bytes + 1048575) / 1048576))
dmg_size_mb=$((dmg_payload_mb + dmg_payload_mb / 5 + 64))
echo "DMG payload: $dmg_payload_bytes bytes; volume capacity: $dmg_size_mb MiB"
df -k "$dmg_stage" "$dmg_dir"
hdiutil create \
  -volname 'Syndocal 1.2.0-alpha.78' \
  -fs HFS+ \
  -size "${dmg_size_mb}m" \
  -srcfolder "$dmg_stage" \
  -ov \
  -format UDZO \
  "$dmg_path"

echo "Bundled ${#bundled_libraries[@]} LGPL FFmpeg libraries and created $dmg_path"
