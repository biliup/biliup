import datetime
import json
import os
import pathlib

import aiohttp_cors
import requests
from aiohttp import web
from peewee import DoesNotExist
from playhouse.shortcuts import model_to_dict

from .aiohttp_basicauth_middleware import basic_auth_middleware
import stream_gears
import biliup.common.reload
from biliup.config import config
from biliup.plugins.bili_webup import BiliBili, Data
from ..database.models import UploadStreamers, LiveStreamers, Configuration, StreamerInfo, FileList
from ..database.db import DB as db

BiliBili = BiliBili(Data())

routes = web.RouteTableDef()
async def get_basic_config(request):
    res = {
        "line": config.data['lines'],
        "limit": config.data['threads'],
    }
    if config.data.get("toml"):
        res['toml'] = True
    else:
        res['user'] = {
            "SESSDATA": config.data['user']['cookies']['SESSDATA'],
            "bili_jct": config.data['user']['cookies']['bili_jct'],
            "DedeUserID__ckMd5": config.data['user']['cookies']['DedeUserID__ckMd5'],
            "DedeUserID": config.data['user']['cookies']['DedeUserID'],
            "access_token": config.data['user']['access_token'],
        }

    return web.json_response(res)

async def url_status(request):
    from biliup.app import context
    return web.json_response(context['KernelFunc'].get_url_status())

async def set_basic_config(request):
    post_data = await request.json()
    config.data['lines'] = post_data['line']
    if config.data['lines'] == 'cos':
        config.data['lines'] = 'cos-internal'
    config.data['threads'] = post_data['limit']
    if not config.data.get("toml"):
        cookies = {
            "SESSDATA": str(post_data['user']['SESSDATA']),
            "bili_jct": str(post_data['user']['bili_jct']),
            "DedeUserID__ckMd5": str(post_data['user']['DedeUserID__ckMd5']),
            "DedeUserID": str(post_data['user']['DedeUserID']),
        }
        config.data['user']['cookies'] = cookies
        config.data['user']['access_token'] = str(post_data['user']['access_token'])
    return web.json_response({"status": 200})


async def get_streamer_config(request):
    return web.json_response(config.data['streamers'])


async def set_streamer_config(request):
    post_data = await request.json()
    # config.data['streamers'] = post_data['streamers']
    for i,j in post_data['streamers'].items():
        if i not in config.data['streamers']:
            config.data['streamers'][i]={}
        for key,Value in j.items():
            config.data['streamers'][i][key]=Value
    for i in config.data['streamers']:
        if i not in post_data['streamers']:
            del config.data['streamers'][i]

    return web.json_response({"status": 200}, status=200)


async def save_config(request):
    config.save()
    biliup.common.reload.global_reloader.triggered = True
    import logging
    logger = logging.getLogger('biliup')
    logger.info("配置保存，将在进程空闲时重启")
    return web.json_response({"status": 200}, status=200)


async def root_handler(request):
    return web.HTTPFound('/index.html')


async def cookie_login(request):
    if config.data.get("toml"):
        print("trying to login by cookie")
        try:
            stream_gears.login_by_cookies()
        except Exception as e:
            return web.HTTPBadRequest(text="login failed" + str(e))
    else:
        cookie = config.data['user']['cookies']
        try:
            BiliBili.login_by_cookies(cookie)
        except Exception as e:
            print(e)
            return web.HTTPBadRequest(text="login failed")
    return web.json_response({"status": 200})


async def sms_login(request):
    pass


async def sms_send(request):
    # post_data = await request.json()

    pass


async def qrcode_get(request):
    if config.data.get("toml"):
        try:
            r = eval(stream_gears.get_qrcode())
        except Exception as e:
            return web.HTTPBadRequest(text="get qrcode failed")
    else:
        r = BiliBili.get_qrcode()
    return web.json_response(r)


async def qrcode_login(request):
    post_data = await request.json()
    if config.data.get("toml"):
        try:
            if stream_gears.login_by_qrcode(json.dumps(post_data)):
                return web.json_response({"status": 200})
        except Exception as e:
            return web.HTTPBadRequest(text="login failed" + str(e))
    else:
        try:
            r = await BiliBili.login_by_qrcode(post_data)
        except:
            return web.HTTPBadRequest(text="timeout for qrcode validate")
        for cookie in r['data']['cookie_info']['cookies']:
            config.data['user']['cookies'][cookie['name']] = cookie['value']
        config.data['user']['access_token'] = r['data']['token_info']['access_token']
        return web.json_response(r)


async def pre_archive(request):
    if config.data.get("toml"):
        config.load_cookies()
    cookies = config.data['user']['cookies']
    return web.json_response(BiliBili.tid_archive(cookies))

async def tag_check(request):
    if BiliBili.check_tag(request.rel_url.query['tag']):
        return web.json_response({"status": 200})
    else:
        return web.HTTPBadRequest(text="标签违禁")

