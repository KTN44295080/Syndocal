#!/bin/bash
set -euo pipefail
# Elevated operations are limited to the exact PKG app/settings/receipt identities.
identifier=jp.seraf.ktn.syndocal
system_settings="/Library/Application Support/$identifier"
system_receipts=("$identifier.pkg.app" "$identifier.pkg.core" "$identifier.pkg.djlink")
for path in /Library '/Library/Application Support' "$system_settings"; do
  [[ ! -L "$path" ]] || { echo 'Redirected installer settings rejected'; exit 1; }
done
[[ ! -e "$system_settings" || -d "$system_settings" ]] || { echo 'Invalid installer settings directory'; exit 1; }
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
printf '%s\n' '削除予定 / Planned removal:' "${targets[@]}" "$system_settings"
if [[ "${1:-}" == --validate-only ]]; then echo 'PASS: nothing removed'; exit 0; fi
if [[ "${1:-}" == --remove-approved ]]; then
  [[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_ENVIRONMENT:-}" == github-hosted ]] || { echo 'Noninteractive removal is restricted to hosted QA'; exit 1; }
elif [[ $# != 0 ]]; then echo 'Unknown argument'; exit 1
else
printf '本体・設定・キャッシュ・復旧データを完全削除します。外部のプロジェクトと素材は保持します。続行するには REMOVE と入力: '
read -r answer
[[ "$answer" == REMOVE ]] || { echo 'Cancelled'; exit 0; }
fi
for target in "${targets[@]}"; do
  if [[ -e "$target" ]]; then
    if [[ "$target" == /Applications/Syndocal.app && ! -w "$target/Contents" ]]; then
      /usr/bin/sudo /bin/rm -rf -- /Applications/Syndocal.app
    else rm -rf -- "$target"; fi
  fi
  [[ ! -e "$target" && ! -L "$target" ]] || { echo "Unable to remove $target"; exit 1; }
done
if [[ -e "$system_settings" ]]; then
  [[ -d "$system_settings" ]] || { echo 'Invalid installer settings directory'; exit 1; }
  /usr/bin/sudo /bin/rm -rf -- "$system_settings"
  [[ ! -e "$system_settings" && ! -L "$system_settings" ]] || exit 1
fi
for receipt in "${system_receipts[@]}"; do
  if /usr/sbin/pkgutil --pkg-info "$receipt" >/dev/null 2>&1; then
    /usr/bin/sudo /usr/sbin/pkgutil --forget "$receipt" >/dev/null
  fi
done
rmdir "$temporary/Syndocal" 2>/dev/null || true
echo 'Syndocalを完全削除しました。 / Syndocal removed.'
