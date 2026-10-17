"""Smoke tests for the installed biliup wheel.

Run against an installed wheel (``pip install dist/biliup-*.whl``), not the
source tree: ``pytest crates/stream-gears/tests``. All tests are offline.
"""

import importlib.metadata
import subprocess
import sys
import sysconfig
from pathlib import Path

import pytest
import stream_gears

EXPORTS = {
    "Credit",
    "PySegment",
    "Segment",
    "StreamGearsError",
    "UploadLine",
    "download",
    "download_with_callback",
    "get_qrcode",
    "login_by_cookies",
    "login_by_qrcode",
    "login_by_sms",
    "login_by_web_cookies",
    "login_by_web_qrcode",
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
    segment = stream_gears.PySegment()
    segment.time = 60
    segment.size = 10 * 1000 * 1000
    with pytest.raises(TypeError):
        segment.time = "60"
