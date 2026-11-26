#!/usr/bin/env bash
# Builds, tests and runs the Linux app from WSL, from this one checkout. The npm packages and the
# tools differ per system, so the Linux ones live in a build folder of their own on WSL's disk,
# $FVH_LINUX_STAGE (default ~/.cache/fvh-linux), never in this checkout: each command first copies
# the app's folder there (rsync; node_modules, tools, target, release and .scratch are left out both ways).
# The build folder is disposable, like target/: deleting it costs one `npm ci` and one tools fetch.
#   linux.sh package          the AppImage, put in release/ of this checkout
#   linux.sh test             npm test and cargo test --lib, as on Windows
#   linux.sh clippy           cargo clippy --all-targets
#   linux.sh run <command>    any command in the build folder's copy (`run npm run tauri dev` is the dev app)
#   linux.sh sync             only the copy (repeat it while the dev app runs to carry edits there)
# Each command also runs `npm ci` when package-lock.json changed and `npm run fetch-tools` when
# fetch-tools.mjs changed (or no tools are there yet).
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)            # the app's folder in this checkout
stage=${FVH_LINUX_STAGE:-$HOME/.cache/fvh-linux}
app=$stage/app
export PATH=$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin   # a pkg-config from another package manager can miss the system's webkit2gtk
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HOME/.cache/fvh-linux-target}
# The build folder has no .git: build.rs takes the build number from here, package.mjs the checkout.
export FVH_APP=$here
FVH_BUILD=$(( $(git -C "$here" rev-list --count HEAD) + $(sed -n 's/^const EARLIER_BUILDS: u32 = \([0-9]*\);/\1/p' "$here/src-tauri/build.rs") ))
export FVH_BUILD

sync() {
  mkdir -p "$app"
  rsync -rlt --delete \
    --exclude=/node_modules/ --exclude=/tools/ --exclude=/dist/ --exclude=/release/ \
    --exclude=/src-tauri/target/ --exclude=/src-tauri/gen/ --exclude=/.scratch/ \
    "$here/" "$app/"
  # Tests read and write scratch folders in .scratch/, as they do in the checkout.
  [ -L "$app/.scratch" ] || ln -s "$here/.scratch" "$app/.scratch"
}

# Runs $2 in the build folder when the hash of file $1 differs from the one kept after its last run.
when_changed() {
  local file=$1 mark=$stage/.$(basename "$1").sha256 now
  now=$(sha256sum "$app/$file" | cut -d' ' -f1)
  if [ "$(cat "$mark" 2>/dev/null)" != "$now" ] || [ ! -e "$app/$3" ]; then
    (cd "$app" && eval "$2")
    echo "$now" >"$mark"
  fi
}

prepare() {
  sync
  when_changed package-lock.json 'npm ci' node_modules
  when_changed scripts/fetch-tools.mjs 'npm run fetch-tools' tools/tools.json
}

case "${1:-}" in
  sync) sync ;;
  package)
    prepare
    out=$(cd "$app" && npm run package | tee -a /dev/stderr | tail -1)
    [ -f "$app/$out" ] || { echo "no AppImage made"; exit 1; }
    mkdir -p "$here/release"
    mv "$app/$out" "$here/$out"
    echo "$here/$out" ;;
  test)
    prepare
    (cd "$app" && npm test)
    (cd "$app/src-tauri" && cargo test --lib) ;;
  clippy)
    prepare
    (cd "$app/src-tauri" && cargo clippy --all-targets) ;;
  run)
    shift; [ $# -gt 0 ] || { echo 'usage: linux.sh run <command>'; exit 2; }
    prepare
    cd "$app" && exec "$@" ;;
  *) echo 'usage: linux.sh package | test | clippy | run <command> | sync'; exit 2 ;;
esac
