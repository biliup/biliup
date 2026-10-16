#!/usr/bin/env python3
"""Regenerates the golden corpus with DanmakuFactory CLI builds.

    python3 -I generate.py --cli PATCHED_CLI --upstream-cli PINNED_CLI

PATCHED_CLI is the CLI built from the pinned commit plus ../../../danmaku-factory-sys/patches
(it writes the expected .ass files); PINNED_CLI is the unmodified pinned commit
(the program biliup used to run). Every case must produce the same bytes with
both, except cases marked "upstream_bug", where the unmodified CLI has
undefined behaviour or emits garbage that the patches fix.

Inputs are generated from a fixed seed and written in the normalised form
biliup's danmaku.rs produces. Neither CLI may have a DanmakuFactoryConfig.json
next to it: the CLI reads that file before its arguments.
"""

import argparse
import json
import random
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent

# ConvertOptions field -> how the CLI receives it.
DEFAULTS = {
    "width": 1920,
    "height": 1080,
    "display_area": 1.0,
    "scroll_area": 1.0,
    "scroll_time": 12.0,
    "fix_time": 5.0,
    "density": 0,
    "line_spacing": 0,
    "top_margin": 0,
    "bottom_margin": 0,
    "font_size": 38,
    "font_size_strict": False,
    "font_size_norm": False,
    "font_name": "Microsoft YaHei",
    "opacity": 180,
    "outline": 0.0,
    "outline_blur": 0.0,
    "outline_opacity": 255,
    "shadow": 1.0,
    "bold": False,
    "save_blocked": True,
    "block_mode": 0,
}
BLOCK_NAMES = [(1, "R2L"), (2, "L2R"), (4, "TOP"), (8, "BOTTOM"), (16, "SPECIAL"), (32, "COLOR"), (64, "REPEAT")]


def number(value):
    return repr(value) if isinstance(value, float) else str(value)


def cli_args(options):
    """Every option is passed explicitly, so the mapping covers all fields."""
    o = dict(DEFAULTS, **options)
    args = ["--resolution", f"{o['width']}x{o['height']}"]
    for field, flag in [
        ("display_area", "--displayarea"),
        ("scroll_area", "--scrollarea"),
        ("scroll_time", "--scrolltime"),
        ("fix_time", "--fixtime"),
        ("density", "--density"),
        ("line_spacing", "--line-spacing"),
        ("top_margin", "--top-margin"),
        ("bottom_margin", "--bottom-margin"),
        ("font_size", "--fontsize"),
        ("opacity", "--opacity"),
        ("outline", "--outline"),
        ("outline_blur", "--outline-blur"),
        ("outline_opacity", "--outline-opacity"),
        ("shadow", "--shadow"),
    ]:
        args += [flag, number(o[field])]
    if o["font_size_strict"]:
        args.append("--font-size-strict")
    if o["font_size_norm"]:
        args.append("--font-size-norm")
    args += ["--fontname", o["font_name"]]
    args += ["--bold", "true" if o["bold"] else "false"]
    args += ["--saveblocked", "true" if o["save_blocked"] else "false"]
    blocks = [name for bit, name in BLOCK_NAMES if o["block_mode"] & bit]
    args += ["--blockmode", "-".join(blocks) if blocks else "null"]
    return args


def escape(text):
    return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def xml(comments):
    """danmaku.rs: <d p="seconds,mode,size,color,0,0,0,0">escaped text</d>"""
    out = ['<?xml version="1.0" encoding="UTF-8"?><i>\n']
    for at_ms, mode, size, color, text in comments:
        out.append(f'<d p="{at_ms / 1000:.3f},{mode},{size},{color},0,0,0,0">{escape(text)}</d>\n')
    out.append("</i>\n")
    return "".join(out)


CJK = "弹幕测试一人一分别让那些外国佬以为我们比不起太对了急死这才图不敢想象后面有多粘先睡会宗来叫我起挖矿主播好厉害哈"
LATIN = "abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789"
EMOJI = ["😂", "🦅", "🐉", "👍🏽", "👨‍👩‍👧", "🇨🇳", "❤️", "🎉", "𠀀", "🫠"]


