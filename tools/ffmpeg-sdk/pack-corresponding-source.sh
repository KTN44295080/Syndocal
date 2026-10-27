#!/bin/bash
# Collect and pack the corresponding source of the hosted LGPL SDK built by
# run 40366564270 (prerelease ffmpeg-sdk-lgpl-n8.1.3-r40366564270).
#
# Expects the patched recipe in ./upstream (same preparation as the build job).
# It re-runs the patched download.sh/generate.sh, proves the regenerated
# Dockerfile equals the hosted one, adds the sources fetched during the build
# (freetype dlg, kvazaar greatest, dav1d checkasm, rav1e Cargo crates), records
# resolved commits, strips VCS metadata and writes a deterministic tar.xz.
set -euo pipefail
export LC_ALL=C TZ=UTC

root="$PWD"
name=ffmpeg-sdk-lgpl-n8.1.3-r40366564270-corresponding-source
out="$root/source-out"
work="${CSRC_WORK:-${RUNNER_TEMP:-/tmp}/csrc}"
tree="$work/$name"
release=https://github.com/KTN44295080/Syndocal/releases/download/ffmpeg-sdk-lgpl-n8.1.3-r40366564270
base_image=ghcr.io/btbn/ffmpeg-builds/base@sha256:91eaaaf32f3f98464e99faf1062fab8ab99a23d0b499c72b51a02ec9137b06a0
target_image=ghcr.io/btbn/ffmpeg-builds/base-win64@sha256:8a3d189e5b615c89e1a9fe2be705fa17173d8f6ea0a955b25c7c28041cde36c7
build_commit=7d0af757f73231fc0c6006d6c21c72c14cb4a31a
epoch=1790726400
git_ro() { git -c safe.directory='*' "$@"; }

rm -rf "$tree"
mkdir -p "$out" "$work" "$tree"/{dependencies,build-time,recipe,syndocal,ffmpeg,receipts}

fetch_hosted() {
  curl --fail --location --silent --show-error --retry 3 "$release/$1" -o "$2"
  echo "$3  $2" | sha256sum --check
}
fetch_hosted dependency.Dockerfile "$tree/receipts/hosted-dependency.Dockerfile" \
  8d694efaa7eaf3e34d6c46ffa68f545c389890725f20928f2506273155ca5238
fetch_hosted build-40366564270.log "$work/hosted-build.log" \
  8685669182ea747e868e5bd969b3aef27d96d9e986c677a7078022ed8bcf85e1
fetch_hosted toolchain.txt "$tree/receipts/hosted-toolchain.txt" \
  fa606a551587b89da1aebb0c0a6094d39c2f9c0f1ce0263f8e13f7793c53c686
fetch_hosted base-images.json "$tree/receipts/hosted-base-images.json" \
  cef5615808260002b8c940f1585425dae5f1432bcb68bc9b3b943c2c116957cf
fetch_hosted ffmpeg-source-29e619e767.tar.gz "$tree/ffmpeg/ffmpeg-source-29e619e767.tar.gz" \
  8e4699bcb6a311772e8f513cc72ce27328891b82a598b82e7cff3dfe580875a0

# 1. Exact recipe download + Dockerfile regeneration (makeimage.sh order).
docker pull "$base_image"
(
  cd upstream
  export GITHUB_REPOSITORY=btbn/ffmpeg-builds
  ./download.sh win64 lgpl-shared 8.1
  ./generate.sh win64 lgpl-shared 8.1
) 2>&1 | tee "$out/download.log"
cmp upstream/Dockerfile "$tree/receipts/hosted-dependency.Dockerfile"
cp upstream/Dockerfile "$tree/receipts/regenerated-dependency.Dockerfile"

grep -o 'scripts\.d/[^,]*\.sh,dst=/stage\.sh --mount=src=\.cache/downloads/[^,]*\.tar\.xz' upstream/Dockerfile \
  | sed 's/,dst=\/stage\.sh --mount=src=/ /' | sort -u > "$tree/receipts/stages.txt"
test "$(cut -d' ' -f2 "$tree/receipts/stages.txt" | sort -u | wc -l)" -eq "$(wc -l < "$tree/receipts/stages.txt")"

: > "$tree/receipts/download-caches.tsv"
while read -r script cache; do
  file="upstream/$cache"
  test -f "$file"
  stage="$(basename "$cache")"; stage="${stage%_*}"
  dest="$tree/dependencies/$stage"
  test ! -e "$dest"
  mkdir -p "$dest"
  tar -xJf "$file" -C "$dest"
  printf '%s\t%s\t%s\t%s\t%s\n' "$stage" "$script" "$(basename "$cache")" \
    "$(stat -c %s "$file")" "$(sha256sum "$file" | cut -d' ' -f1)" >> "$tree/receipts/download-caches.tsv"
