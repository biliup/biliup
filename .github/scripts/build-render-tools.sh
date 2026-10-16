#!/usr/bin/env bash
set -euo pipefail

# Assembles the render tools directory shipped with the desktop installer and
# the Docker image, and used by CI: the CJK font shared by the browser preview
# and FFmpeg/libass, its licence, and the notices for the third-party code
# linked into biliup. Monochrome Noto Emoji is embedded in the biliup binary;
# its licence is shipped here. Danmaku XML -> ASS conversion is built into biliup
# (crates/danmaku-factory), so no converter program is built or shipped.
#
#   build-render-tools.sh [OUTPUT_DIR]
FONT_SHA256="2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b"

output="${1:-tauri-app/src-tauri/render-tools}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$output/fonts"

font="$tmp/NotoSansCJKsc-Regular.otf"
curl -fsSL --retry 3 \
  'https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf' \
  -o "$font"
echo "$FONT_SHA256  $font" | sha256sum -c -
cp "$font" "$output/fonts/NotoSansSC-Regular.otf"
curl -fsSL --retry 3 'https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/LICENSE' \
  -o "$output/fonts/OFL.txt"
cp "$repo/crates/danmaku-factory/THIRD-PARTY-NOTICES.txt" "$output/THIRD-PARTY-NOTICES.txt"
cat > "$output/README.txt" <<EOF
biliup render tools

Font: Noto Sans CJK SC Regular, used by the danmaku preview in the browser
and by FFmpeg/libass when danmaku are burnt into a video.
Source: https://github.com/notofonts/noto-cjk
Font license: SIL Open Font License 1.1 (fonts/OFL.txt)

Font: Noto Emoji Regular, embedded in biliup for libass and browser emoji fallback.
Source: https://github.com/google/fonts/tree/main/ofl/notoemoji
Font license: SIL Open Font License 1.1 (fonts/NotoEmoji-OFL.txt)

Danmaku XML to ASS conversion is built into biliup as native Rust, derived
from DanmakuFactory's layout; see THIRD-PARTY-NOTICES.txt.
EOF
cp "$repo/crates/biliup-cli/assets/NotoEmoji-OFL.txt" "$output/fonts/NotoEmoji-OFL.txt"
echo "render tools installed in $output"
