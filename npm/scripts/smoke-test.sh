#!/usr/bin/env bash
# 把 prepare.mjs 的产物打成 tarball，在空目录里只装根包和本机平台子包，确认 `biliup --version` 与包版本一致：
#   npm/scripts/smoke-test.sh npm/dist [平台子包目录，默认 linux-x64-gnu]
set -euo pipefail

dist=$(cd "${1:?usage: smoke-test.sh <dist-dir> [platform-dir]}" && pwd)
platform=${2:-linux-x64-gnu}
version=$(node -p "require('$dist/biliup/package.json').version")

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/tgz" "$tmp/app"

while read -r dir; do
  [ -n "$dir" ] || continue
  (cd "$dist/$dir" && npm pack --silent --pack-destination "$tmp/tgz" >/dev/null)
done < "$dist/publish-order.txt"
ls -l "$tmp/tgz"

root_tgz=$(ls "$tmp"/tgz/biliup-"$version".tgz)
platform_tgz=$(ls "$tmp"/tgz/biliup-"$platform"-"$version".tgz)

cd "$tmp/app"
npm init -y >/dev/null
npm install --no-audit --no-fund "$root_tgz" "$platform_tgz"

out=$(npx --no-install biliup --version)
echo "biliup --version: $out"
[ "$out" = "biliup-cli $version" ] || { echo "::error::expected 'biliup-cli $version', got '$out'"; exit 1; }
npx --no-install biliup server --help >/dev/null
echo "smoke test passed: biliup $version ($platform)"