done < "$tree/receipts/stages.txt"

# 2. Sources fetched by the build stages themselves (seen in the hosted build log).
for stage in 25-freetype 50-freetype; do
  git_ro -C "$tree/dependencies/$stage" submodule update --init subprojects/dlg
  test "$(git_ro -C "$tree/dependencies/$stage/subprojects/dlg" rev-parse HEAD)" = 395ccad2c1e0daae535c4d20bb0a3f2424648e17
done
git_ro -C "$tree/dependencies/50-kvazaar" submodule update --init greatest
test "$(git_ro -C "$tree/dependencies/50-kvazaar/greatest" rev-parse HEAD)" = 60e25ce7658ced49f03b52ac14c553cf6ea33de4
wrap="$tree/dependencies/50-dav1d/subprojects/checkasm.wrap"
grep -qx 'url = https://code.videolan.org/videolan/checkasm.git' "$wrap"
grep -qx 'revision = v1.2.0' "$wrap"
git clone --quiet --depth 1 --branch v1.2.0 https://code.videolan.org/videolan/checkasm.git \
  "$tree/dependencies/50-dav1d/subprojects/checkasm"

rav1e_cache="$(awk '$1 == "50-rav1e" {print $3}' "$tree/receipts/download-caches.tsv")"
rav1e_src="$work/rav1e-vendor-src"
rm -rf "$rav1e_src"; mkdir -p "$rav1e_src"
tar -xJf "upstream/.cache/downloads/$rav1e_cache" -C "$rav1e_src"
cp tools/ffmpeg-sdk/rav1e.Cargo.lock "$rav1e_src/Cargo.lock"
docker pull "$target_image"
docker run --rm -v "$rav1e_src:/src" -w /src "$target_image" \
  bash -c 'set -e; cargo --version > /src/cargo-version.txt; cargo vendor --locked --versioned-dirs /src/vendor > /src/cargo-vendor-config.toml'
sudo chown -R "$(id -u):$(id -g)" "$rav1e_src"
cmp "$rav1e_src/Cargo.lock" tools/ffmpeg-sdk/rav1e.Cargo.lock
mkdir -p "$tree/build-time/50-rav1e-cargo-vendor"
mv "$rav1e_src/vendor" "$rav1e_src/Cargo.lock" "$rav1e_src/cargo-vendor-config.toml" "$rav1e_src/cargo-version.txt" \
  "$tree/build-time/50-rav1e-cargo-vendor/"

# 3. Resolved VCS identities, then strip VCS metadata (non-deterministic bytes).
: > "$tree/receipts/git-identities.tsv"
while IFS= read -r -d '' g; do
  repo="$(dirname "$g")"
  printf '%s\t%s\t%s\n' "${repo#"$tree"/}" "$(git_ro -C "$repo" rev-parse HEAD)" \
    "$(git_ro -C "$repo" config --get remote.origin.url || true)" >> "$tree/receipts/git-identities.tsv"
done < <(find "$tree/dependencies" -name .git -prune -print0 | sort -z)
find "$tree/dependencies" \( -name .git -o -name .svn \) -prune -exec rm -rf {} +

# 4. Recipe tree (patched, with the rav1e lock overlay) and Syndocal build controls.
recipe_dir="$tree/recipe/FFmpeg-Builds-6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b-patched"
mkdir -p "$recipe_dir"
tar -C upstream --exclude=./.git --exclude=./.cache --exclude=./Dockerfile -cf - . | tar -C "$recipe_dir" -xf -
git fetch --quiet --depth=1 origin "$build_commit"
mkdir -p "$tree/syndocal/build-controls-$build_commit"
git archive "$build_commit" tools/ffmpeg-sdk .github/workflows/ffmpeg-lgpl-sdk.yml \
  | tar -x -C "$tree/syndocal/build-controls-$build_commit"
mkdir -p "$tree/syndocal/source-pack"
cp tools/ffmpeg-sdk/pack-corresponding-source.sh tools/ffmpeg-sdk/corresponding-source-manifest.py \
  tools/ffmpeg-sdk/component-licenses.json .github/workflows/ffmpeg-lgpl-sdk-source.yml "$tree/syndocal/source-pack/"
git rev-parse HEAD > "$tree/syndocal/source-pack/syndocal-commit.txt"

cat > "$tree/README.txt" <<'EOF'
Corresponding source for the Syndocal-hosted FFmpeg LGPL SDK
=============================================================
Binary: ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1.zip
  SHA-256 81587fa070a9fa2da3d8ff3e63beb9654729782bc8e9899ee951baf5fc5d9f1a
  built by GitHub Actions run 40366564270 of KTN44295080/Syndocal (commit
  7d0af757f73231fc0c6006d6c21c72c14cb4a31a), hosted on the prerelease
  ffmpeg-sdk-lgpl-n8.1.3-r40366564270.

