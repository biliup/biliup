from functools import wraps
from biliup.common.tars import tarscore


STANDARD_CHARSET = 'utf-8'


def auto_decode_fields(cls):
    """自动解码类中的bytes类型数据"""
    original_read_from = cls.readFrom

    @staticmethod
    @wraps(original_read_from)
    def wrapped_read_from(ios: tarscore.TarsInputStream):
        value = original_read_from(ios)
        # 遍历对象的所有属性
        for attr_name, attr_value in vars(value).items():
            # 如果是bytes类型
            if isinstance(attr_value, bytes):
                # 将其解码为字符串
                setattr(value, attr_name, attr_value.decode(STANDARD_CHARSET))
        return value

    cls.readFrom = wrapped_read_from
    return cls