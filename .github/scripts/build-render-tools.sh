#!/usr/bin/env bash
set -euo pipefail

# Build DanmakuFactory from this pinned upstream source.
DANMAKU_FACTORY_COMMIT="3813e93b13b087e95901d1822baf9c3540e3f3f6"
FONT_SHA256="2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b"

platform="${1:-auto}"
output="${2:-tauri-app/src-tauri/render-tools}"
case "$platform" in
  auto)
    case "${RUNNER_OS:-$(uname -s)}" in
      Windows*) platform=windows ;;
      Linux*) platform=linux ;;
      *) echo "unsupported render-tools platform: ${RUNNER_OS:-$(uname -s)}" >&2; exit 2 ;;
    esac
    ;;
  windows|linux) ;;
  *) echo "usage: $0 [windows|linux] [output-dir]" >&2; exit 2 ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$output/fonts"
git clone --quiet https://github.com/hihkm/DanmakuFactory.git "$tmp/DanmakuFactory"
git -C "$tmp/DanmakuFactory" checkout --quiet "$DANMAKU_FACTORY_COMMIT"
if [[ "$platform" == linux ]]; then
  mapfile -d '' sources < <(find "$tmp/DanmakuFactory/src" -type f -name '*.c' -print0)
  read -r -a pcre_flags <<< "$(pkg-config --cflags --libs libpcre2-8)"
  cc -std=gnu11 -O2 "${sources[@]}" "${pcre_flags[@]}" -lm -o "$tmp/DanmakuFactory-cli"
  factory="$tmp/DanmakuFactory-cli"
else
  xmake -C "$tmp/DanmakuFactory" f -y -m release
  xmake -C "$tmp/DanmakuFactory" build -y cli
  factory="$(find "$tmp/DanmakuFactory/build" -type f \( -iname 'DanmakuFactory.exe' -o -iname 'DanmakuFactory' -o -name 'cli' \) -print -quit)"
fi
[[ -n "$factory" && -f "$factory" ]] || { echo "DanmakuFactory executable missing from archive" >&2; exit 1; }
target="$output/DanmakuFactory"
[[ "$platform" == windows ]] && target+=".exe"
cp "$factory" "$target"
chmod +x "$output/DanmakuFactory"* 2>/dev/null || true

font="$tmp/NotoSansCJKsc-Regular.otf"
curl -fsSL --retry 3 \
  'https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf' \
  -o "$font"
echo "$FONT_SHA256  $font" | sha256sum -c -
cp "$font" "$output/fonts/NotoSansSC-Regular.otf"
cat > "$output/README.txt" <<EOF
biliup render tools

DanmakuFactory: hihkm/DanmakuFactory source commit ${DANMAKU_FACTORY_COMMIT}
Upstream source commit: https://github.com/hihkm/DanmakuFactory/tree/${DANMAKU_FACTORY_COMMIT}
DanmakuFactory is licensed under MIT; see the upstream LICENSE.

Font: Noto Sans CJK SC Regular
Source: https://github.com/notofonts/noto-cjk
Font license: SIL Open Font License 1.1 (OFL-1.1)
EOF
curl -fsSL 'https://raw.githubusercontent.com/hihkm/DanmakuFactory/3813e93b13b087e95901d1822baf9c3540e3f3f6/LICENSE' \
  -o "$output/DANMAKUFACTORY-LICENSE.txt"
curl -fsSL 'https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/LICENSE' \
  -o "$output/fonts/OFL.txt"
echo "render tools installed in $output"
