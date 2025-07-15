import stream_gears


class Segment:
    pass


if __name__ == '__main__':
    segment = Segment()
    # segment.time = 60
    # segment.size = 6000 * 1024 * 1024
    segment.size = 60 * 1024 * 1024
    stream_gears.download(
        "",
        {"referer": "https://live.bilibili.com"},
        # {},
        "new_test%Y-%m-%dT%H_%M_%S",
        segment
    )
