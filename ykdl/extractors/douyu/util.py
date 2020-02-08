#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.util.jsengine import JSEngine

assert JSEngine, "No JS Interpreter found, can't parse douyu live/video!"

import random
import json
import uuid
import time
import string
import pkgutil

try:
    # try load local .js file first
    # from https://cdnjs.com/libraries/crypto-js
    js_md5 = pkgutil.get_data(__name__, 'crypto-js-md5.min.js')
    if not isinstance(js_md5, str):
        js_md5 = js_md5.decode()
except IOError:
    js_md5 = get_content('https://cdnjs.cloudflare.com/ajax/libs/crypto-js/3.1.9-1/crypto-js.min.js')

def get_random_name(l):
    return random.choice(string.ascii_lowercase) + \
           ''.join(random.sample(string.ascii_letters + string.digits, l - 1))

def get_h5enc(html, vid):
    js_enc = match1(html, '(var vdwdae325w_64we =[\s\S]+?)\s*</script>')
    if js_enc is None or 'ub98484234(' not in js_enc:
        html_h5enc = get_content('https://www.douyu.com/swf_api/homeH5Enc?rids=' + vid)
        data = json.loads(html_h5enc)
        assert data['error'] == 0, data['msg']
        js_enc = data['data']['room' + vid]
    return js_enc

def ub98484234(js_enc, extractor, params):
    names_dict = {
        'debugMessages': get_random_name(8),
        'decryptedCodes': get_random_name(8),
        'resoult': get_random_name(8),
        '_ub98484234': get_random_name(8),
        'workflow': match1(js_enc, 'function ub98484234\(.+?\Weval\((\w+)\);'),
    }
    js_dom = '''
    {debugMessages} = {{{decryptedCodes}: []}};
    if (!this.window) {{window = {{}};}}
    if (!this.document) {{document = {{}};}}
    '''.format(**names_dict)
    js_patch = '''
    {debugMessages}.{decryptedCodes}.push({workflow});
    var patchCode = function(workflow) {{
        var testVari = /(\w+)=(\w+)\([\w\+]+\);.*?(\w+)="\w+";/.exec(workflow);
        if (testVari && testVari[1] == testVari[2]) {{
            {workflow} += testVari[1] + "[" + testVari[3] + "] = function() {{return true;}};";
        }}
    }};
    patchCode({workflow});
    var subWorkflow = /(?:\w+=)?eval\((\w+)\)/.exec({workflow});
    if (subWorkflow) {{
        var subPatch = (
            `{debugMessages}.{decryptedCodes}.push('sub workflow: ' + subWorkflow);
            patchCode(subWorkflow);`
        ).replace(/subWorkflow/g, subWorkflow[1]) + subWorkflow[0];
        {workflow} = {workflow}.replace(subWorkflow[0], subPatch);
    }}
    eval({workflow});
    '''.format(**names_dict)
    js_debug = '''
    var {_ub98484234} = ub98484234;
    ub98484234 = function(p1, p2, p3) {{
        try {{
            var resoult = {_ub98484234}(p1, p2, p3);
            {debugMessages}.{resoult} = resoult;
        }} catch(e) {{
            {debugMessages}.{resoult} = e.message;
        }}
        return {debugMessages};
    }};
    '''.format(**names_dict)
    js_enc = js_enc.replace('eval({workflow});'.format(**names_dict), js_patch)

    js_ctx = JSEngine()
    js_ctx.append(js_md5)
    js_ctx.append(js_dom)
    js_ctx.append(js_enc)
    js_ctx.append(js_debug)

    did = uuid.uuid4().hex
    tt = str(int(time.time()))
    ub98484234 = js_ctx.call('ub98484234', extractor.vid, did, tt)
    extractor.logger.debug('ub98484234: %s', ub98484234)
    ub98484234 = ub98484234[names_dict['resoult']]
    params.update({
        'v': match1(ub98484234, 'v=(\d+)'),
        'did': did,
        'tt': tt,
        'sign': match1(ub98484234, 'sign=(\w{32})')
    })