Contents
  ffmpeg/        exact FFmpeg source archive (commit 29e619e767cde9045a75c29bc9a8278ae7b3a98b)
  recipe/        BtbN FFmpeg-Builds 6c9aec5fc9a72ec3abedd1fa84db141fa18cf52b with the
                 Syndocal patch applied and util/rav1e.Cargo.lock overlaid
  dependencies/  every download stage mounted by the build Dockerfile, extracted from
                 the recipe's own download step (directory = stage name); build-time
                 submodules/subprojects fetched by freetype, kvazaar and dav1d are
                 checked out in place. VCS metadata (.git, .svn) was removed; resolved
                 commits are in receipts/git-identities.tsv and MANIFEST.json.
  build-time/    rav1e Cargo dependencies vendored from the committed lock with the
                 build image's cargo (cargo vendor --locked --versioned-dirs).
  syndocal/      the Syndocal patch, lock, pin, build scripts and workflow at the build
                 commit, plus the scripts that produced this archive.
  receipts/      hosted and regenerated Dockerfile (identical), stage list, download
                 cache hashes, VCS identities, toolchain and base-image receipts.

Rebuild outline: apply nothing further; in recipe/<tree> run
  GITHUB_REPOSITORY=btbn/ffmpeg-builds ./makeimage.sh win64 lgpl-shared 8.1
  ./build.sh win64 lgpl-shared 8.1
with the digest-pinned images named in MANIFEST.json, or place each
dependencies/<stage> tree as that stage's /cache.tar.xz contents. Byte-identical
rebuilds have not been demonstrated (timestamps and toolchain nondeterminism).
The toolchain images are identified by digest only; GCC/MinGW/Rust runtime
sources are not included here (see MANIFEST.json "not_preserved").
EOF

# 5. Manifest (inside the archive) and cross-checks against the hosted build log.
python3 -I tools/ffmpeg-sdk/corresponding-source-manifest.py \
  --tree "$tree" --build-log "$work/hosted-build.log" \
  --licenses tools/ffmpeg-sdk/component-licenses.json \
  --output "$tree/MANIFEST.json"

# 6. Deterministic archive.
tar_args=(--sort=name --format=gnu --mtime="@$epoch" --owner=0 --group=0 --numeric-owner
  --mode='u+rwX,go+rX,go-w' -C "$work" -cf - "$name")
tar "${tar_args[@]}" | xz -6 -T4 --block-size=64MiB > "$out/$name.tar.xz"
tar_sha="$(xz -dc "$out/$name.tar.xz" | sha256sum | cut -d' ' -f1)"
tar_sha_again="$(tar "${tar_args[@]}" | sha256sum | cut -d' ' -f1)"
test "$tar_sha" = "$tar_sha_again"
archive_sha="$(sha256sum "$out/$name.tar.xz" | cut -d' ' -f1)"
archive_bytes="$(stat -c %s "$out/$name.tar.xz")"
python3 -I - "$tree/MANIFEST.json" "$out/$name.manifest.json" "$name.tar.xz" "$archive_bytes" "$archive_sha" "$tar_sha" "$(xz --version | sed -n 1p)" "$(tar --version | sed -n 1p)" <<'EOF'
import json, sys
src, dst, name, size, sha, tar_sha, xz_v, tar_v = sys.argv[1:]
m = json.load(open(src, encoding="utf-8"))
m["archive"] = {"name": name, "bytes": int(size), "sha256": sha, "uncompressed_tar_sha256": tar_sha,
                "tar_stream_reproduced_in_job": True, "xz": xz_v + "; -6 -T4 --block-size=64MiB", "tar": tar_v,
                "tar_options": "--sort=name --format=gnu --mtime=@1790726400 --owner=0 --group=0 --numeric-owner --mode=u+rwX,go+rX,go-w"}
open(dst, "w", encoding="utf-8", newline="\n").write(json.dumps(m, indent=2, ensure_ascii=False) + "\n")
EOF
if (( archive_bytes > 2000000000 )); then
  split -b 1900000000 -d -a 2 "$out/$name.tar.xz" "$out/$name.tar.xz.part"
  rm "$out/$name.tar.xz"
fi
( cd "$out" && sha256sum "$name".tar.xz* "$name.manifest.json" > SHA256SUMS-corresponding-source && cat SHA256SUMS-corresponding-source )
cp "$tree/receipts/"*.tsv "$tree/receipts/stages.txt" "$out/"
df -h