@routes.get('/v1/videos')
async def streamers(request):
    media_extensions = ['.mp4', '.flv', '.3gp', '.webm', '.mkv', '.ts']
    # 获取文件列表
    file_list = []
    i = 1
    for file_name in os.listdir('.'):
        name, ext = os.path.splitext(file_name)
        if ext in media_extensions:
            file_list.append({'key': i,'name': file_name, 'updateTime': os.path.getmtime(file_name), 'size': os.path.getsize(file_name)})
            i += 1
    return web.json_response(file_list)

@routes.get('/v1/streamer-info')
async def streamers(request):
    res = []
    for s_info in StreamerInfo.select():
        streamer_info = model_to_dict(s_info)
        streamer_info['files'] = []
        for file in s_info.file_list:
            tmp = model_to_dict(file)
            del tmp['streamer_info']
            streamer_info['files'].append(tmp)
        streamer_info['date'] = int(streamer_info['date'].timestamp())
        res.append(streamer_info)
    return web.json_response(res)


@routes.get('/v1/streamers')
async def streamers(request):
    from biliup.app import context
    res = []
    for ls in LiveStreamers.select():
        temp = model_to_dict(ls)
        url = temp['url']
        status = 'Idle'
        if context['PluginInfo'].url_status.get(url) == 1:
            status = 'Working'
        if context['url_upload_count'].get(url, 0) > 0:
            status = 'Inspecting'
        temp['status'] = status
        if temp.get("upload_streamers"):  # 返回 upload_id 而不是 upload_streamers
            temp["upload_id"] = temp["upload_streamers"]["id"]
        temp.pop("upload_streamers")
        res.append(temp)
    return web.json_response(res)

@routes.post('/v1/streamers')
async def add_lives(request):
    from biliup.app import context
    json_data = await request.json()
    uid = json_data.get('upload_id')
    if uid:
        us = UploadStreamers.get_by_id(uid)
        to_save = LiveStreamers(**json_data, upload_streamers=us)
    else:
        to_save = LiveStreamers(**json_data)
    # to_save = LiveStreamers(remark=name, url=url, filename_prefix=None, upload_streamers=us)
    try:
        to_save.save()
    except Exception as e:
        return web.HTTPBadRequest(text=str(e))
    config.load_from_db()
    context['PluginInfo'].add(json_data['remark'], json_data['url'])
    return web.json_response(model_to_dict(to_save))

@routes.put('/v1/streamers')
async def lives(request):
    from biliup.app import context
    json_data = await request.json()
    old = LiveStreamers.get_by_id(json_data['id'])
    old_url = old.url
    uid = json_data.get('upload_id')
    try:
        if uid:
            us = UploadStreamers.get_by_id(json_data['upload_id'])
            # LiveStreamers.update(**json_data, upload_streamers=us).where(LiveStreamers.id == old.id).execute()
            db.update_live_streamer(**{**json_data, "upload_streamers": us})
        else:
            # LiveStreamers.update(**json_data).where(LiveStreamers.id == old.id).execute()
            db.update_live_streamer(**json_data)
    except Exception as e:
        return web.HTTPBadRequest(text=str(e))
    config.load_from_db()
    context['PluginInfo'].delete(old_url)
    context['PluginInfo'].add(json_data['remark'], json_data['url'])
    return web.json_response(LiveStreamers.get_dict(id=json_data['id']))

@routes.delete('/v1/streamers/{id}')
async def streamers(request):
    from biliup.app import context
    org = LiveStreamers.get_by_id(request.match_info['id'])
    LiveStreamers.delete_by_id(request.match_info['id'])
    context['PluginInfo'].delete(org.url)
    return web.HTTPOk()

@routes.get('/v1/upload/streamers')
async def get_streamers(request):
    res = []
    for us in UploadStreamers.select():
        res.append(model_to_dict(us))
    return web.json_response(res)

@routes.get('/v1/upload/streamers/{id}')
async def streamers_id(request):
    id = request.match_info['id']
    return web.json_response(UploadStreamers.get_dict(id=id))

@routes.delete('/v1/upload/streamers/{id}')
async def streamers(request):
    UploadStreamers.delete_by_id(request.match_info['id'])
    return web.HTTPOk()

@routes.post('/v1/upload/streamers')
async def streamers_post(request):
    json_data = await request.json()
    to_save = UploadStreamers(**json_data)
    to_save.save()
    config.load_from_db()
    res = model_to_dict(to_save)
    return web.json_response(res)

@routes.put('/v1/upload/streamers')
async def streamers_put(request):
    json_data = await request.json()
    UploadStreamers.update(**json_data)
    config.load_from_db()
    return web.json_response(UploadStreamers.get_dict(id=json_data['id']))

@routes.get('/v1/users')
async def users(request):
    records = Configuration.select().where(Configuration.key == 'bilibili-cookies')
    res = []
    for record in records:
        res.append({
            'id': record.id,
            'name': record.value,
            'value': record.value,
            'platform': record.key,
        })
    return web.json_response(res)

