#!/usr/bin/env bash
# 按 prepare.mjs 写下的顺序发布（先 7 个平台子包，最后根包 @biliup/cli）：
#   npm/scripts/publish.sh npm/dist [npm publish 的额外参数，如 --provenance / --dry-run]
# registry 上已有同版本的包会跳过，所以中途失败后可以直接重跑。
# 补发旧版本（registry 上 latest 比它新）时改打 backport 标签，不把 latest 拉回旧版本。
# 任何一个包发布失败都立即停止并以非 0 退出，日志里写明是哪个包、npm 报了什么；子包没发全时不发根包。
set -euo pipefail

dist=$(cd "${1:?usage: publish.sh <dist-dir> [npm publish args...]}" && pwd)
shift

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 取 npm 输出里的错误码与原因（前两行有内容的 `npm error`），拼成一行
npm_error() {
  grep '^npm error' "$1" | grep -Ev '^npm error (syscall|errno) |A complete log of this run' | head -n 2 | sed 's/^npm error //' | paste -sd ' ' -
}

# GitHub 注解的消息里 % 与换行要转义
annotate() {
  local msg=${2//%/%25}
  echo "::error title=$1::${msg//$'\n'/%0A}"
}

# 0：registry 上已有 $1@$2；1：包或该版本不存在（E404）。查询本身出错时直接失败，不盲目发布
is_published() {
  local out
  if out=$(npm view "$1@$2" version 2>"$tmp/view.log"); then
    [ "$out" = "$2" ]
    return
  fi
  grep -q '^npm error code E404' "$tmp/view.log" && return 1
  annotate 'npm view failed' "查询 $1@$2 失败，未继续发布：$(npm_error "$tmp/view.log")"
  exit 1
}

# npm publish 默认给刚发的版本打 latest；registry 上 latest 更新时改用 backport
dist_tag() {
  local latest
  latest=$(npm view "$1" dist-tags.latest 2>/dev/null) || true
  if [ -n "$latest" ] && [ "$(printf '%s\n%s\n' "$latest" "$2" | sort -V | tail -n 1)" != "$2" ]; then
    echo backport
  else
    echo latest
  fi
}

mapfile -t dirs < <(grep -v '^$' "$dist/publish-order.txt")
published=()
skipped=()
for i in "${!dirs[@]}"; do
  pkg=$dist/${dirs[$i]}
  name=$(node -p 'require(process.argv[1]).name' "$pkg/package.json")
  version=$(node -p 'require(process.argv[1]).version' "$pkg/package.json")

  if is_published "$name" "$version"; then
    echo "skip $name@$version: already published"
    skipped+=("$name@$version")
    continue
  fi

  tag=$(dist_tag "$name" "$version")
  echo "publish $name@$version (dist-tag $tag)"
  if (cd "$pkg" && npm publish --access public --tag "$tag" "$@") 2>&1 | tee "$tmp/publish.log"; then
    published+=("$name@$version")
    continue
  fi

  # 发布前的查询可能读到 registry 缓存的旧数据，失败后再查一次：已在 registry 上就等同跳过
  if is_published "$name" "$version"; then
    echo "skip $name@$version: already published (npm publish failed, but the version is on the registry)"
    skipped+=("$name@$version")
    continue
  fi

  annotate "npm publish failed" "$name@$version 发布失败：$(npm_error "$tmp/publish.log")"
  echo "已发布：${published[*]:-无}；已存在跳过：${skipped[*]:-无}"
  rest=$(for d in "${dirs[@]:$((i + 1))}"; do node -p 'require(process.argv[1]).name' "$dist/$d/package.json"; done | paste -sd ' ' -)
  [ -z "$rest" ] || echo "未尝试（前面的包失败后停止）：$rest"
  exit 1
done

echo "已发布：${published[*]:-无}；已存在跳过：${skipped[*]:-无}"
