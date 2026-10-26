#!/bin/bash
set -euo pipefail
root="$PWD"
cd upstream
retain_diagnostics() {
  [[ ! -f Dockerfile ]] || cp Dockerfile "$root/candidate/dependency.Dockerfile"
  [[ ! -f ffbuild/ffmpeg/ffbuild/config.log ]] || cp ffbuild/ffmpeg/ffbuild/config.log "$root/candidate/ffmpeg-config.log"
}
trap retain_diagnostics EXIT
# Only the two fixed toolchain/download images are reused. Dependency libraries
# are rebuilt from the patched recipe; an existing full SDK image is never used.
docker pull ghcr.io/btbn/ffmpeg-builds/base@sha256:91eaaaf32f3f98464e99faf1062fab8ab99a23d0b499c72b51a02ec9137b06a0
docker pull ghcr.io/btbn/ffmpeg-builds/base-win64@sha256:8a3d189e5b615c89e1a9fe2be705fa17173d8f6ea0a955b25c7c28041cde36c7
docker image inspect \
  ghcr.io/btbn/ffmpeg-builds/base@sha256:91eaaaf32f3f98464e99faf1062fab8ab99a23d0b499c72b51a02ec9137b06a0 \
  ghcr.io/btbn/ffmpeg-builds/base-win64@sha256:8a3d189e5b615c89e1a9fe2be705fa17173d8f6ea0a955b25c7c28041cde36c7 \
  > "$root/candidate/base-images.json"
export GITHUB_REPOSITORY=btbn/ffmpeg-builds
bash ./makeimage.sh win64 lgpl-shared 8.1
cp Dockerfile "$root/candidate/dependency.Dockerfile"
if grep -Ein 'chromaprint|fftw' Dockerfile | grep -v -- '--disable-chromaprint'; then
  echo 'Forbidden dependency in generated Dockerfile' >&2
  exit 1
fi
docker image inspect ghcr.io/btbn/ffmpeg-builds/win64-lgpl-shared-8.1:latest \
  > "$root/candidate/dependency-image.json"
docker run --rm ghcr.io/btbn/ffmpeg-builds/win64-lgpl-shared-8.1:latest \
  cat /opt/ffbuild/share/syndocal-source-locks/rav1e-Cargo.lock > "$root/candidate/built-rav1e.Cargo.lock"
cmp "$root/candidate/rav1e.Cargo.lock" "$root/candidate/built-rav1e.Cargo.lock"
docker run --rm ghcr.io/btbn/ffmpeg-builds/win64-lgpl-shared-8.1:latest \
  bash -c 'set -e; x86_64-w64-mingw32-gcc --version; rustc --version; cargo --version; test -z "$(find /opt/ffbuild -iname "*fftw*" -o -iname "*chromaprint*")"' \
  > "$root/candidate/toolchain.txt"
bash ./build.sh win64 lgpl-shared 8.1
test "$(git -C ffbuild/ffmpeg rev-parse HEAD)" = 29e619e767cde9045a75c29bc9a8278ae7b3a98b
git -C ffbuild/ffmpeg describe --tags --always > "$root/candidate/source-version.txt"
cp ffbuild/ffmpeg/ffbuild/config.log "$root/candidate/ffmpeg-config.log"
cp artifacts/*.zip "$root/candidate/"
sha256sum "$root"/candidate/*.zip "$root"/candidate/*tar.gz > "$root/candidate/archive-sha256.txt"
df -h
