"""The danmaku worker under Python, where the process is the interpreter, not
biliup: the server must start ``sys.executable`` with the extension module's
own file, and ``main_loop`` must recognise the worker flag first.

Run against an installed wheel: ``pytest crates/stream-gears/tests``.
"""

import json
import subprocess
import sys
import textwrap
import urllib.request

import stream_gears

from test_entry import TIMEOUT, biliup_script, env_without_rust_log, free_port

FLAG = "--biliup-internal-danmaku-worker"
XML = (
    '<?xml version="1.0" encoding="UTF-8"?><i>\n'
    '<d p="0.500,1,25,16777215,0,0,0,0">中文 弹幕 😂</d>\n'
    '<d p="1.000,5,25,255,0,0,0,0">top &amp; red</d>\n'
    "</i>\n"
)


def write_request(work, protocol=1):
    (work / "in.xml").write_text(XML, encoding="utf-8")
    request = {
        "protocol": protocol,
        "input": "in.xml",
        "output": "out.ass",
        "result": "result.json",
        "options": {"width": 640, "height": 360, "font_name": "Noto Sans CJK SC", "font_size_strict": True},
    }
    (work / "request.json").write_text(json.dumps(request), encoding="utf-8")


def run_main_loop(work, *argv):
    code = textwrap.dedent(
        f"""
        import sys
        import stream_gears
        sys.argv = {["biliup", *argv]!r}
        stream_gears.main_loop()
        """
    )
    return subprocess.run(
        [sys.executable, "-c", code], cwd=work, capture_output=True, text=True, timeout=TIMEOUT
    )


def test_main_loop_runs_the_worker_and_nothing_else(tmp_path):
    write_request(tmp_path)
    result = run_main_loop(tmp_path, FLAG, "request.json")
    assert result.returncode == 0, result.stderr
    outcome = json.loads((tmp_path / "result.json").read_text(encoding="utf-8"))
    assert outcome["outcome"] == "converted"
    assert outcome["stats"] == {"danmaku_count": 2, "blocked_count": 0}
    ass = (tmp_path / "out.ass").read_text(encoding="utf-8")
    assert "PlayResX: 640" in ass
    assert "中文\\h弹幕\\h😂" in ass
    assert "{\\c&HFF0000}top\\h&\\hred" in ass
    # No server, log file or database was started.
    assert sorted(p.name for p in tmp_path.iterdir()) == ["in.xml", "out.ass", "request.json", "result.json"]


def test_worker_failures_become_exit_codes(tmp_path):
    write_request(tmp_path, protocol=999)
    result = run_main_loop(tmp_path, FLAG, "request.json")
    assert result.returncode == 3, result.stderr
    assert not (tmp_path / "result.json").exists()
    result = run_main_loop(tmp_path, FLAG)
    assert result.returncode == 2


def test_server_starts_the_worker_through_the_interpreter(tmp_path):
    """``GET /v1/render-tools`` converts a test XML in a real worker; under
    Python that worker is ``sys.executable`` loading this exact module."""
    port = free_port()
    proc = subprocess.Popen(
        [biliup_script(), "server", "--port", str(port)],
        cwd=tmp_path,
        env=env_without_rust_log(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        import time

        deadline = time.monotonic() + TIMEOUT
        while True:
            assert proc.poll() is None, proc.stderr.read()
            try:
                url = f"http://127.0.0.1:{port}/v1/render-tools"
                with urllib.request.urlopen(url, timeout=TIMEOUT) as response:
                    tools = json.load(response)["tools"]
                    break
            except OSError:
                if time.monotonic() > deadline:
                    raise
                time.sleep(0.5)
        assert tools["danmaku_factory"] is True, tools
        assert "弹幕转换" not in (tools["error"] or ""), tools
    finally:
        proc.terminate()
        proc.wait(timeout=TIMEOUT)


def test_the_native_module_file_is_what_the_worker_loads():
    native = stream_gears.stream_gears
    assert native.__file__.endswith((".so", ".pyd")), native.__file__
