from collections import UserDict
from .decorators import Plugin


class Config(UserDict):
    def load(self, file):
        import yaml
        if file is None:
            file = open('config.yaml', encoding='utf-8')
        with file as stream:
            self.data = yaml.load(stream, Loader=yaml.FullLoader)


config = Config()


def invert_dict(d: dict):
    inverse_dict = {}
    for k, v in d.items():
        for item in v:
            inverse_dict[item] = k
    return inverse_dict


__all__ = ['config', 'invert_dict', 'Plugin']
