import stream_gears


class Segment:
    pass


if __name__ == '__main__':
    segment = Segment()
    segment.time = 60 * 1 # secs(1 min)
    # segment.size = 6000 * 1024 * 1024
    segment.size = 1000 * 1000 * 10 # bytes(10MB)
    url = ""
    stream_gears.download(
        url = url,
        header_map = {
            "referer": "https://live.bilibili.com",
            "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"
        },
        file_name = "new_test%Y-%m-%dT%H_%M_%S",
        segment = segment,
        proxy = None
    )
