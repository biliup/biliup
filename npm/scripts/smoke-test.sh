#!/usr/bin/env bash
# 把 prepare.mjs 的产物打成 tarball，在空目录里只装根包 @biliup/cli 和本机平台子包，
# 确认 `npx @biliup/cli --version` 与 `biliup --version` 都输出包版本：
#   npm/scripts/smoke-test.sh npm/dist [平台子包目录，默认 linux-x64-gnu]
set -euo pipefail

dist=$(cd "${1:?usage: smoke-test.sh <dist-dir> [platform-dir]}" && pwd)
platform=${2:-linux-x64-gnu}
name=$(node -p "require('$dist/biliup/package.json').name")
version=$(node -p "require('$dist/biliup/package.json').version")

# npm pack 的文件名：@scope/pkg → scope-pkg-<版本>.tgz
tgz_name() {
  node -p 'const p = require(process.argv[1]); p.name.replace(/^@/, "").replace("/", "-") + "-" + p.version + ".tgz"' "$1/package.json"
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/tgz" "$tmp/app"

while read -r dir; do
  [ -n "$dir" ] || continue
  (cd "$dist/$dir" && npm pack --silent --pack-destination "$tmp/tgz" >/dev/null)
done < "$dist/publish-order.txt"
ls -l "$tmp/tgz"

root_tgz=$(ls "$tmp/tgz/$(tgz_name "$dist/biliup")")
platform_tgz=$(ls "$tmp/tgz/$(tgz_name "$dist/$platform")")

cd "$tmp/app"
npm init -y >/dev/null
npm install --no-audit --no-fund "$root_tgz" "$platform_tgz"

for cmd in "$name" biliup; do
  out=$(npx --no-install "$cmd" --version)
  echo "npx $cmd --version: $out"
  [ "$out" = "biliup-cli $version" ] || { echo "::error::npx $cmd --version: expected 'biliup-cli $version', got '$out'"; exit 1; }
done
npx --no-install "$name" server --help >/dev/null
echo "smoke test passed: $name@$version ($platform)"