def random_text(rng, low, high):
    length = rng.randint(low, high)
    pools = [CJK, LATIN, "".join(EMOJI)]
    pool = rng.choice(pools)
    if pool == "".join(EMOJI):
        return "".join(rng.choice(EMOJI) for _ in range(max(1, length // 3)))
    return "".join(rng.choice(pool) for _ in range(length)).strip() or "x"


def inputs():
    rng = random.Random(20261010)
    white = 16777215
    files = {}
    files["basic_cn"] = [
        (220 + i * 497 % 20000, 1, 25, rng.choice([white, white, white, 8046667, 16738740, 0]), random_text(rng, 2, 24))
        for i in range(40)
    ]
    files["long_text"] = [
        (500, 1, 25, white, "The quick brown fox jumps over the lazy dog. " * 6),
        (900, 1, 25, white, "x" * 1500),  # truncated by readXml at 1023 bytes
        (1300, 1, 25, white, "WordsWithoutAnySpaces" * 30),
        (1700, 5, 25, white, "top " * 60),
        (2100, 4, 25, white, "a" + "长" * 400),  # truncation splits a UTF-8 sequence
        (2500, 1, 25, white, "空 格 很 多 的 弹 幕 " * 40),  # ASS escaping grows the text past 1023
    ]
    files["emoji"] = [(300 + i * 350, 1, 25, white, text) for i, text in enumerate([
        "太对了一人一分急死🦅🐉美😂",
        "family 👨‍👩‍👧 flag 🇨🇳 skin 👍🏽",
        "&#128514; numeric reference",
        "a&#b mid-text marker",
        "&#; empty reference",
        "&#9999999; out of range",
        "&#12 no semicolon here",
        "𠀀 extension B and 🫠",
        "❤️❤️❤️",
    ])]
    files["special_chars"] = [(200 + i * 300, mode, 25, white, text) for i, (mode, text) in enumerate([
        (1, "{\\b1}not bold{\\i1}"),
        (1, "back\\slash \\n \\N \\h literal"),
        (5, "line one\nline two"),
        (4, "tab\there"),
        (1, "quotes \" and ' here"),
        (1, "amp & lt < gt > done"),
        (1, "comma, semicolon; colon:"),
        (1, "printf %s %d %n %x"),
        (6, "left to right"),
        (1, "  leading and trailing  "),
        (1, "&amp; already escaped"),
    ])]
    files["types"] = [
        (100, 1, 25, white, "mode 1 R2L"),
        (200, 2, 25, white, "mode 2"),
        (300, 3, 25, white, "mode 3"),
        (400, 4, 25, white, "mode 4 bottom"),
        (500, 5, 25, white, "mode 5 top"),
        (600, 6, 25, white, "mode 6 L2R"),
        (700, 7, 25, 16711680, '[0.1,0.2,"1-0.5",4.5,"特殊弹幕/n第二行",0,30,0.8,0.9,1500,200,true,"微软雅黑",1]'),
        (800, 7, 36, white, '[100,200,"0.8-0.8",3,"fixed pixel",0,0,100,200,0,0,true,"",1]'),
        (900, 8, 25, white, "code danmaku"),
        (1000, 9, 25, white, "mode 9"),
        (1100, 0, 25, white, "mode 0"),
        (1200, -1, 25, white, "negative R2L"),
    ]
    files["dense"] = sorted(
        (rng.randint(0, 120000), rng.choice([1] * 8 + [4, 5]), 25,
         rng.choice([white] * 7 + [255, 65280, 16776960]), random_text(rng, 1, 40))
        for _ in range(1500)
    )
    files["same_time"] = [(0, rng.choice([1, 1, 4, 5]), 25, white, f"同时 {i}") for i in range(200)]
    files["sizes"] = [(300 * i, rng.choice([1, 4, 5]), size, white, f"size {size}")
                      for i, size in enumerate([12, 18, 25, 36, 45, 64, 1, 1000])]
    files["long_time"] = [(at, 1, 25, white, f"t={at}") for at in
                          [0, 59_990, 3_599_995, 3_600_000, 36_000_005, 359_999_990, 999_999_000]]
    files["unsorted"] = [(rng.randint(0, 60000), 1, 25, white, f"#{i}") for i in range(300)]
    files["repeat"] = [(i * 250, 1, 25, white, ["666", "666", "哈哈哈", "666", "好"][i % 5]) for i in range(60)]
    # One-pixel display areas and lines as tall as the display (upstream_bug cases).
    files["tiny"] = [(0, mode, 25, white, f"t{mode}{i}") for i in range(3) for mode in (1, 4, 5)]
    files["trailing_ref"] = [(0, 1, 25, white, "ends with &#"), (100, 4, 25, white, "half emoji &#55357;")]
    return files


BILIUP = {
    "font_name": "Noto Sans CJK SC",
    "font_size": 38,
    "font_size_strict": True,
    "opacity": 204,
    "outline": 2.0,
    "shadow": 0.0,
    "scroll_time": 12.0,
    "fix_time": 12.0,
    "display_area": 0.4,
    "density": -1,
}


def cases():
    small = dict(BILIUP, width=320, height=180, font_size=26, opacity=255, outline=0.5,
                 scroll_time=4.0, fix_time=4.0, display_area=1.0, density=0)
    edge = dict(BILIUP, width=96, height=64, font_size=8, opacity=1, outline=4.0,
                scroll_time=2.0, fix_time=2.0, display_area=0.05, density=1000)
    large = dict(BILIUP, width=3840, height=2160, font_size=200, outline=3.5,
                 scroll_time=60.0, fix_time=60.0, display_area=1.0)
    extras = {"line_spacing": 4, "top_margin": 10, "bottom_margin": 12, "scroll_area": 0.5, "bold": True,
              "outline": 1.5, "outline_blur": 2.0, "outline_opacity": 128, "font_size_norm": True, "width": 1280,
              "height": 720}
    out = []

    def add(name, input_name, options, bug=None):
        case = {"name": name, "input": f"{input_name}.xml", "options": options}
        if bug:
            case["upstream_bug"] = bug
        out.append(case)

    for name in ["basic_cn", "long_text", "emoji", "special_chars", "types", "dense", "same_time", "sizes",
                 "long_time", "unsorted", "repeat"]:
        add(f"{name}-biliup", name, BILIUP)
    for name in ["basic_cn", "dense", "same_time", "types"]:
        add(f"{name}-small", name, small)
        add(f"{name}-defaults", name, {})
    add("dense-edge", "dense", edge,
        "F4: a 3 px display area with 8 px lines; the unmodified CLI corrupts the heap and aborts")
    add("dense-large", "dense", large)
    add("sizes-not-strict", "sizes", dict(BILIUP, font_size_strict=False))
    add("sizes-extras", "sizes", extras)
    add("dense-extras", "dense", extras)
    add("repeat-block", "repeat", dict(BILIUP, block_mode=64, save_blocked=False, density=5))
    add("types-block", "types", dict(BILIUP, block_mode=2 | 4 | 16, save_blocked=True))
    add("basic_cn-block-color", "basic_cn", dict(BILIUP, block_mode=32, save_blocked=False))
    add("tiny-line-fills-area", "tiny", dict(BILIUP, width=64, height=38, display_area=1.0),
        "F4: findMin reads array[-1]/array[height] and the caller writes fixEndTime[-1]")
    add("tiny-one-pixel", "tiny", dict(BILIUP, width=4, height=1, display_area=0.5),
        "F4: zero-row occupancy arrays are read and written")
    add("trailing_ref", "trailing_ref", BILIUP,
        "F3/F7: a trailing '&#' reads past the text; '&#55357;' is written as invalid UTF-8")
    return out


class Failed(Exception):
    pass


def run(cli, workdir, case):
    shutil.copyfile(HERE / "inputs" / case["input"], workdir / "in.xml")
    out = workdir / "out.ass"
    out.unlink(missing_ok=True)
    args = [str(cli), "-i", "in.xml", "-o", "out.ass", "--ignore-warnings", "--force", *cli_args(case["options"])]
    proc = subprocess.run(args, cwd=workdir, capture_output=True, timeout=120)
    if proc.returncode != 0 or not out.is_file():
        raise Failed(f"{case['name']}: {cli} exited with {proc.returncode}\n{proc.stderr.decode(errors='replace')}")
    return out.read_bytes()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path, help="CLI built with biliup's patches")
    parser.add_argument("--upstream-cli", required=True, type=Path, help="unmodified CLI at the pinned commit")
    args = parser.parse_args()
    for cli in (args.cli, args.upstream_cli):
        if (cli.resolve().parent / "DanmakuFactoryConfig.json").exists():
            raise SystemExit(f"remove DanmakuFactoryConfig.json next to {cli}")

    (HERE / "inputs").mkdir(exist_ok=True)
    (HERE / "expected").mkdir(exist_ok=True)
    for name, comments in inputs().items():
        (HERE / "inputs" / f"{name}.xml").write_text(xml(comments), encoding="utf-8", newline="\n")
    all_cases = cases()
    (HERE / "cases.json").write_text(json.dumps(all_cases, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")

    differing = []
    with tempfile.TemporaryDirectory() as tmp:
        workdir = Path(tmp)
        for case in all_cases:
            expected = run(args.cli, workdir, case)
            (HERE / "expected" / f"{case['name']}.ass").write_bytes(expected)
            try:
                if run(args.upstream_cli, workdir, case) != expected:
                    differing.append(case["name"])
            except Failed as error:
                if "upstream_bug" not in case:
                    raise SystemExit(str(error))
                differing.append(f"{case['name']} (crashed)")
    marked = [case["name"] for case in all_cases if "upstream_bug" in case]
    unexpected = sorted({name.split(" ")[0] for name in differing} - set(marked))
    print(f"{len(all_cases)} cases; differing from the unmodified CLI: {differing or 'none'}")
    if unexpected:
        raise SystemExit(f"patches changed the output of ordinary input: {unexpected}")


if __name__ == "__main__":
    sys.exit(main())
