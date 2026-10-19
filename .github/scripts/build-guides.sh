#!/bin/sh
# Builds the installation guide of each camera in each language as
# OUT/<camera>/<language>.pdf. OUT must not exist.
# GRKIT is the grkit binary, and TOOLS is the directory that
# install-guide-tools.sh fills.
# Usage: sh .github/scripts/build-guides.sh GRKIT TOOLS OUT
set -eu

if [ "$#" -ne 3 ]; then
  echo "Usage: $0 GRKIT TOOLS OUT" >&2
  exit 2
fi
grkit=$1
tools=$2
out=$3
if [ -e "$out" ]; then
  echo "$out already exists." >&2
  exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$grkit" guides --out "$work/source" >/dev/null
for source in "$work"/source/*/*; do
  camera=$(basename "$(dirname "$source")")
  language=$(basename "$source")
  mkdir -p "$out/$camera"
  "$tools/bin/typst" compile --root "$source" --font-path "$tools/fonts" \
    --ignore-system-fonts "$source/guide.typ" "$out/$camera/$language.pdf"
done
