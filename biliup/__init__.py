import asyncio

from .common.reload import AutoReload
from .common.timer import Timer

from .engine.event import Event
from .engine import config

__version__ = "0.2.10"


async def main(args):
    from .handler import CHECK_UPLOAD, CHECK, event_manager

    event_manager.start()

    async def check_timer():
        event_manager.send_event(Event(CHECK_UPLOAD))
        for k in event_manager.context['checker'].keys():
            event_manager.send_event(Event(CHECK, (k,)))

    wait = config.get('event_loop_interval') if config.get('event_loop_interval') else 40
    # 初始化定时器
    timer = Timer(func=check_timer, interval=wait)

    interval = config.get('check_sourcecode') if config.get('check_sourcecode') else 15
    if args.http:
        from aiohttp import web

        async def url_status(request):
            return web.json_response(event_manager.context['KernelFunc'].get_url_status())

        app = web.Application()
        app.add_routes([web.get('/url-status', url_status)])
        # web.run_app(app, host=host, port=port)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, host=args.host, port=args.port)

        detector = AutoReload(event_manager, timer, runner.cleanup, interval=interval)
        await asyncio.gather(detector.astart(), timer.astart(), site.start(), return_exceptions=True)
    else:
        # 模块更新自动重启
        detector = AutoReload(event_manager, timer, interval=interval)
        await asyncio.gather(detector.astart(), timer.astart(), return_exceptions=True)
