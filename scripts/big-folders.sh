#!/usr/bin/env bash
# Makes the two large folders the speed checks use, under WSL's /tmp (stalls.mjs measures them):
#   big-folders.sh files [N]     /tmp/fvh-big-files: N small files in one folder (150000)
#   big-folders.sh works [N]     /tmp/fvh-big-works: N folders x 3 x 100 files (2000, so 600,000
#                                Works as a Books Section)
# Each starts from an empty folder. Delete them with rm -rf when done.
set -euo pipefail
case "${1:-}" in
  files) dir=/tmp/fvh-big-files; n=${2:-150000}
    rm -rf "$dir"; mkdir -p "$dir"; cd "$dir"
    seq 1 "$n" | sed 's/$/.txt/' | xargs sh -c 'for f; do echo x > "$f"; done' _ ;;
  works) dir=/tmp/fvh-big-works; n=${2:-2000}
    rm -rf "$dir"; mkdir -p "$dir"; cd "$dir"
    for a in $(seq -f 'A%04g' 0 $((n - 1))); do
      for s in S1 S2 S3; do mkdir -p "$a/$s"; (cd "$a/$s" && for i in $(seq -f '%03g' 0 99); do echo x > "$i.txt"; done); done
    done ;;
  *) echo 'usage: big-folders.sh files [N] | works [N]'; exit 2 ;;
esac
echo "$dir: $(find "$dir" -type f | wc -l) files"
