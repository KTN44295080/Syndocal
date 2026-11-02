#!/bin/bash
# Optional, explicit Gatekeeper helper for this exact application identity only.
set -euo pipefail
app=/Applications/Syndocal.app
[[ $(uname -s) == Darwin && $# == 0 ]] || { echo 'Run on macOS without arguments.' >&2; exit 1; }
for path in /Applications "$app" "$app/Contents" "$app/Contents/Info.plist"; do
  [[ ! -L "$path" ]] || { echo 'Redirected application rejected.' >&2; exit 1; }
done
[[ -d "$app" ]] || { echo 'Install /Applications/Syndocal.app first.' >&2; exit 1; }
identity=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist")
[[ "$identity" == jp.seraf.ktn.syndocal ]] || { echo 'Unrecognized application.' >&2; exit 1; }
echo 'Syndocal.appのみ隔離属性を解除します。 / Clear quarantine only for /Applications/Syndocal.app.'
xattr -cr "$app"
echo '完了。Syndocalを開いてください。 / Done. Open Syndocal.'
