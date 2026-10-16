#!/usr/bin/env bash
# Builds the C interface and the vendored core with AddressSanitizer,
# UndefinedBehaviorSanitizer and LeakSanitizer and runs df_ffi_test.c, then
# checks that the core exports no unprefixed global symbol.
#
#   crates/danmaku-factory-sys/tests/c/check.sh     (needs clang)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/../.." && pwd)"
golden="$(cd "$crate/../danmaku-factory/tests/golden" && pwd)"
core="$crate/vendor/DanmakuFactory/src"
pcre2="$crate/vendor/pcre2/src"
CC="${CC:-clang}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

# The same language flags as build.rs.
common=(-std=gnu11 -fsigned-char -ffp-contract=off -g -O1 -fno-omit-frame-pointer
  -fsanitize=address,undefined -fno-sanitize-recover=all)
pcre2_defines=(-DHAVE_CONFIG_H -DPCRE2_CODE_UNIT_WIDTH=8 -DPCRE2_STATIC -DSUPPORT_PCRE2_8 -DSUPPORT_UNICODE
  -DHAVE_STDLIB_H -DHAVE_STRING_H -DHAVE_LIMITS_H -DHAVE_STDINT_H -DHAVE_INTTYPES_H)
mapfile -t pcre2_sources < <(sed -n 's/^    "\(pcre2_[a-z0-9_]*\.c\)",$/\1/p' "$crate/build.rs")
mapfile -t core_sources < <(sed -n '/^const CORE_SOURCES/,/^];/s/^    "\(.*\.c\)",$/\1/p' "$crate/build.rs")
[[ ${#pcre2_sources[@]} -gt 20 && ${#core_sources[@]} -gt 5 ]] || { echo "cannot read the source lists from build.rs" >&2; exit 1; }

objects=()
for file in "${pcre2_sources[@]}"; do
  "$CC" "${common[@]}" -w "${pcre2_defines[@]}" -I"$pcre2" -c "$pcre2/$file" -o "$out/${file%.c}.o"
  objects+=("$out/${file%.c}.o")
done
core_objects=()
for file in "${core_sources[@]}" ../../../csrc/df_ffi.c; do
  name="$(basename "${file%.c}")"
  warnings=(-w)
  [[ "$name" == df_ffi ]] && warnings=(-Wall -Wextra -Werror)
  "$CC" "${common[@]}" "${warnings[@]}" -include "$crate/csrc/df_namespace.h" -DPCRE2_CODE_UNIT_WIDTH=8 -DPCRE2_STATIC \
    -I"$core" -I"$pcre2" -I"$crate/csrc" -c "$core/$file" -o "$out/$name.o"
  core_objects+=("$out/$name.o")
done
"$CC" "${common[@]}" -I"$crate/csrc" "$here/df_ffi_test.c" "${core_objects[@]}" "${objects[@]}" -lm \
  -o "$out/df_ffi_test"

mkdir "$out/work"
ASAN_OPTIONS=detect_leaks=1:abort_on_error=1:strict_string_checks=1:detect_stack_use_after_return=1 \
UBSAN_OPTIONS=print_stacktrace=1:halt_on_error=1 \
  "$out/df_ffi_test" "$golden" "$out/work"

unprefixed="$(nm -g --defined-only "${core_objects[@]}" | awk 'NF == 3 { print $3 }' \
  | grep -vE '^(biliup_df_|df_|_+(asan|odr_asan))' || true)"
if [[ -n "$unprefixed" ]]; then
  echo "unprefixed global symbols (add them to csrc/df_namespace.h):" >&2
  echo "$unprefixed" >&2
  exit 1
fi
echo "sanitizer and symbol checks passed"
