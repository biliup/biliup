#!/usr/bin/env bash
# 按 prepare.mjs 写下的顺序发布（先平台子包，最后根包）：
#   npm/scripts/publish.sh npm/dist [npm publish 的额外参数，如 --provenance / --dry-run]
# registry 上已有同版本的包会跳过，所以中途失败后可以直接重跑。
set -euo pipefail

dist=${1:?usage: publish.sh <dist-dir> [npm publish args...]}
shift

while read -r dir; do
  [ -n "$dir" ] || continue
  pkg="$dist/$dir"
  name=$(node -p "require('./$pkg/package.json').name")
  version=$(node -p "require('./$pkg/package.json').version")
  if [ "$(npm view "$name@$version" version 2>/dev/null)" = "$version" ]; then
    echo "skip $name@$version: already published"
    continue
  fi
  echo "publish $name@$version"
  (cd "$pkg" && npm publish --access public "$@")
done < "$dist/publish-order.txt"
