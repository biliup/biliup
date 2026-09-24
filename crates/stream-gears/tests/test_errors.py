"""Error semantics and argument types of the stream_gears Python API.

Run against an installed wheel: ``pytest crates/stream-gears/tests``. All tests
are offline: downloads hit a local ``http.server`` or a closed port, and
logins go through a proxy on a closed port.
"""

import http.server
import inspect
import socket
import threading
from types import SimpleNamespace

import pytest
import stream_gears

LOGIN_FUNCTIONS = [
    "get_qrcode",
    "login_by_cookies",
    "login_by_qrcode",
    "login_by_sms",
    "login_by_web_cookies",
    "login_by_web_qrcode",
    "send_sms",
]

# FLV file header (9 bytes, audio + video) followed by PreviousTagSize0.
FLV_HEADER = b"FLV\x01\x05\x00\x00\x00\x09" + b"\x00\x00\x00\x00"

ROUTES = {
    "/empty.flv": b"",
    "/header-only.flv": FLV_HEADER,
    # A tag header needs 11 bytes; the stream ends after 5.
    "/truncated.flv": FLV_HEADER + b"\x09\x00\x00\x10\x00",
    "/not-a-stream": b"<html><body>hello</body></html>",
}


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = ROUTES.get(self.path)
        if body is None:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


@pytest.fixture(scope="module")
def server():
    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{httpd.server_address[1]}"
    httpd.shutdown()
    httpd.server_close()


@pytest.fixture
def closed_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture(autouse=True)
def in_tmp_path(tmp_path, monkeypatch):
    # download() writes download.log and logins write cookies.json into the cwd.
    monkeypatch.chdir(tmp_path)


def assert_stream_gears_error(call):
    """`call` must raise StreamGearsError, catchable with `except Exception`.

    Catches BaseException so that a Rust panic (pyo3 PanicException derives
    from BaseException) fails the assertion instead of aborting the run.
    """
    try:
        call()
    except BaseException as e:  # noqa: B036
        assert isinstance(e, Exception), f"{type(e).__name__} is not an Exception: {e}"
        assert type(e).__name__ == "StreamGearsError", f"{type(e).__name__}: {e}"
        return e
    pytest.fail("did not raise")


def download(url, tmp_path, segment=None, **kwargs):
    if segment is None:
        segment = stream_gears.PySegment()
    return stream_gears.download_with_callback(
        url, {}, str(tmp_path / "out"), segment, **kwargs
    )


def test_stream_gears_error_hierarchy():
    assert issubclass(stream_gears.StreamGearsError, RuntimeError)
    assert issubclass(stream_gears.StreamGearsError, Exception)


@pytest.mark.parametrize("name", LOGIN_FUNCTIONS)
def test_login_proxy_defaults_to_none(name):
    params = inspect.signature(getattr(stream_gears, name)).parameters
    assert params["proxy"].default is None


@pytest.mark.parametrize("name", ["download", "download_with_callback"])
def test_download_signature(name):
    params = inspect.signature(getattr(stream_gears, name)).parameters
    assert list(params)[:4] == ["url", "header_map", "file_name", "segment"]
    assert params["proxy"].default is None


def test_download_closed_port(tmp_path, closed_port):
    assert_stream_gears_error(
        lambda: download(f"http://127.0.0.1:{closed_port}/live.flv", tmp_path)
    )


def test_download_http_error(server, tmp_path):
    e = assert_stream_gears_error(lambda: download(f"{server}/missing", tmp_path))
    assert "404" in str(e)


def test_download_empty_response(server, tmp_path):
    e = assert_stream_gears_error(lambda: download(f"{server}/empty.flv", tmp_path))
    assert "incomplete" in str(e)


def test_download_truncated_flv(server, tmp_path):
    assert_stream_gears_error(lambda: download(f"{server}/truncated.flv", tmp_path))


def test_download_hls_error(server, tmp_path):
    e = assert_stream_gears_error(lambda: download(f"{server}/not-a-stream", tmp_path))
    assert "HLS" in str(e)


def test_download_plain_function_rejects_bad_segment(server, tmp_path):
    with pytest.raises(TypeError):
        stream_gears.download(f"{server}/header-only.flv", {}, str(tmp_path / "out"), 42)


