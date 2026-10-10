#!/usr/bin/env bash
# Re-creates vendor/ from the pinned upstream sources. Run it by hand after
# changing a pin or a patch; Cargo builds only ever use the committed files.
#
#   crates/danmaku-factory-sys/scripts/vendor.sh
#
# Needs git and network access. The result must be committed.
set -euo pipefail

DANMAKU_FACTORY_REPO="https://github.com/hihkm/DanmakuFactory.git"
DANMAKU_FACTORY_COMMIT="3813e93b13b087e95901d1822baf9c3540e3f3f6"
PCRE2_REPO="https://github.com/PCRE2Project/pcre2.git"
PCRE2_TAG="pcre2-10.49"
PCRE2_COMMIT="6f9d7c1373262c541324a16a358785b33ef116cf"

crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Only the XML -> ASS conversion core. main.c (the CLI), Config.c (config
# files), JsonFile.c and TemplateFile/ are not needed and not built.
factory_files=(
  LICENSE
  src/CDanmakuFactory.h
  src/XmlFile.c
  src/AssFile/AssFile.c
  src/AssFile/AssFile.h
  src/AssFile/AssStringProcessing.c
  src/AssFile/AssStringProcessing.h
  src/Config/Config.h
  src/Define/CLIDef.h
  src/Define/DanmakuDef.h
  src/Define/Status.h
  src/FileUtil/FileUtil.c
  src/FileUtil/FileUtil.h
  src/List/DanmakuFactoryList.c
  src/List/DanmakuFactoryList.h
  src/String/DanmakuFactoryString.c
  src/String/DanmakuFactoryString.h
  src/TemplateFile/TemplateFile.h
)

# The 8-bit library without JIT, pcre2posix, pcre2grep or pcre2test; the same
# source list as PCRE2's CMakeLists.txt.
pcre2_files=(
  LICENCE.md
  src/pcre2_auto_possess.c
  src/pcre2_chkdint.c
  src/pcre2_compile.c
  src/pcre2_compile.h
  src/pcre2_compile_cgroup.c
  src/pcre2_compile_class.c
  src/pcre2_config.c
  src/pcre2_context.c
  src/pcre2_convert.c
  src/pcre2_dfa_match.c
  src/pcre2_error.c
  src/pcre2_extuni.c
  src/pcre2_find_bracket.c
  src/pcre2_internal.h
  src/pcre2_intmodedep.h
  src/pcre2_jit_compile.c
  src/pcre2_jit_match_inc.h
  src/pcre2_jit_misc_inc.h
  src/pcre2_maketables.c
  src/pcre2_match.c
  src/pcre2_match_data.c
  src/pcre2_match_next.c
  src/pcre2_newline.c
  src/pcre2_ord2utf.c
  src/pcre2_pattern_info.c
  src/pcre2_script_run.c
  src/pcre2_serialize.c
  src/pcre2_string_utils.c
  src/pcre2_study.c
  src/pcre2_substitute.c
  src/pcre2_substring.c
  src/pcre2_tables.c
  src/pcre2_ucd.c
  src/pcre2_ucp.h
  src/pcre2_ucptables_inc.h
  src/pcre2_util.h
  src/pcre2_valid_utf.c
  src/pcre2_xclass.c
)

git clone --quiet "$DANMAKU_FACTORY_REPO" "$tmp/DanmakuFactory"
git -C "$tmp/DanmakuFactory" checkout --quiet "$DANMAKU_FACTORY_COMMIT"
for patch in "$crate"/patches/*.patch; do
  git -C "$tmp/DanmakuFactory" apply --whitespace=nowarn "$patch"
done
rm -rf "$crate/vendor/DanmakuFactory"
for file in "${factory_files[@]}"; do
  install -D -m 644 "$tmp/DanmakuFactory/$file" "$crate/vendor/DanmakuFactory/$file"
done
cat > "$crate/vendor/DanmakuFactory/UPSTREAM" <<EOF
Repository: $DANMAKU_FACTORY_REPO
Commit: $DANMAKU_FACTORY_COMMIT
Patches: crates/danmaku-factory-sys/patches/*.patch (applied)
License: MIT (LICENSE)
EOF

git clone --quiet --depth 1 --branch "$PCRE2_TAG" "$PCRE2_REPO" "$tmp/pcre2"
actual="$(git -C "$tmp/pcre2" rev-parse HEAD)"
[[ "$actual" == "$PCRE2_COMMIT" ]] || { echo "$PCRE2_TAG is $actual, expected $PCRE2_COMMIT" >&2; exit 1; }
rm -rf "$crate/vendor/pcre2"
for file in "${pcre2_files[@]}"; do
  install -D -m 644 "$tmp/pcre2/$file" "$crate/vendor/pcre2/$file"
done
# The generic files are the ones PCRE2 documents for builds without configure.
install -m 644 "$tmp/pcre2/src/pcre2.h.generic" "$crate/vendor/pcre2/src/pcre2.h"
install -m 644 "$tmp/pcre2/src/config.h.generic" "$crate/vendor/pcre2/src/config.h"
install -m 644 "$tmp/pcre2/src/pcre2_chartables.c.dist" "$crate/vendor/pcre2/src/pcre2_chartables.c"
cat > "$crate/vendor/pcre2/UPSTREAM" <<EOF
Repository: $PCRE2_REPO
Tag: $PCRE2_TAG
Commit: $PCRE2_COMMIT
Unmodified. pcre2.h, config.h and pcre2_chartables.c are copies of
pcre2.h.generic, config.h.generic and pcre2_chartables.c.dist.
License: BSD-3-Clause with the PCRE2 exception (LICENCE.md)
EOF

bash "$crate/scripts/notices.sh"
