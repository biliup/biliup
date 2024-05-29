import json
import pathlib
import shutil
import os
from collections import UserDict
from sqlalchemy import select

from biliup.database.models import Configuration, LiveStreamers, UploadStreamers

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib


class Config(UserDict):
    def load_cookies(self, file='cookies.json'):
        if not os.path.exists(file):
            raise FileNotFoundError(file)

        if self.data.get('user') is None:
            self.data['user'] = {'cookies': {}}
        if self.data["user"].get("cookies") is None:
            self.data['user']['cookies'] = {}
        with open(file, encoding='utf-8') as stream:
            s = json.load(stream)
            for i in s["cookie_info"]["cookies"]:
                name = i["name"]
                self.data["user"]["cookies"][name] = i["value"]
            self.data["user"]["access_token"] = s["token_info"]["access_token"]

    def load_from_db(self, db):
        context = {
            'url_upload_count': self.data.get('url_upload_count', {}),
            'upload_filename': self.data.get('upload_filename', []),
            'PluginInfo': self.data.get('PluginInfo')
        }

        for con in db.execute(select(Configuration.value).where(Configuration.key == 'config')):
            self.data = json.loads(con.value)
        self.data.update(context)
        self['streamers'] = {}
        for ls in db.scalars(select(LiveStreamers)):
            self['streamers'][ls.remark] = {
                k: v for k, v in ls.as_dict().items() if v and (k not in ['upload_streamers_id', 'id', 'remark'])}
            # self['streamers'][ls.remark].pop('upload_streamers')
            if ls.upload_streamers_id:
                self['streamers'][ls.remark].update({
                    k: v for k, v in ls.uploadstreamers.as_dict().items() if v and k not in ['id', 'template_name']})
            # if self['streamers'][ls.remark].get('tags'):
            #     self['streamers'][ls.remark]['tags'] = self['streamers'][ls.remark]['tags']
        # for us in UploadStreamers.select():
        #     config.data[con.key] = con.value

    def save_to_db(self, db):
        for k, v in self['streamers'].items():
            us = UploadStreamers(**UploadStreamers.filter_parameters(
                {"template_name": k, "tags": v.pop('tags', ['biliup']), ** v}))
            db.add(us)
            db.flush()
            url = v.pop('url')
            urls = url if isinstance(url, list) else [url]  # 兼容 url 输入字符串和列表
            for url in urls:
                ls = LiveStreamers(**LiveStreamers.filter_parameters(
                    {"upload_streamers_id": us.id, "remark": k, "url": url, ** v}))
                db.add(ls)
        del self['streamers']
        configuration = Configuration(key='config', value=json.dumps(self.data))
        db.add(configuration)
        db.commit()

    def load(self, file):
        import yaml
        if file is None:
            if pathlib.Path('config.yaml').exists():
                file = open('config.yaml', 'rb')
            elif pathlib.Path('config.toml').exists():
                self.data['toml'] = True
                file = open('config.toml', "rb")
            else:
                raise FileNotFoundError('未找到配置文件，请先创建配置文件')
        with file as stream:
            if file.name.endswith('.toml'):
                self.data = tomllib.load(stream)
            else:
                self.data = yaml.load(stream, Loader=yaml.FullLoader)

    def create_without_config_input(self, file):
        import yaml
        if file is None:
            if pathlib.Path('config.toml').exists():
                file = open('config.toml', 'rb')
            elif pathlib.Path('config.yaml').exists():
                file = open('config.yaml', encoding='utf-8')
            else:
                try:
                    from importlib.resources import files
                except ImportError:
                    from importlib_resources import files
                shutil.copy(files("biliup.web").joinpath('public/config.toml'), '.')
                file = open('config.toml', 'rb')

            # else:
            #     try:
            #         from importlib.resources import files
            #     except ImportError:
            #         # Try backported to PY<37 `importlib_resources`.
            #         from importlib_resources import files
            #     shutil.copy(files("biliup.web").joinpath('public/config.yaml'), 'common')
            #     file = open('config.yaml', encoding='utf-8')

        with file as stream:
            if file.name.endswith('.toml'):
                self.data = tomllib.load(stream)
                self.data['toml'] = True
            else:
                self.data = yaml.load(stream, Loader=yaml.FullLoader)

    def save(self):
        if self.data.get('toml'):
            import tomli_w
            with open('config.toml', 'rb') as stream:
                old_data = tomllib.load(stream)
                old_data["lines"] = self.data["lines"]
                old_data["threads"] = self.data["threads"]
                old_data["streamers"] = self.data["streamers"]
            with open('config.toml', 'wb') as stream:
                tomli_w.dump(old_data, stream)
        else:
            import yaml
            with open('config.yaml', 'w+', encoding='utf-8') as stream:
                old_data = yaml.load(stream, Loader=yaml.FullLoader)
                old_data["user"]["cookies"] = self.data["user"]["cookies"]
                old_data["user"]["access_token"] = self.data["user"]["access_token"]
                old_data["lines"] = self.data["lines"]
                old_data["threads"] = self.data["threads"]
                old_data["streamers"] = self.data["streamers"]
                yaml.dump(old_data, stream, default_flow_style=False, allow_unicode=True)

    def dump(self, file):
        """ 转储到旧版配置文件 """
        if not file:
            file = 'config.toml'
        if os.path.exists(file):
            from datetime import datetime
            import logging
            logger = logging.getLogger('biliup')
            new_name = f'{file}.backup.{datetime.now().strftime("%Y%m%d%H%M%S")}'
            logger.info(f"{file} 文件已存在，已将原文件重命名为 {new_name}")
            os.rename(file, new_name)
        exclude_keys = ['PluginInfo', 'upload_filename', 'url_upload_count']
        temp = {k: v for k, v in self.data.items() if k not in exclude_keys}
        if self.data.get('yaml') or file.endswith(".yaml"):
            import yaml
            with open(file, 'w+', encoding='utf-8') as stream:
                yaml.dump(temp, stream, default_flow_style=False, allow_unicode=True)
        else:
            import tomli_w
            # print(config)
            with open(file, 'wb') as stream:
                tomli_w.dump(temp, stream)
        return file


config = Config()
