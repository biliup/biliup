"""The installed wheel reports the built-in danmaku converter.

XML -> ASS conversion is a Rust crate linked into the extension module, so
there is no converter program and no worker process: ``GET /v1/render-tools``
proves the running server can convert in this process.

Run against an installed wheel: ``pytest crates/stream-gears/tests``.
"""

import json
import subprocess
import time
import urllib.request

from test_entry import TIMEOUT, biliup_script, env_without_rust_log, free_port


def test_server_reports_the_built_in_danmaku_converter(tmp_path):
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
