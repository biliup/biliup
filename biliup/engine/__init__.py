from collections import UserDict
import os
import shutil

import biliup
from .decorators import Plugin


class Config(UserDict):
    def load(self, file):
        import yaml
        if file is None:
            file = open('config.yaml', encoding='utf-8')
        with file as stream:
            self.data = yaml.load(stream, Loader=yaml.FullLoader)
    def create_without_config_input(self, file):
        import yaml
        if file is None:
            try:
                file = open('config.yaml', encoding='utf-8')
            except FileNotFoundError:
                biliup_dir = os.path.dirname(biliup.__file__)
                config_data = os.path.join(biliup_dir,"public/","config.yaml")
                shutil.copy(config_data, os.getcwd())
                file = open('config.yaml', encoding='utf-8')
        with file as stream:
            self.data = yaml.load(stream, Loader=yaml.FullLoader)
    def save(self):
        import yaml
        old_file = open('config.yaml',encoding='utf-8')
        old_data = yaml.load(old_file, Loader=yaml.FullLoader)
        old_data["user"]["cookies"]=self.data["user"]["cookies"]
        old_data["user"]["access_token"]=self.data["user"]["access_token"]
        old_data["streamers"]=self.data["streamers"]
        file = open('config.yaml', 'w', encoding='utf-8')
        with file as stream:
            yaml.dump(old_data, stream, default_flow_style=False, allow_unicode=True)


config = Config()


def invert_dict(d: dict):
    inverse_dict = {}
    for k, v in d.items():
        for item in v:
            inverse_dict[item] = k
    return inverse_dict


__all__ = ['config', 'invert_dict', 'Plugin']
