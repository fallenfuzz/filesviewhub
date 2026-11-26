#!/usr/bin/env bash
# The git pre-push hook: a push waits for the check gate (check.mjs) and stops if it fails, then
# warns about any security advisory with a fix not yet taken (updates.mjs --advisories).
# Install once per checkout, from the repo root:
#   printf '#!/bin/sh\nexec bash "%s/scripts/pre-push.sh"\n' "$PWD" > "$(git rev-parse --git-path hooks)/pre-push" && chmod +x "$(git rev-parse --git-path hooks)/pre-push"
# It checks the working tree, so uncommitted edits count too.
cd "$(dirname "$0")/.." || exit 1
echo 'pre-push: running node scripts/check.mjs'
node scripts/check.mjs || exit 1
# Security advisories warn and never stop the push; offline, the check is skipped.
if ! out=$(timeout 60 node scripts/updates.mjs --advisories 2>&1); then
  printf '%s\n' "$out" | grep -E 'fixed in|advisor|npm audit|vulnerab' | grep -v '\[waiting' | grep -v 'no advisories'
  echo 'pre-push: WARNING, an advisory has a fix to take (node scripts/updates.mjs); pushing anyway'
fi
exit 0
