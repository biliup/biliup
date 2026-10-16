# danmaku-factory-sys

Rust ports of DanmakuFactory's XML/ASS core and PCRE2 are statically linked
into biliup. The compatibility C interface is declared in `csrc/df_ffi.h`.
Use the safe `danmaku-factory` crate; biliup runs
every conversion in an internal worker process (`biliup-cli/src/danmaku_worker.rs`).

## Pinned sources

| | Version | Files |
| --- | --- | --- |
| [DanmakuFactory](https://github.com/hihkm/DanmakuFactory) | commit `3813e93b13b087e95901d1822baf9c3540e3f3f6` + `patches/` | Rust XML, ASS, filtering, strings, statistics and message-box modules in `src/` |
| [PCRE2](https://github.com/PCRE2Project/pcre2) | `pcre2-10.49` (commit `6f9d7c13…`) | 8/16/32-bit algorithms, POSIX and JIT in `../pcre2-rust/` |

Not used: DanmakuFactory's `main.c` (the CLI), `Config.c` (config files),
`JsonFile.c`, `TemplateFile/`. PCRE2's command-line tools are independent
test clients rather than shipped conversion programs.
Licences: `vendor/*/LICENSE*`, collected in `THIRD-PARTY-NOTICES.txt`, which
is shipped next to every biliup executable.

`vendor/` retains attribution, licences and exact upstream pins. Original C
algorithm sources are not part of production builds. CI builds an external
pinned C reference to verify compatibility without changing golden fixtures.

## Patches

Fixes for bugs found while auditing the core. They are recorded with
reproducers in a separate tree meant for upstream. These patches now describe
the C reference behavior preserved by the Rust implementation.

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
written into the caller's `df_result`. Rust owns and frees the list.
`df_ffi_layout` and the independent C client verify the original structure layout.

Deliberate limits:

- the legacy C interface keeps message boxes, statistics, user names and
  blacklists disabled; the complete owned `Config` and `convert_full` expose
  these capabilities to Rust callers;
- the legacy interface preserves its `LC_NUMERIC` requirement and
  `DF_ERR_LOCALE`; the complete owned interface parses and formats numbers
  independently of process locale;
- times use signed 32-bit milliseconds; overflow is rejected before writing.
  biliup also drops comments more than 250 hours from the segment;
- the core cuts a comment after 1023 bytes, possibly inside a UTF-8
  character; biliup drops the broken bytes when it reads the ASS.

### Thread safety

Conversions own their state and may run concurrently. Callers must synchronize
changes to process-global locale while using the compatibility interface.
biliup retains its worker process for cancellation, timeouts and isolation.

## Building

Cargo builds Rust sources without a C compiler, bindgen or system PCRE2.
The compatibility header remains available to C clients. `pcre2-rust` provides
all 8/16/32-bit PCRE2 and POSIX exports, including real JIT backends for the
project's supported CPU families. Development and minimal profiles retain
the old native engine's optimization level.

## Tests

- `cargo test -p danmaku-factory-sys -p danmaku-factory`: layout, defaults,
  the golden corpus byte for byte, every error path, Chinese/space paths,
  concurrent callers, descriptor leaks, the locale check;
- `tests/c/check.sh` (nightly Rust and clang): AddressSanitizer/LeakSanitizer
  instrument the Rust implementation; the unchanged C ABI client preserves
  its assertions and uses UBSan. The full Rust regex interface is exercised too;
- `pcre2-rust/tests/upstream_suite.rs`: the unmodified upstream syntax/JIT/POSIX
  clients and serialization interoperability against an independent C reference;
- `../danmaku-factory/tests/golden/generate.py`: regenerates the corpus with
  the pinned CLI built with and without the patches (CI runs it).

## Updating DanmakuFactory or PCRE2

1. Review the pins in `vendor/*/UPSTREAM` and reconcile the Rust algorithms.
2. Build external pinned C references; preserve their original expected fixtures.
3. Run the golden, full-interface and independent differential tests.
4. Run the sanitizer and performance comparisons; regenerate the aggregate
   licence file with `scripts/notices.sh` when attribution changes.
