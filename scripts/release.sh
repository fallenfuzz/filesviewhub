#!/usr/bin/env bash
# Makes the packages from WSL, only from a committed app that passes the check gate:
#   bash scripts/release.sh            the Windows zip, then the Linux AppImage
#   bash scripts/release.sh windows    only the zip
#   bash scripts/release.sh linux      only the AppImage
# 1. It stops when WSL git sees uncommitted changes in the app's folder. A package named after a
#    commit must hold that commit only. Windows git is
#    not asked: it sees line-ending and file-mode differences in files that are clean.
# 2. It runs check.mjs (with --windows for the zip, --linux for the AppImage).
# 3. It runs `npm run package` on Windows through cmd.exe, then `linux.sh package`.
# 4. It checks again that HEAD did not move and the app stayed clean while it built, and moves the
#    older builds in release/ to release/archive/.
# Tag HEAD and push the tag first, by hand, as its message is the release note: the packages link
# to the app's source at that tag, and notices.mjs stops a build (FVH_RELEASE=1) without it.
set -euo pipefail
export FVH_RELEASE=1
app=$(cd "$(dirname "$0")/.." && pwd)
cd "$app"
grep -qi microsoft /proc/version 2>/dev/null || { echo 'release.sh runs from WSL.'; exit 2; }
case "${1:-both}" in
  both) targets=(windows linux) ;;
  windows|linux) targets=("$1") ;;
  *) echo 'usage: release.sh [windows | linux]'; exit 2 ;;
esac

clean() {
  local dirty
  dirty=$(git status --porcelain -- .)
  [ -z "$dirty" ] && return 0
  printf 'The app has changes not committed (WSL git):\n%s\n' "$dirty"
  return 1
}
clean || { echo 'Commit them first, or wait for the task that owns them.'; exit 1; }
head=$(git rev-parse HEAD)
build=$(( $(git rev-list --count HEAD) + $(sed -n 's/^const EARLIER_BUILDS: u32 = \([0-9]*\);/\1/p' src-tauri/build.rs) ))
echo "release: build $build, commit $(git rev-parse --short=12 HEAD)"

for t in "${targets[@]}"; do
  if [ "$t" = windows ]; then node scripts/check.mjs --windows; else node scripts/check.mjs --linux; fi
done

for t in "${targets[@]}"; do
  if [ "$t" = windows ]; then
    win=$(wslpath -w "$app")
    (cd /mnt/c && /mnt/c/Windows/System32/cmd.exe /d /c \
      "set \"PATH=%USERPROFILE%\\.cargo\\bin;%PATH%\" && set \"FVH_RELEASE=1\" && cd /d $win && npm run package")
  else
    bash scripts/linux.sh package
  fi
done

# A commit or an edit made by another task during the build may be in the packages.
if [ "$(git rev-parse HEAD)" != "$head" ] || ! clean; then
  echo "HEAD or the app changed while the packages were built: they may not match build $build. Run release.sh again."
  exit 1
fi

mkdir -p release/archive
for f in release/*; do
  case "$f" in release/archive|*-build-"$build"-*) ;; *) mv -n -- "$f" release/archive/ ;; esac
done
echo "release: build $build made; release/ holds:"
ls -1 release | grep -v '^archive$'
