#!/bin/sh
# Replaces the files of the site branch checkout in the current directory with
# the files in TREE, then commits and pushes them as MESSAGE. If the files do not
# change, it makes no commit. Cloudflare Pages deploys each commit on the branch.
# Usage: sh commit-site.sh TREE MESSAGE
set -eu

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 TREE MESSAGE" >&2
  exit 2
fi
tree=$1
message=$2

find . -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
cp -R "$tree/." .
git add --all
if git diff --cached --quiet; then
  echo "The site branch already holds this site."
  exit 0
fi
git -c user.name='github-actions[bot]' \
  -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  commit --quiet --message "$message"
git push origin HEAD:site
