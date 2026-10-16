#!/usr/bin/env bash
# Instrument Rust with ASan/LSan and run the unchanged C ABI regression client.
# No native C engine is built. CI supplies a nightly Rust compiler.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/../.." && pwd)"
golden="$(cd "$crate/../danmaku-factory/tests/golden" && pwd)"
compiler="${CC:-clang}"
cargo_bin="${CARGO:-cargo}"
rustc_bin="${RUSTC:-rustc}"
host="$("$rustc_bin" -vV | sed -n 's/^host: //p')"
[[ -n "$host" ]] || { echo 'cannot determine the Rust host target' >&2; exit 1; }
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
target_dir="${BILIUP_SANITIZER_TARGET_DIR:-$out/target}"

export RUSTFLAGS="${RUSTFLAGS:-} -Zsanitizer=address -Cforce-frame-pointers=yes"
export ASAN_OPTIONS="detect_leaks=1:abort_on_error=1:strict_string_checks=1:detect_stack_use_after_return=1"
export UBSAN_OPTIONS="print_stacktrace=1:halt_on_error=1"

# Explicit --target keeps sanitizer flags away from host proc-macros.
"$cargo_bin" build --locked -p danmaku-factory-sys --lib \
  --target "$host" --target-dir "$target_dir"
library="$target_dir/$host/debug/libdanmaku_factory_sys.a"
"$compiler" -std=gnu11 -g -O1 -fno-omit-frame-pointer \
  -fsanitize=address,undefined -fno-sanitize-recover=all \
  -Wall -Wextra -Werror -I"$crate/csrc" "$here/df_ffi_test.c" \
  "$library" -ldl -lpthread -lm -o "$out/df_ffi_test"
mkdir "$out/work"
"$out/df_ffi_test" "$golden" "$out/work"

# Exercise Rust allocators, callbacks, DFA and JIT, not only the adapter.
"$cargo_bin" test --locked -p pcre2-rust --test public_contract \
  --target "$host" --target-dir "$target_dir"

symbols="$(nm -g --defined-only "$library")"
for symbol in df_ffi_abi_version df_ffi_version df_options_init df_ffi_layout df_convert_xml_to_ass; do
  if ! awk -v expected="$symbol" '$NF == expected { found = 1 } END { exit !found }' <<< "$symbols"; then
    echo "missing Rust C ABI export: $symbol" >&2
    exit 1
  fi
done
echo 'Rust sanitizer and C ABI checks passed'
