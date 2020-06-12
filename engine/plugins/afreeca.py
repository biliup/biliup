# import json
# import re
import requests
from engine.plugins import YDownload, BatchCheckBase, SDownload
from common import logger

headers = {
    'client-id': '5qnc2cacngon0bg6yy42633v2y9anf',
    'Authorization': 'Bearer wx8vi6yxg9mvgg8t365ekmuka3a1fz'
}
VALID_URL_BASE = r'(?:https?://)?(?:(?:play|go|m)\.)?afreecatv\.com/(?P<id>[0-9_a-zA-Z]+)'

class Afreeca(SDownload):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
__plugin__ = Afreeca
