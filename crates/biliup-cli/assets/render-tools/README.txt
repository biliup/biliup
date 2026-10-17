biliup render fonts

Noto Sans CJK SC Regular and Noto Emoji are external assets shared by the
browser preview and FFmpeg/libass. They are not embedded in the executable or
the Python extension. Keep this directory beside the executable, or set the
render_fonts_dir setting to point at it. Desktop installers and Docker images
ship the same files. The Python API package does not include fonts or the
rendering service.

Neither font is committed. Fetch both into this directory with the same pinned
sources and checksums the builds use:

  curl -fsSL -o fonts/NotoSansSC-Regular.otf 'https://raw.githubusercontent.com/notofonts/noto-cjk/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf'
  echo '2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b  fonts/NotoSansSC-Regular.otf' | sha256sum -c -
  curl -fsSL -o fonts/NotoEmoji.ttf 'https://raw.githubusercontent.com/google/fonts/47d54ca1ebb14fd864695f1d39dc734f39017c33/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf'
  echo 'de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551  fonts/NotoEmoji.ttf' | sha256sum -c -

The build steps in .github/workflows/test.yml, release.yml and
desktop-publish.yml, and the Dockerfile, run exactly this before they test or
package, so a checkout without the fonts still builds and only skips the tests
that need real glyphs. Upstream ships the emoji file as NotoEmoji[wght].ttf;
biliup expects NotoEmoji.ttf, which is only a rename.

Noto Sans CJK SC: https://github.com/notofonts/noto-cjk
SIL Open Font License 1.1: fonts/OFL.txt
Noto Emoji: https://github.com/google/fonts/tree/main/ofl/notoemoji
SIL Open Font License 1.1: fonts/NotoEmoji-OFL.txt

The native Rust XML-to-ASS converter is derived from DanmakuFactory.
See THIRD-PARTY-NOTICES.txt for attribution.
