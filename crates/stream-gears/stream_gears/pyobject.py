from typing import Optional


class Segment:
    """视频分段设置"""

    @staticmethod
    def by_time(time: int) -> 'Segment':
        """
        按时长分段

        :param int time: 分段时长, 单位为秒
        :return: 视频分段设置
        """
        segment = Segment()
        segment.time = time
        return segment

    @staticmethod
    def by_size(size: int) -> 'Segment':
        """
        按大小分段

        :param int size: 分段大小, 单位为字节
        :return: 视频分段设置
        """
        segment = Segment()
        segment.size = size
        return segment


class Credit:
    # FIXME: docstring
    type_id: int
    raw_text: str
    biz_id: Optional[str]
