biliup render fonts

Noto Sans CJK SC Regular and monochrome Noto Emoji are external assets shared
by browser preview and FFmpeg/libass. They are not embedded in the executable
or Python extension. Keep this directory beside the executable, or configure
BILIUP_RENDER_TOOLS_DIR to point to it. Desktop installers and Docker images ship the same files. The Python API
package does not include fonts or the rendering service.

Noto Sans CJK SC: https://github.com/notofonts/noto-cjk
SIL Open Font License 1.1: fonts/OFL.txt
Noto Emoji: https://github.com/google/fonts/tree/main/ofl/notoemoji
SIL Open Font License 1.1: fonts/NotoEmoji-OFL.txt

The native Rust XML-to-ASS converter is derived from DanmakuFactory.
See THIRD-PARTY-NOTICES.txt for attribution.