@routes.post('/v1/users')
async def users(request):
    json_data = await request.json()
    to_save = Configuration(key=json_data['platform'], value=json_data['value'])
    to_save.save()
    return web.json_response([{
        'id': to_save.id,
        'name': to_save.value,
        'value': to_save.value,
        'platform': to_save.key,
    }])

@routes.delete('/v1/users/{id}')
async def users(request):
    Configuration.delete_by_id(request.match_info['id'])
    return web.HTTPOk()

@routes.get('/v1/configuration')
async def users(request):
    try:
        record = Configuration.get(Configuration.key == 'config')
    except DoesNotExist:
        return web.json_response({})
    return web.json_response(json.loads(record.value))

@routes.put('/v1/configuration')
async def users(request):
    json_data = await request.json()
    try:
        record = Configuration.get(Configuration.key == 'config')
        to_save = Configuration(key='config', value=json.dumps(json_data), id=record.id)
    except DoesNotExist:
        to_save = Configuration(key='config', value=json.dumps(json_data))
    to_save.save()
    config.load_from_db()
    return web.json_response(model_to_dict(to_save))

@routes.get('/bili/archive/pre')
async def pre_archive(request):
    path = 'cookies.json'
    conf = Configuration.get_or_none(Configuration.key == 'bilibili-cookies')
    if conf is not None:
        path = conf.value
    config.load_cookies(path)
    cookies = config.data['user']['cookies']
    return web.json_response(BiliBili.tid_archive(cookies))

@routes.get('/bili/space/myinfo')
async def myinfo(request):
    config.load_cookies(request.query['user'])
    cookies = config.data['user']['cookies']
    return web.json_response(BiliBili.myinfo(cookies))

@routes.get('/bili/proxy')
async def proxy(request):
    return web.Response(body=requests.get(request.query['url']).content)


def find_all_folders(directory):
    result = []
    for foldername, subfolders, filenames in os.walk(directory):
        for subfolder in subfolders:
            result.append(os.path.relpath(os.path.join(foldername, subfolder), directory))
    return result

async def service(args):
    try:
        from importlib.resources import files
    except ImportError:
        # Try backported to PY<37 `importlib_resources`.
        from importlib_resources import files

    app = web.Application()
    app.add_routes([
        web.get('/api/check_tag', tag_check),
        web.get('/url-status', url_status),
        web.get('/api/basic', get_basic_config),
        web.post('/api/setbasic', set_basic_config),
        web.get('/api/getconfig', get_streamer_config),
        web.post('/api/setconfig', set_streamer_config),
        web.get('/api/login_by_cookie', cookie_login),
        web.get('/api/login_by_sms', sms_login),
        web.post('/api/send_sms', sms_send),
        web.get('/api/save', save_config),
        web.get('/api/get_qrcode', qrcode_get),
        web.post('/api/login_by_qrcode', qrcode_login),
        web.get('/api/archive_pre', pre_archive),
        web.get('/', root_handler)
    ])
    routes.static('/static', '.', show_index = True)
    app.add_routes(routes)
    if args.static_dir:
        app.add_routes([web.static('/', args.static_dir, show_index=False)])
    else:
        # res = [web.static('/', files('biliup.web').joinpath('public'))]
        res = []
        for fdir in pathlib.Path(files('biliup.web').joinpath('public')).glob('*.html'):
            fname = fdir.relative_to(files('biliup.web').joinpath('public'))
            def _copy(fname):
                async def static_view(request):
                    return web.FileResponse(files('biliup.web').joinpath('public/' + str(fname)))
                return static_view
            res.append(web.get('/' + str(fname.with_suffix('')), _copy(fname)))
            # res.append(web.static('/'+fdir.replace('\\', '/'), files('biliup.web').joinpath('public/'+fdir)))
        res.append(web.static('/', files('biliup.web').joinpath('public')))
        app.add_routes(res)
    if args.password:
        app.middlewares.append(basic_auth_middleware(('/',), {'biliup': args.password}, ))

    # web.run_app(app, host=host, port=port)
    cors = aiohttp_cors.setup(app, defaults={
        "*": aiohttp_cors.ResourceOptions(
            allow_credentials=True,
            allow_methods="*",
            expose_headers="*",
            allow_headers="*"
        )
    })

    for route in list(app.router.routes()):
        cors.add(route)

    runner = web.AppRunner(app)
    setup_middlewares(app)
    await runner.setup()
    site = web.TCPSite(runner, host=args.host, port=args.port)
    return runner, site


async def handle_404(request):
    return web.HTTPFound('404')


def create_error_middleware(overrides):

    @web.middleware
    async def error_middleware(request, handler):
        try:
            return await handler(request)
        except web.HTTPException as ex:
            override = overrides.get(ex.status)
            if override:
                return await override(request)

            raise
        except Exception:
            request.protocol.logger.exception("Error handling request")
            return await overrides[500](request)

    return error_middleware


def setup_middlewares(app):
    error_middleware = create_error_middleware({
        404: handle_404
    })
    app.middlewares.append(error_middleware)