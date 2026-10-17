biliup render fonts

Noto Sans CJK SC Regular and monochrome Noto Emoji are external assets shared
by the browser preview and FFmpeg/libass. They are not embedded in the
executable or the Python extension. Keep this directory beside the executable,
or set the render_fonts_dir setting to point at it. Desktop installers and
Docker images ship the same files. The Python API package does not include
fonts or the rendering service.

NotoSansSC-Regular.otf is not committed to the repository. Fetch it into this
directory with the same pinned source and checksum the builds use:

  curl -fsSL -o fonts/NotoSansSC-Regular.otf 'https://raw.githubusercontent.com/notofonts/noto-cjk/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf'
  echo '2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b  fonts/NotoSansSC-Regular.otf' | sha256sum -c -

The build steps in .github/workflows/test.yml, release.yml and
desktop-publish.yml, and the Dockerfile, run exactly this before they test or
package, so a checkout without the font still builds and only skips the tests
that need real glyphs. NotoEmoji.ttf stays in the repository: it is small and
upstream publishes no byte-identical download of it.

Noto Sans CJK SC: https://github.com/notofonts/noto-cjk
SIL Open Font License 1.1: fonts/OFL.txt
Noto Emoji: https://github.com/google/fonts/tree/main/ofl/notoemoji
SIL Open Font License 1.1: fonts/NotoEmoji-OFL.txt

The native Rust XML-to-ASS converter is derived from DanmakuFactory.
See THIRD-PARTY-NOTICES.txt for attribution.
