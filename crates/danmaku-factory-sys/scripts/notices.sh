#!/usr/bin/env bash
# Writes THIRD-PARTY-NOTICES.txt from the vendored licences. vendor.sh runs it;
# packaging copies the result next to every biliup executable.
set -euo pipefail

crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
field() { sed -n "s/^$1: //p" "$2"; }
factory="$crate/vendor/DanmakuFactory"
pcre2="$crate/vendor/pcre2"

{
  cat <<EOF
Third-party software linked into biliup
=======================================

biliup converts recorded danmaku (XML) to ASS subtitles with the conversion
core of DanmakuFactory and the PCRE2 library. Both are compiled from source
and statically linked into the biliup executable, the desktop app and the
Python module; no separate DanmakuFactory program is shipped.


DanmakuFactory
--------------

$(field Repository "$factory/UPSTREAM"), commit $(field Commit "$factory/UPSTREAM")
Only the XML/ASS conversion files are used, with the fixes in
crates/danmaku-factory-sys/patches of the biliup source.
License: MIT

EOF
  cat "$factory/LICENSE"
  cat <<EOF


PCRE2
-----

$(field Repository "$pcre2/UPSTREAM"), $(field Tag "$pcre2/UPSTREAM") (commit $(field Commit "$pcre2/UPSTREAM"))
The 8-bit library, unmodified, without the JIT compiler.
License: BSD-3-Clause WITH PCRE2-exception

EOF
  cat "$pcre2/LICENCE.md"
} > "$crate/THIRD-PARTY-NOTICES.txt"
echo "wrote $crate/THIRD-PARTY-NOTICES.txt"