def test_callback_accepts_lambda(server, tmp_path):
    names = []
    download(f"{server}/header-only.flv", tmp_path, file_name_callback_fn=lambda n: names.append(n))
    assert names == [str(tmp_path / "out.flv")]
    assert (tmp_path / "out.flv").is_file()


def test_callback_accepts_bound_method(server, tmp_path):
    names = []
    download(f"{server}/header-only.flv", tmp_path, file_name_callback_fn=names.append)
    assert names == [str(tmp_path / "out.flv")]


def test_callback_must_be_callable(server, tmp_path):
    with pytest.raises(TypeError, match="callable"):
        download(f"{server}/header-only.flv", tmp_path, file_name_callback_fn=42)


def test_callback_exception_is_reraised(server, tmp_path):
    def callback(name):
        raise ValueError(f"boom {name}")

    with pytest.raises(ValueError, match="boom"):
        download(f"{server}/header-only.flv", tmp_path, file_name_callback_fn=callback)
    # The download itself finished before the callback error was re-raised.
    assert (tmp_path / "out.flv").is_file()


def test_py_segment_fields_are_readable():
    segment = stream_gears.PySegment()
    assert segment.time is None and segment.size is None
    segment.time = 60
    segment.size = 10 * 1000 * 1000
    assert (segment.time, segment.size) == (60, 10 * 1000 * 1000)


@pytest.mark.parametrize(
    "segment",
    [
        {"time": 3600},
        {"size": 1 << 30, "time": None},
        {},
        SimpleNamespace(time=3600, size=None),
        SimpleNamespace(size=1 << 30),
    ],
    ids=["dict-time", "dict-size", "dict-empty", "object", "object-size-only"],
)
def test_segment_accepts_dict_and_object(server, tmp_path, segment):
    download(f"{server}/header-only.flv", tmp_path, segment=segment)
    assert (tmp_path / "out.flv").is_file()


def upload(tmp_path, **kwargs):
    kwargs.setdefault("cookie_file", str(tmp_path / "no-such-cookies.json"))
    return stream_gears.upload([str(tmp_path / "video.flv")], title="t", **kwargs)


@pytest.mark.parametrize("extra_fields", ['{"a":1,}', "[1, 2]", "{"])
def test_upload_invalid_extra_fields(tmp_path, extra_fields):
    with pytest.raises(ValueError, match="extra_fields"):
        upload(tmp_path, extra_fields=extra_fields)


@pytest.mark.parametrize("extra_fields", ["", '{"a": 1}'])
def test_upload_valid_extra_fields_reach_login(tmp_path, extra_fields):
    # Parsing passes; the missing cookie file is the next (offline) failure.
    assert_stream_gears_error(lambda: upload(tmp_path, extra_fields=extra_fields))


@pytest.mark.parametrize(
    "credit",
    [
        {"type": 1, "raw_text": "hi", "biz_id": ""},
        {"type": 1, "raw_text": "hi"},
        SimpleNamespace(type_id=1, raw_text="hi", biz_id=None),
    ],
    ids=["dict", "dict-no-biz-id", "object"],
)
def test_upload_desc_v2_accepts_dict_and_object(tmp_path, credit):
    assert_stream_gears_error(lambda: upload(tmp_path, desc_v2=[credit]))


def test_upload_desc_v2_missing_field(tmp_path):
    with pytest.raises(TypeError, match="raw_text"):
        upload(tmp_path, desc_v2=[{"type": 1}])


@pytest.mark.parametrize("name", ["login_by_sms", "login_by_qrcode"])
def test_login_invalid_json_raises_value_error(name):
    args = (123456, "{not json") if name == "login_by_sms" else ("{not json",)
    with pytest.raises(ValueError, match="ret"):
        getattr(stream_gears, name)(*args, proxy=None)


def test_login_by_sms_failure_raises(tmp_path, closed_port):
    e = assert_stream_gears_error(
        lambda: stream_gears.login_by_sms(
            123456,
            '{"captcha_key": "x", "tel": "13800000000", "cid": 86}',
            proxy=f"http://127.0.0.1:{closed_port}",
        )
    )
    assert str(e)
    assert not (tmp_path / "cookies.json").exists()


def test_login_by_cookies_missing_file(tmp_path):
    e = assert_stream_gears_error(
        lambda: stream_gears.login_by_cookies(str(tmp_path / "no-such-cookies.json"))
    )
    assert "no-such-cookies.json" in str(e) or "No such file" in str(e)
