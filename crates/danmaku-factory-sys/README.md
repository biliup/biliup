# danmaku-factory-sys

DanmakuFactory's XML → ASS conversion core and PCRE2, compiled from vendored
sources and statically linked into biliup, behind a small C interface
(`csrc/df_ffi.h`). Use it through the safe `danmaku-factory` crate; biliup runs
every conversion in an internal worker process (`biliup-cli/src/danmaku_worker.rs`).

## Pinned sources

| | Version | Files |
| --- | --- | --- |
| [DanmakuFactory](https://github.com/hihkm/DanmakuFactory) | commit `3813e93b13b087e95901d1822baf9c3540e3f3f6` + `patches/` | `XmlFile.c`, `AssFile/`, `List/`, `String/`, `FileUtil/` and the headers they include (`vendor/DanmakuFactory/`) |
| [PCRE2](https://github.com/PCRE2Project/pcre2) | `pcre2-10.49` (commit `6f9d7c13…`), unmodified | the 8-bit library without JIT (`vendor/pcre2/`) |

Not used: DanmakuFactory's `main.c` (the CLI), `Config.c` (config files),
`JsonFile.c`, `TemplateFile/`; PCRE2's JIT, POSIX wrapper and tools.
Licences: `vendor/*/LICENSE*`, collected in `THIRD-PARTY-NOTICES.txt`, which
is shipped next to every biliup executable.

`scripts/vendor.sh` recreates `vendor/` from the pins and `patches/` (it needs
network access; Cargo builds never download anything).

## Patches

Fixes for bugs found while auditing the core. They are recorded with
reproducers in a separate tree meant for upstream; here only the files that
are compiled are patched.

| Patch | Bug | Reachable from biliup before |
| --- | --- | --- |
| 0001 | `readXml` leaks the input `FILE` when no danmaku is read | no (biliup skips empty XML) |
| 0002 | a tag of ≥ 10240 bytes writes one byte past `label[]` (stack overflow) | no (biliup writes short tags) |
| 0003 | text ending in `&#` makes `assEscape` read past the string: a comment `abc&#` came out as `abcp;#` | **yes**, any viewer could send it |
| 0004 | when one line is as tall as the display area, `findMin` reads `array[-1]`/`array[n]` and bottom danmaku write `fixEndTime[-1]`; a display area under 1 px gives empty arrays | **yes**: GUI settings such as 720p, font 38, display area 0.05 read out of bounds (AddressSanitizer), and the unpatched CLI aborts with "double free or corruption" on the corpus case `dense-edge` (96x64, font 8, display area 0.05) |
| 0005 | on a write error the output `FILE` is closed twice (double free), the occupancy arrays leak, and errors from the final `fflush`/`fclose` are ignored, so a full disk produced a truncated ASS reported as success | **yes** (disk full) |
| 0006 | `&#55357;` (a UTF-16 surrogate) is written as invalid UTF-8 | **yes**: biliup then failed to read the ASS |
| 0007 | `freeList` leaks the user, gift and special parts of each danmaku | no (one conversion per process) |

All patched behaviour is identical to the pinned CLI for every input that
does not hit one of these bugs; `../danmaku-factory/tests/golden/generate.py`
checks that on the whole corpus.

## Interface

`df_convert_xml_to_ass(input, output, options, result)` runs what the CLI's
`main()` runs for `-i in.xml -o out.ass` — `readXml`, `blockByType`,
`normFontSize`, `sortList`, `writeAss` — with options passed explicitly in
`df_options` (the CLI defaults come from `df_options_init`). It reads no
configuration file, prints nothing, never waits for input and does not change
the working directory or the locale. Every pointer is borrowed for the call;
nothing is returned that needs freeing: status, counts and a message are
written into the caller's `df_result`. The danmaku list is allocated, cleaned
up on failure and freed inside the wrapper. `df_ffi_layout` lets the Rust
tests check the hand-written bindings against the C layout.

Deliberate limits:

- message boxes (gift/SC/guard), statistics, user names and block lists are
  off: the message-box code has undefined behaviour (`getMsgBoxHeight` has no
  `return`) and biliup's XML only contains `<d>` danmaku;
- `LC_NUMERIC` must use `.`; otherwise `DF_ERR_LOCALE` (the core formats with
  `printf("%f")` and parses with `atof`);
- danmaku times above about 2,000,000 s overflow the core's `int`
  milliseconds; biliup drops comments more than 250 hours from the segment;
- the core cuts a comment after 1023 bytes, possibly inside a UTF-8
  character; biliup drops the broken bytes when it reads the ASS.

### Thread safety

The audit (see `csrc/df_ffi.c`) found no mutable static data on the
conversion path, but upstream does not promise it, so the safe crate
serialises calls with a mutex. biliup does not rely on either: each
conversion runs in its own worker process, which can be killed on cancel or
timeout and cannot take the server down when C code crashes.

## Building

`build.rs` uses the `cc` crate: no libclang/bindgen, no system PCRE2. Flags
that keep the output identical across targets:

- MSVC: `/utf-8` (the sources contain UTF-8 string literals) and `/std:c11`;
- GCC/Clang: `-fsigned-char` (ARM's `char` is unsigned; the baseline is x86),
  `-ffp-contract=off` (no fused multiply-add in the float layout maths),
  `-fvisibility=hidden` (the copies here never interpose a system PCRE2);
- the core's global names are prefixed (`csrc/df_namespace.h`, force-included).

## Tests

- `cargo test -p danmaku-factory-sys -p danmaku-factory`: layout, defaults,
  the golden corpus byte for byte, every error path, Chinese/space paths,
  concurrent callers, descriptor leaks, the locale check;
- `tests/c/check.sh` (clang): the C interface under AddressSanitizer,
  UndefinedBehaviorSanitizer and LeakSanitizer, success and failure paths,
  and a check that no unprefixed global symbol is exported;
- `../danmaku-factory/tests/golden/generate.py`: regenerates the corpus with
  the pinned CLI built with and without the patches (CI runs it).

## Updating DanmakuFactory or PCRE2

1. Change the pins in `scripts/vendor.sh`, rebase or drop `patches/`, run it.
2. Check `csrc/df_ffi.c` against the new `main()` and the audit notes above.
3. Rebuild both CLIs, run `generate.py` and review every changed ASS.
4. Run `tests/c/check.sh`; add new global names to `csrc/df_namespace.h`.
