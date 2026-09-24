"""Layout and type stubs of the installed ``stream_gears`` package.

Run against an installed wheel: ``pytest crates/stream-gears/tests``.
"""

import importlib.machinery
import importlib.metadata
import inspect
import json
import os
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest
import stream_gears
from stream_gears import Credit, Segment, StreamGearsError, UploadLine

PACKAGE_FILES = {
    "stream_gears/__init__.py",
    "stream_gears/py.typed",
    "stream_gears/stream_gears.pyi",
}
PYTHON_SOURCE = Path(__file__).resolve().parents[1]


def test_top_level_names():
    assert Segment is stream_gears.PySegment
    assert UploadLine is stream_gears.UploadLine
    assert issubclass(StreamGearsError, RuntimeError)
    assert Credit(type=1, raw_text="hi") == {"type": 1, "raw_text": "hi"}
    assert Credit.__required_keys__ == {"type", "raw_text"}
    assert Credit.__optional_keys__ == {"biz_id"}


def test_native_module_is_a_submodule():
    native = stream_gears.stream_gears
    assert native.__name__ == "stream_gears.stream_gears"
    assert native.__file__.endswith(tuple(importlib.machinery.EXTENSION_SUFFIXES))
    assert Path(native.__file__).parent == Path(stream_gears.__file__).parent


def test_all_matches_native_module():
    assert set(stream_gears.__all__) == set(stream_gears.stream_gears.__all__) | {
        "Credit",
        "Segment",
    }


def is_editable_install():
    direct_url = importlib.metadata.distribution("biliup").read_text("direct_url.json")
    return bool(direct_url) and json.loads(direct_url).get("dir_info", {}).get(
        "editable", False
    )


def test_wheel_ships_stubs():
    package_dir = Path(stream_gears.__file__).parent
    assert (package_dir / "py.typed").is_file()
    assert (package_dir / "stream_gears.pyi").is_file()
    # An editable install's RECORD lists the .pth, not the package files.
    if not is_editable_install():
        files = {str(f).replace("\\", "/") for f in importlib.metadata.files("biliup")}
        assert PACKAGE_FILES <= files


def test_biliup_resolves_from_python_source(tmp_path):
    """Editable installs put only `python-source` on sys.path; `biliup` must
    still resolve to the repository's root package."""
    code = (
        "import importlib.util, biliup\n"
        "print(biliup.__file__)\n"
        "print(importlib.util.find_spec('biliup.__main__').origin)\n"
    )
    result = subprocess.run(
        [sys.executable, "-c", code],
        cwd=tmp_path,
        env={**os.environ, "PYTHONPATH": str(PYTHON_SOURCE)},
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert result.returncode == 0, result.stderr
    package = PYTHON_SOURCE.parents[1] / "biliup"
    assert result.stdout.splitlines() == [
        str(package / "__init__.py"),
        str(package / "__main__.py"),
    ]


def test_upload_text_signature_is_accepted():
    """`upload` has a hand-written `text_signature`; every name and default in it
    must go through PyO3's argument parsing. An invalid `extra_fields` raises
    ValueError before anything else happens; a wrong name would raise TypeError."""
    params = inspect.signature(stream_gears.upload).parameters
    required = {"video_path": ["a.mp4"], "cookie_file": "cookies.json", "title": "t"}
    kwargs = {
        name: required[name] if p.default is inspect.Parameter.empty else p.default
        for name, p in params.items()
    }
    assert kwargs["tag"] == "" and kwargs["desc_v2"] == [] and kwargs["tid"] == 171
    kwargs["extra_fields"] = "{"
    with pytest.raises(ValueError, match="extra_fields"):
        stream_gears.upload(**kwargs)


GOOD = """
import stream_gears
from stream_gears import Credit, Segment, StreamGearsError, UploadLine

segment = Segment()
segment.time = 60
credit: Credit = {"type": 2, "raw_text": "name", "biz_id": "1"}
qrcode: str = stream_gears.login_by_qrcode(stream_gears.get_qrcode())
ok: bool = stream_gears.login_by_cookies("cookies.json")

try:
    stream_gears.download("http://x/live.flv", {}, "out", {"time": 60})
    stream_gears.download_with_callback("http://x/live.flv", {}, "out", segment, print)
    stream_gears.upload(
        ["a.mp4"], "cookies.json", "title", desc_v2=[credit], line=UploadLine.Cnbldsa
    )
except StreamGearsError as e:
    print(e)
"""

BAD = """
import stream_gears

a: bool = stream_gears.login_by_qrcode("{}")
stream_gears.upload(["a.mp4"], "cookies.json", title=1)
stream_gears.UploadLine.Nope
stream_gears.download("http://x/live.flv", {}, "out", 60)
stream_gears.upload(["a.mp4"], "cookies.json", "t", desc_v2=[{"raw_text": "x"}])
"""


def run_mypy(source, tmp_path):
    pytest.importorskip("mypy")
    (tmp_path / "example.py").write_text(textwrap.dedent(source))
    return subprocess.run(
        [sys.executable, "-m", "mypy", "--strict", "--no-incremental", "example.py"],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        timeout=300,
    )


def test_stubs_accept_valid_usage(tmp_path):
    result = run_mypy(GOOD, tmp_path)
    assert result.returncode == 0, result.stdout + result.stderr


def test_stubs_reject_invalid_usage(tmp_path):
    result = run_mypy(BAD, tmp_path)
    assert result.returncode == 1, result.stdout + result.stderr
    lines = {
        int(line.split(":")[1])
        for line in result.stdout.splitlines()
        if ": error:" in line
    }
    assert lines == {4, 5, 6, 7, 8}, result.stdout
