#!/usr/bin/env bash
set -euo pipefail
app_path=${1:?usage: build-macos-installer.sh APP_PATH OUTPUT_DIR}
output_dir=${2:?usage: build-macos-installer.sh APP_PATH OUTPUT_DIR}
workspace_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
version=$(node -p "require(process.argv[1]).version" "$workspace_root/app/package.json")
package_version=$(node -p "const v = require(process.argv[1]).version.match(/^([0-9]+)\\.([0-9]+)\\.0-alpha\\.([0-9]+)$/); if (!v) throw Error('Unsupported package version'); v.slice(1).join('.')" "$workspace_root/app/package.json")
[[ $(uname -s) == Darwin && $(uname -m) == arm64 ]] || exit 1
[[ -d "$app_path" && ! -L "$app_path" ]] || exit 1
codesign --verify --deep --strict "$app_path"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/app-scripts" "$stage/djlink-scripts" "$stage/core-scripts" "$output_dir"
source_dir="$workspace_root/tools/distribution/macos-installer"
cp "$source_dir/preinstall" "$stage/app-scripts/preinstall"
chmod 755 "$stage/app-scripts/preinstall"
mkdir -p "$stage/app-root"
ditto "$app_path" "$stage/app-root/Syndocal.app"
pkgbuild --analyze --root "$stage/app-root" "$stage/app-components.plist"
# An existing build/DMG copy must not redirect installation away from Applications.
/usr/libexec/PlistBuddy -c 'Set :0:BundleIsRelocatable false' "$stage/app-components.plist"
pkgbuild --root "$stage/app-root" --component-plist "$stage/app-components.plist" --install-location /Applications --scripts "$stage/app-scripts" --identifier jp.seraf.ktn.syndocal.pkg.app --version "$package_version" "$stage/app.pkg"
for feature in djlink core; do
  enabled=0; [[ "$feature" != djlink ]] || enabled=1
  cp "$source_dir/preinstall" "$stage/$feature-scripts/preinstall"
  sed "s/@DJLINK@/$enabled/g" "$source_dir/postinstall" > "$stage/$feature-scripts/postinstall"
  chmod 755 "$stage/$feature-scripts/"*
  pkgbuild --nopayload --scripts "$stage/$feature-scripts" --identifier "jp.seraf.ktn.syndocal.pkg.$feature" --version "$package_version" "$stage/$feature.pkg"
done
sed "s/@VERSION@/$version/g" "$source_dir/Distribution.xml" > "$stage/Distribution.xml"
productbuild --distribution "$stage/Distribution.xml" --package-path "$stage" "$output_dir/Syndocal_${version}_arm64.pkg"
shasum -a 256 "$output_dir/Syndocal_${version}_arm64.pkg" > "$output_dir/Syndocal_${version}_arm64.pkg.sha256"
