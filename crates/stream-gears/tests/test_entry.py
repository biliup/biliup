"""Entry-point behaviour of the installed wheel (``biliup`` / ``main_loop``).

Run against an installed wheel, not the source tree:
``pytest crates/stream-gears/tests``. Server tests bind a free loopback port
and run in a temp dir, so they stay offline.
"""

import os
import signal
import socket
import subprocess
import sys
import sysconfig
import textwrap
import time
import urllib.request
from pathlib import Path

import pytest

TIMEOUT = 60


def biliup_script():
    name = "biliup.exe" if sys.platform == "win32" else "biliup"
    path = Path(sysconfig.get_path("scripts")) / name
    assert path.is_file(), f"console script not installed: {path}"
    return str(path)


def run(args, cwd, **kwargs):
    return subprocess.run(
        args, cwd=cwd, capture_output=True, text=True, timeout=TIMEOUT, **kwargs
    )


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def env_without_rust_log(**extra):
    env = {k: v for k, v in os.environ.items() if k != "RUST_LOG"}
    env.update(extra)
    return env


def run_server(tmp_path, global_args=(), env=None):
    """Starts ``biliup server``, waits for ``GET /`` to return 200, stops it
    and returns the contents of ``ds_update.log``."""
    port = free_port()
    args = [biliup_script(), *global_args, "server", "--port", str(port)]
    proc = subprocess.Popen(
        args,
        cwd=tmp_path,
        env=env or env_without_rust_log(),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        deadline = time.monotonic() + TIMEOUT
        while True:
            assert proc.poll() is None, proc.stderr.read()
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=2) as r:
                    assert r.status == 200
                    break
            except OSError:
                if time.monotonic() > deadline:
                    raise
                time.sleep(0.5)
    finally:
        if proc.poll() is None:
            proc.send_signal(signal.SIGINT)
            try:
                proc.wait(timeout=TIMEOUT)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
    assert not (tmp_path / "download.log").exists()
    return (tmp_path / "ds_update.log").read_text(encoding="utf-8")


def debug_lines(log):
    """DEBUG records outside tower_http, which is at debug level by default."""
    return [
        line for line in log.splitlines() if " DEBUG " in line and "tower_http" not in line
    ]


def test_version_exits_zero(tmp_path):
    result = run([biliup_script(), "--version"], tmp_path)
    assert result.returncode == 0, result.stderr


def test_help_exits_zero(tmp_path):
    result = run([biliup_script(), "--help"], tmp_path)
    assert result.returncode == 0, result.stderr
    assert "server" in result.stdout


def test_bad_flag_exits_nonzero(tmp_path):
    result = run([biliup_script(), "--bad-flag"], tmp_path)
    assert result.returncode == 2
    assert "--bad-flag" in result.stderr


def test_main_loop_returns_on_help_and_raises_system_exit_on_error(tmp_path):
    code = textwrap.dedent(
        """
        import sys
        import stream_gears

        sys.argv = ["biliup", "--version"]
        assert stream_gears.main_loop() is None
        assert stream_gears.main_loop() is None
        sys.argv = ["biliup", "--help"]
        assert stream_gears.main_loop() is None
        sys.argv = ["biliup", "--bad-flag"]
        try:
            stream_gears.main_loop()
        except SystemExit as e:
            assert e.code == 2, e.code
        else:
            raise AssertionError("expected SystemExit")
        print("still running")
        """
    )
    result = run([sys.executable, "-c", code], tmp_path)
    assert result.returncode == 0, result.stderr
    assert result.stdout.rstrip().endswith("still running")


@pytest.mark.skipif(sys.platform == "win32", reason="uses SIGINT to stop the server")
class TestServer:
    def test_writes_ds_update_log(self, tmp_path):
        log = run_server(tmp_path)
        assert "listening on" in log
        assert not debug_lines(log)

    def test_rust_log_flag(self, tmp_path):
        assert debug_lines(run_server(tmp_path, ["--rust-log", "debug"]))

    def test_rust_log_env(self, tmp_path):
        env = env_without_rust_log(RUST_LOG="debug")
        assert debug_lines(run_server(tmp_path, env=env))

    def test_rust_log_flag_overrides_env(self, tmp_path):
        env = env_without_rust_log(RUST_LOG="debug")
        log = run_server(tmp_path, ["--rust-log", "warn"], env=env)
        assert " INFO " not in log
        assert not debug_lines(log)
