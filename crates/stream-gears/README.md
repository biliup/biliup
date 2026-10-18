# stream-gears

Rust/PyO3 bindings for Bilibili login, video upload and direct FLV/HLS download.
The PyPI distribution is named `biliup`; import its APIs as `stream_gears`.

```python
from stream_gears import download, upload

download("https://example.com/live.flv", {}, "recording", {"time": 600})
upload(["recording.flv"], "cookies.json", "My recording")
```

The package contains the native extension and type stubs. It has no service,
recording scheduler, web frontend, fonts, FFmpeg or `biliup` command. Install the
native biliup release or Docker image to run the complete application.

For development, run `maturin develop -m crates/stream-gears/Cargo.toml` from
the repository root. Tests run against an installed package:
`pytest crates/stream-gears/tests` and `python -m mypy.stubtest stream_gears`.
