#!/bin/bash
set -euo pipefail
# No sudo, broad filename search, or deletion outside these exact product paths.
identifier=jp.seraf.ktn.syndocal
if pgrep -x syndocal >/dev/null; then
  echo 'Syndocalを終了してから実行してください。 / Close Syndocal first.'
  exit 1
fi
user_home=$(cd "$HOME" && pwd -P)
targets=(
  "$user_home/Library/Application Support/$identifier"
  "$user_home/Library/Caches/$identifier"
  "$user_home/Library/WebKit/$identifier"
  "$user_home/Library/Logs/$identifier"
  "$user_home/Library/Preferences/$identifier.plist"
  "$user_home/Library/Saved Application State/$identifier.savedState"
)
for app in /Applications/Syndocal.app "$user_home/Applications/Syndocal.app"; do
  if [[ -e "$app" || -L "$app" ]]; then
    [[ ! -L "$app" && -d "$app" ]] || { echo 'Redirected application rejected'; exit 1; }
    actual=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist")
    [[ "$actual" == "$identifier" ]] || { echo 'Unrecognized application rejected'; exit 1; }
    targets+=("$app")
  fi
done
temporary=$(getconf DARWIN_USER_TEMP_DIR)
[[ -n "$temporary" && "$temporary" == /var/folders/*/T/ ]] || { echo 'Invalid user temporary directory'; exit 1; }
temporary=$(cd "$temporary" && pwd -P)
targets+=("$temporary/Syndocal/crash-reports")
shopt -s nullglob
for entry in "$temporary"/syndocal-media-snapshot-* "$temporary"/syndocal-gdtf-share-*.cookies; do
  name=${entry##*/}
  if [[ "$name" =~ ^syndocal-media-snapshot-[0-9]+-[0-9a-f]+-[0-9a-f]+$ ||
        "$name" =~ ^syndocal-gdtf-share-[0-9]+-[0-9]+\.cookies$ ]]; then targets+=("$entry"); fi
done
# Preflight every existing ancestor before deleting anything. rm never follows
# symlinks inside a target; no redirected target/ancestor is admitted.
for target in "${targets[@]}"; do
  [[ "$target" == "$user_home/Library/"* || "$target" == "$temporary/"* ||
     "$target" == /Applications/Syndocal.app || "$target" == "$user_home/Applications/Syndocal.app" ]] || exit 1
  cursor=$target
  while [[ "$cursor" != / ]]; do
    [[ ! -L "$cursor" ]] || { echo 'Redirected cleanup path rejected'; exit 1; }
    cursor=${cursor%/*}; [[ -n "$cursor" ]] || cursor=/
  done
done
printf '%s\n' '削除予定 / Planned removal:' "${targets[@]}"
if [[ "${1:-}" == --validate-only ]]; then echo 'PASS: nothing removed'; exit 0; fi
[[ $# == 0 ]] || { echo 'Unknown argument'; exit 1; }
printf '本体・設定・キャッシュ・復旧データを完全削除します。外部のプロジェクトと素材は保持します。続行するには REMOVE と入力: '
read -r answer
[[ "$answer" == REMOVE ]] || { echo 'Cancelled'; exit 0; }
for target in "${targets[@]}"; do
  if [[ -e "$target" ]]; then rm -rf -- "$target"; fi
  [[ ! -e "$target" && ! -L "$target" ]] || { echo "Unable to remove $target"; exit 1; }
done
rmdir "$temporary/Syndocal" 2>/dev/null || true
echo 'Syndocalを完全削除しました。 / Syndocal removed.'
