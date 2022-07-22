import json

from aiohttp import web
from .aiohttp_basicauth_middleware import basic_auth_middleware
import stream_gears
import biliup.common.reload
from biliup.config import config
from biliup.plugins.bili_webup import BiliBili, Data

BiliBili = BiliBili(Data())


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


async def save_config(reequest):
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

async  def tag_check(request):
    if BiliBili.check_tag(request.rel_url.query['tag']):
        return web.json_response({"status": 200})
    else:
        return web.HTTPBadRequest(text="标签违禁")
async def service(args, event_manager):
    async def url_status(request):
        return web.json_response(event_manager.context['KernelFunc'].get_url_status())

    app = web.Application()
    try:
        from importlib.resources import files
    except ImportError:
        # Try backported to PY<37 `importlib_resources`.
        from importlib_resources import files
    app.add_routes([web.get('/api/check_tag', tag_check)])
    app.add_routes([web.get('/url-status', url_status)])
    app.add_routes([web.get('/api/basic', get_basic_config)])
    app.add_routes([web.post('/api/setbasic', set_basic_config)])
    app.add_routes([web.get('/api/getconfig', get_streamer_config)])
    app.add_routes([web.post('/api/setconfig', set_streamer_config)])
    app.add_routes([web.get('/api/login_by_cookie', cookie_login)])
    app.add_routes([web.get('/api/login_by_sms', sms_login)])
    app.add_routes([web.post('/api/send_sms', sms_send)])
    app.add_routes([web.get('/api/save', save_config)])
    app.add_routes([web.get('/api/get_qrcode', qrcode_get)])
    app.add_routes([web.post('/api/login_by_qrcode', qrcode_login)])
    app.add_routes([web.get('/api/archive_pre', pre_archive)])
    app.add_routes([web.get('/', root_handler)])
    if args.static_dir:
        app.add_routes([web.static('/', args.static_dir, show_index=False)])
    else:
        app.add_routes([web.static('/', files('biliup.web').joinpath('public'), show_index=False)])
        app.add_routes([web.static('/build', files('biliup.web').joinpath('public/build'), show_index=False)])
    if args.password:
        app.middlewares.append(basic_auth_middleware(('/',), {'biliup': args.password}, ))

    # web.run_app(app, host=host, port=port)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host=args.host, port=args.port)
    return runner, site
