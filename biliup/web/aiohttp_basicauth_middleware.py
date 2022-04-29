"""
https://github.com/bugov/aiohttp-basicauth-middleware
"""

import inspect
import logging
from typing import (
    Callable,
    Iterable,
    Type,
    Coroutine,
    Tuple
)
from aiohttp import web
from .http_basic_auth import parse_header, BasicAuthException


log = logging.getLogger(__name__)


class BaseStrategy:
    def __init__(self, request: web.Request, storage: dict, handler: Callable, header: str):
        self.request = request
        self.storage = storage
        self.handler = handler
        self.header = header

        log.debug('Init strategy %r', (self.request, self.storage, self.handler))

    def get_credentials(self) -> Tuple[str, str]:
        try:
            return parse_header(self.header)
        except BasicAuthException:
            log.info('Invalid basic auth header: %r', self.header)
            self.on_error()

    async def password_test(self) -> bool:
        login, password = self.get_credentials()
        server_password = self.storage.get(login)

        if server_password != password:
            return False

        return True

    async def check(self) -> web.Response:
        if await self.password_test():
            return await self.handler(self.request)

        self.on_error()

    def on_error(self):
        raise web.HTTPUnauthorized(headers={'WWW-Authenticate': 'Basic'})


def check_access(
    auth_dict: dict,
    header_value: str,
    strategy: Callable = lambda x: x
) -> bool:
    log.debug('Check access: %r', header_value)

    try:
        login, password = parse_header(header_value)
    except BasicAuthException:
        return False

    hashed_password = auth_dict.get(login)
    hashed_request_password = strategy(password)

    if hashed_password != hashed_request_password:
        return False

    return True


def basic_auth_middleware(
    urls: Iterable,
    auth_dict: dict,
    strategy: Type[BaseStrategy] = lambda x: x
) -> Coroutine:
    async def factory(app, handler) -> Coroutine:
        async def middleware(request) -> web.Response:
            for url in urls:
                if not request.path.startswith(url):
                    continue

                if inspect.isclass(strategy) and issubclass(strategy, BaseStrategy):
                    log.debug("Use Strategy: %r", strategy.__name__)
                    strategy_obj = strategy(
                        request,
                        auth_dict,
                        handler,
                        request.headers.get('Authorization', '')
                    )
                    return await strategy_obj.check()

                if not check_access(auth_dict, request.headers.get('Authorization', ''), strategy):
                    raise web.HTTPUnauthorized(headers={'WWW-Authenticate': 'Basic'})

                return await handler(request)
            return await handler(request)
        return middleware
    return factory
