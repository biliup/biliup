"""
HTTP Basic Auth python lib from https://github.com/bugov/http-basic-auth
"""

import base64

__version__ = '1.2.0'


class BasicAuthException(Exception):
    """General exception for all http-basic-auth problems
    """


def parse_token(token: str, coding='utf-8') -> (str, str):
    """Get login + password tuple from Basic Auth token.
    """
    try:
        b_token = bytes(token, encoding=coding)
    except UnicodeEncodeError as e:
        raise BasicAuthException from e
    except TypeError as e:
        raise BasicAuthException from e

    try:
        auth_pair = base64.b64decode(b_token, validate=True)
    except base64.binascii.Error as e:
        raise BasicAuthException from e

    try:
        (login, password) = auth_pair.split(b':', maxsplit=1)
    except ValueError as e:
        raise BasicAuthException from e

    try:
        return str(login, encoding=coding), str(password, encoding=coding)
    except UnicodeDecodeError as e:
        raise BasicAuthException from e


def generate_token(login: str, password: str, coding='utf-8') -> str:
    """Generate Basic Auth token from login and password
    """
    try:
        b_login = bytes(login, encoding=coding)
        b_password = bytes(password, encoding=coding)
    except UnicodeEncodeError as e:
        raise BasicAuthException from e
    except TypeError as e:
        raise BasicAuthException from e

    if b':' in b_login:
        raise BasicAuthException

    b_token = base64.b64encode(b'%b:%b' % (b_login, b_password))

    return str(b_token, encoding=coding)


def parse_header(header_value: str, coding='utf-8') -> (str, str):
    """Get login + password tuple from Basic Auth header value.
    """
    if header_value is None:
        raise BasicAuthException

    try:
        basic_prefix, token = header_value.strip().split(maxsplit=1)
    except AttributeError as e:
        raise BasicAuthException from e
    except ValueError as e:
        raise BasicAuthException from e

    if basic_prefix.lower() != 'basic':
        raise BasicAuthException

    return parse_token(token, coding=coding)


def generate_header(login: str, password: str, coding='utf-8') -> str:
    """Generate Basic Auth header value from login and password
    """
    return 'Basic %s' % generate_token(login, password, coding=coding)