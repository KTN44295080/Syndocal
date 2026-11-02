#!/bin/bash
# Collect only this acceptance run's files; never opens a device or starts the app.
set -euo pipefail
kit_dir=$(cd "$(dirname "$0")" && pwd -P)
if [[ $(uname -s) != Darwin ]]; then
  echo 'macOS only / macOSで実行してください。' >&2
  exit 1
fi
app=/Applications/Syndocal.app
if [[ ${1:-} == --app && $# == 2 ]]; then app=$2
elif [[ $# != 0 ]]; then echo 'Usage: collect-syphon-acceptance.command [--app /path/Syndocal.app]' >&2; exit 1
fi
stamp=$(date -u +%Y%m%dT%H%M%SZ)
desktop="$HOME/Desktop"
[[ -d "$desktop" ]] || { echo 'Desktop folder missing / Desktopフォルダがありません。' >&2; exit 1; }
result_dir=$(mktemp -d "$desktop/syphon-friend-result-${stamp}-XXXXXX")
echo "収集先 / Results: $result_dir"
echo 'screenshots/screenshot1.png に映像、screenshot2.png にBlackoutを保存してください。'
echo 'Save the output and blackout screenshots as screenshots/screenshot1.png and screenshot2.png in the kit folder.'
echo 'observations.txt の結果を記入し、Enterを押してください。 / Complete observations.txt, then press Enter.'
if [[ -t 0 ]]; then read -r _; fi
sw_vers > "$result_dir/macos.txt" 2>&1
uname -m > "$result_dir/architecture.txt"
system_profiler SPDisplaysDataType > "$result_dir/displays.txt" 2>&1 || true
app_identity=false; signature=false; linkage=false; framework=false
if [[ -d "$app" && ! -L "$app" && -f "$app/Contents/Info.plist" ]]; then
  identity=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist" 2>/dev/null || true)
  if [[ "$identity" == jp.seraf.ktn.syndocal ]]; then
    app_identity=true
    /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist" > "$result_dir/app-version.txt" 2>&1 || true
    codesign -dvv "$app" > "$result_dir/codesign-details.txt" 2>&1 || true
    if codesign --verify --deep --strict "$app" > "$result_dir/codesign-verify.txt" 2>&1; then signature=true; fi
    if otool -L "$app/Contents/MacOS/syndocal" > "$result_dir/linkage.txt" 2>&1; then
      if grep -F '@rpath/Syphon.framework/' "$result_dir/linkage.txt" >/dev/null; then linkage=true; fi
    fi
    if [[ -f "$app/Contents/Frameworks/Syphon.framework/Syphon" && -f "$app/Contents/Frameworks/Syphon.framework/Resources/default.metallib" ]]; then framework=true; fi
  fi
fi
log="$HOME/Library/Application Support/jp.seraf.ktn.syndocal/logs/syphon-acceptance.json"
# A report is bounded to 1 MiB; an absent/stale/invalid report is never a pass.
if [[ -f "$log" && ! -L "$log" ]]; then
  bytes=$(stat -f %z "$log")
  if [[ "$bytes" -gt 0 && "$bytes" -le 1048576 ]]; then
    cp "$log" "$result_dir/syphon-acceptance.json"
  fi
fi
mkdir "$result_dir/screenshots"
screenshots=0
for name in screenshot1.png screenshot2.png; do
  source="$kit_dir/screenshots/$name"
  if [[ -f "$source" && ! -L "$source" ]]; then
    bytes=$(stat -f %z "$source")
    if [[ "$bytes" -gt 0 && "$bytes" -le 20971520 ]] && file -b "$source" | grep '^PNG image data' >/dev/null; then
      cp "$source" "$result_dir/screenshots/$name"
      screenshots=$((screenshots + 1))
    fi
  fi
done
if [[ -f "$kit_dir/observations.txt" && ! -L "$kit_dir/observations.txt" && $(stat -f %z "$kit_dir/observations.txt") -le 65536 ]]; then
  cp "$kit_dir/observations.txt" "$result_dir/observations.txt"
fi
if [[ -f "$kit_dir/kit-manifest.json" && ! -L "$kit_dir/kit-manifest.json" && $(stat -f %z "$kit_dir/kit-manifest.json") -le 1048576 ]]; then
  cp "$kit_dir/kit-manifest.json" "$result_dir/kit-manifest.json"
fi
# macOS ships JavaScript for Automation; no Python/Node installation is needed.
# Validate only retained copies, so the checks describe exactly the returned bytes.
if ! /usr/bin/osascript -l JavaScript - "$result_dir" > "$result_dir/diagnostic-checks.json" 2> "$result_dir/diagnostic-validation.txt" <<'JXA'
// BEGIN DIAGNOSTIC VALIDATOR (also exercised as pure JavaScript by tests)
function checkDiagnostic(report, kit, now) {
  function object(v) { return v !== null && typeof v === 'object' && !Array.isArray(v); }
  function integer(v) { return Number.isSafeInteger(v) && v >= 0; }
  var validKit = object(kit) && kit.schema_version === 1 && /^[0-9a-f]{40}$/.test(kit.commit) &&
    typeof kit.version === 'string' && kit.version.length > 0 && kit.syphon_input_enabled === false;
  var validReport = object(report) && report.schema_version === 1 && integer(report.process_id) && report.process_id > 0 &&
    typeof report.app_version === 'string' && typeof report.build_commit === 'string' && /^[0-9a-f]{40}$/.test(report.build_commit) &&
    report.input_enabled === false && report.external_client_verified === false && object(report.routes) && Object.keys(report.routes).length <= 64;
  var matchedBuild = validKit && validReport && report.build_commit === kit.commit && report.app_version === kit.version;
  var routes = validReport ? Object.keys(report.routes).map(function (id) { return report.routes[id]; }).filter(function (r) {
    return object(r) && r.server_name === 'Syndocal-Syphon-Test';
  }) : [];
  var route = routes.length === 1 ? routes[0] : null;
  var validRoute = !!route && integer(route.output_id) && integer(route.frames_published) &&
    typeof route.native_retired === 'boolean' && typeof route.route_state === 'string' && integer(route.observed_unix_ms) &&
    (route.last_error === null || typeof route.last_error === 'string') && object(route.output_ownership) &&
    object(route.native) && typeof route.native.macos_version === 'string' &&
    (route.native.metal_device_name === null || typeof route.native.metal_device_name === 'string') &&
    (route.native.has_unified_memory === null || typeof route.native.has_unified_memory === 'boolean');
  // Recency is only a plausibility check, never proof that this was the friend's run.
  var recent = validRoute && route.observed_unix_ms <= now + 300000 && route.observed_unix_ms >= now - 86400000;
  var current = matchedBuild && validRoute && recent;
  return { report_json_valid: object(report), report_schema_valid: !!validReport, kit_manifest_valid: !!validKit,
    matched_build: !!matchedBuild, named_test_route_valid: !!validRoute, timestamp_within_24_hours: !!recent,
    test_route_published: !!(current && route.frames_published > 0),
    test_route_native_retired: !!(current && route.native_retired && route.route_state === 'native_closed'),
    test_route_no_native_error: !!(current && route.last_error === null),
    test_route_1920x1080: !!(current && route.width === 1920 && route.height === 1080) };
}
function run(argv) {
  ObjC.import('Foundation');
  function readJSON(name) {
    try {
      var value = $.NSString.stringWithContentsOfFileEncodingError(argv[0] + '/' + name, $.NSUTF8StringEncoding, null);
      return JSON.parse(ObjC.unwrap(value));
    } catch (_) { return null; }
  }
  return JSON.stringify(checkDiagnostic(readJSON('syphon-acceptance.json'), readJSON('kit-manifest.json'), Date.now()));
}
// END DIAGNOSTIC VALIDATOR
JXA
then
  echo '{"validation_execution_failed":true,"report_schema_valid":false,"matched_build":false,"test_route_published":false,"test_route_native_retired":false}' > "$result_dir/diagnostic-checks.json"
fi
cat > "$result_dir/collection.json" <<EOF
{"schema_version":1,"collected_utc":"$stamp","app_identity_verified":$app_identity,"codesign_verified":$signature,"syphon_linkage_verified":$linkage,"framework_present":$framework,"diagnostic_checks":$(cat "$result_dir/diagnostic-checks.json"),"screenshots_collected":$screenshots,"physical_acceptance":"requires_owner_review","syphon_input":"not_built"}
EOF
cat > "$result_dir/READ-ME.txt" <<EOF
Collection is evidence, not an acceptance verdict. Review diagnostic build/time/server,
nonzero published frame count, ownership, any error, screenshots and observations.
Missing/false values block the corresponding claim; stale logs do not prove this run.
Syphon input is not built in Kit 1. Screenshot 2 records Blackout instead.
収集完了は合格ではありません。診断のビルド・時刻・サーバー・フレーム数・所有権・エラー、
スクリーンショットと観察結果を確認してください。不足や古いログは未検証として扱います。
EOF
zip_path="$result_dir.zip"
ditto -c -k --keepParent "$result_dir" "$zip_path"
echo "返送するファイル / Send back: $zip_path"
echo '元の収集フォルダも保存しました。 / The source evidence folder is preserved.'
