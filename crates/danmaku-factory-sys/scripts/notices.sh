#!/usr/bin/env bash
# Writes THIRD-PARTY-NOTICES.txt from the retained upstream licences;
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

biliup uses Rust ports of the DanmakuFactory conversion core and PCRE2,
including its SLJIT-based JIT compiler. The algorithms are compiled from Rust
sources and statically linked into the executable, desktop app and Python
module. No original C engine or separate DanmakuFactory program is shipped.


DanmakuFactory
--------------

$(field Repository "$factory/UPSTREAM"), commit $(field Commit "$factory/UPSTREAM")
The XML/ASS core was ported to Rust. The fixes in
crates/danmaku-factory-sys/patches document the independent C compatibility
reference used to verify the port.
License: MIT

EOF
  cat "$factory/LICENSE"
  cat <<EOF


PCRE2
-----

$(field Repository "$pcre2/UPSTREAM"), $(field Tag "$pcre2/UPSTREAM") (commit $(field Commit "$pcre2/UPSTREAM"))
The complete 8-, 16- and 32-bit interfaces, POSIX facade and JIT compiler
were ported to Rust in crates/pcre2-rust. The algorithms and original
copyright notices are retained in the Rust source.
License: BSD-3-Clause WITH PCRE2-exception AND BSD-2-Clause (SLJIT)

EOF
  cat "$pcre2/LICENCE.md"
  printf '\n\nSLJIT\n-----\n\n'
  cat "$crate/../pcre2-rust/LICENSE-SLJIT"
} > "$crate/THIRD-PARTY-NOTICES.txt"
echo "wrote $crate/THIRD-PARTY-NOTICES.txt"
