"""Smoke tests for the installed biliup wheel.

Run against an installed wheel (``pip install dist/biliup-*.whl``), not the
source tree: ``pytest crates/stream-gears/tests``. All tests are offline.
"""

import importlib.metadata
import inspect
import subprocess
import sys
import sysconfig
from pathlib import Path

import pytest
import stream_gears

EXPORTS = {
    "PySegment",
    "UploadLine",
    "config_bindings",
    "download",
    "download_with_callback",
    "get_qrcode",
    "login_by_cookies",
    "login_by_qrcode",
    "login_by_sms",
    "login_by_web_cookies",
    "login_by_web_qrcode",
    "main_loop",
    "send_sms",
    "stream_gears",
    "upload",
}

UPLOAD_LINES = [
    "Bldsa",
    "Cnbldsa",
    "Andsa",
    "Atdsa",
    "Bda2",
    "Cnbd",
    "Anbd",
    "Atbd",
    "Tx",
    "Cntx",
    "Antx",
    "Attx",
    "Txa",
    "Alia",
    "Estx",
    "Akbd",
]

LOGIN_FUNCTIONS = [
    "get_qrcode",
    "login_by_cookies",
    "login_by_qrcode",
    "login_by_sms",
    "login_by_web_cookies",
    "login_by_web_qrcode",
    "send_sms",
]

# The CLI exits the process (clap's `--help` / `--version`), so it must run in
# a subprocess. cwd is a temp dir so the repo's `biliup/` source package can't
# shadow the installed one.
TIMEOUT = 60


def run(args, cwd):
    return subprocess.run(
        args, cwd=cwd, capture_output=True, text=True, timeout=TIMEOUT
    )


def biliup_script():
    name = "biliup.exe" if sys.platform == "win32" else "biliup"
    path = Path(sysconfig.get_path("scripts")) / name
    assert path.is_file(), f"console script not installed: {path}"
    return str(path)


def test_exports():
    public = {name for name in dir(stream_gears) if not name.startswith("_")}
    assert public == EXPORTS


def test_upload_line_members():
    members = [
        name
        for name in dir(stream_gears.UploadLine)
        if isinstance(getattr(stream_gears.UploadLine, name), stream_gears.UploadLine)
    ]
    assert sorted(members) == sorted(UPLOAD_LINES)
    assert len(members) == 16


def test_py_segment():
    # Fields are write-only (`#[pyclass(set_all)]` without `get_all`).
    segment = stream_gears.PySegment()
    segment.time = 60
    segment.size = 10 * 1000 * 1000
    with pytest.raises(TypeError):
        segment.time = "60"


def test_config_bindings():
    config = stream_gears.config_bindings()
    assert config.get("streamers") == {}
    assert config.get("no_such_key", "fallback") == "fallback"
    with pytest.raises(AttributeError):
        config.get("no_such_key")


@pytest.mark.parametrize("name", LOGIN_FUNCTIONS)
@pytest.mark.xfail(
    strict=True,
    raises=AssertionError,
    reason="OPS-19: PyO3 >= 0.24 no longer defaults trailing Option args to None",
)
def test_login_proxy_is_optional(name):
    params = inspect.signature(getattr(stream_gears, name)).parameters
    assert params["proxy"].default is None


def test_biliup_version(tmp_path):
    result = run([biliup_script(), "--version"], tmp_path)
    assert result.returncode == 0, result.stderr
    assert importlib.metadata.version("biliup") in result.stdout


def test_python_m_biliup_help(tmp_path):
    result = run([sys.executable, "-m", "biliup", "--help"], tmp_path)
    assert result.returncode == 0, result.stderr
    assert "Usage:" in result.stdout
    assert "server" in result.stdout
