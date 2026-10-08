"""Offline regressions for the Python upload libraries shipped in the package."""

import asyncio
from io import BytesIO
import queue
import threading
from types import SimpleNamespace

import pytest

from biliup.plugins import bili_webup, bili_webup_sync


def test_chunk_retries_exhaustion_stops_upload():
    attempts = []

    async def fail(session, data, params):
        attempts.append(params['chunk'])
        raise asyncio.TimeoutError('offline failure')

    with pytest.raises(RuntimeError, match='chunk 0 failed after 10 attempts'):
        asyncio.run(bili_webup.BiliBili._upload({}, BytesIO(b'abcdefgh'), 4, fail, tasks=1))
    assert attempts == [0] * 10


def test_chunk_retry_retains_the_same_bytes_and_succeeds():
    attempts = []

    async def retry(session, data, params):
        attempts.append((data, params['partNumber']))
        if len(attempts) == 1:
            raise asyncio.TimeoutError('offline failure')

    asyncio.run(bili_webup.BiliBili._upload({}, BytesIO(b'abcdefgh'), 4, retry, tasks=1))
    assert attempts == [(b'abcd', 1), (b'abcd', 1), (b'efgh', 2)]


@pytest.mark.parametrize('tasks, chunk_size', [(0, 4), (1, 0)])
def test_chunk_invalid_parallelism_and_size(tasks, chunk_size):
    async def upload(*args):
        pytest.fail('invalid arguments reached uploading')

    with pytest.raises(ValueError, match='positive'):
        asyncio.run(bili_webup.BiliBili._upload({}, BytesIO(b'x'), chunk_size, upload, tasks=tasks))


def test_sync_constructor_has_no_dependency_on_removed_app_config(tmp_path):
    with bili_webup_sync.BiliBili(bili_webup_sync.Data()) as default:
        assert default.save_dir is None
    target = tmp_path / 'recordings'
    with bili_webup_sync.BiliBili(bili_webup_sync.Data(), save_dir=target) as configured:
        assert configured.save_dir == target
        assert target.is_dir()


def test_sync_stream_submission_reuses_local_video_state(monkeypatch):
    videos = bili_webup_sync.Data(title='test')
    with bili_webup_sync.BiliBili(videos) as uploader:
        uploader._auto_os = {'os': 'upos', 'query': 'test', 'probe_url': '//test'}
        uploader._BiliBili__session.get = lambda *args, **kwargs: SimpleNamespace(
            json=lambda: {'chunk_size': 4}
        )

        async def upload(*args):
            return {'title': args[1], 'filename': args[1], 'desc': ''}

        uploader.upos_stream = upload
        submissions = []

        def submit(*, submit_api, edit, videos):
            submissions.append((edit, len(videos.videos)))
            return {'code': 0, 'data': {'aid': 42}}

        uploader.submit = submit
        for name in ['first', 'second']:
            uploader.upload_stream(queue.SimpleQueue(), name, 4, stop_event=threading.Event())
        assert submissions == [(False, 1), (True, 2)]
        assert videos.aid == 42


def test_sync_failed_chunk_is_not_submitted_as_a_successful_part():
    with bili_webup_sync.BiliBili(bili_webup_sync.Data()) as uploader:
        merged = []
        uploader._BiliBili__session.post = lambda *args, **kwargs: (
            merged.append(kwargs) or SimpleNamespace(json=lambda: {'upload_id': 'test'})
        )
        uploader.upload_chunk_thread = lambda *args: None
        queued = queue.SimpleQueue()
        queued.put(b'abcdefgh')
        queued.put(None)
        ret = {'chunk_size': 4, 'auth': 'offline', 'endpoint': '//localhost',
               'biz_id': 1, 'upos_uri': 'upos://offline'}
        with pytest.raises(RuntimeError, match='chunk failed'):
            asyncio.run(uploader.upos_stream(queued, 'offline.mkv', 8, ret))
        assert len(merged) == 1  # Only the upload ID request, never the merge.


@pytest.mark.parametrize('sync', [False, True])
def test_missing_credit_placeholder_preserves_description(sync):
    module = bili_webup_sync if sync else bili_webup
    cls = module.BiliWebAsync if sync else module.BiliWeb
    kwargs = {'principal': 'test', 'data': {}, 'description': 'plain description',
              'credits': [{'username': 'author', 'uid': 1}]}
    if not sync:
        kwargs['user'] = {}
    uploader = cls(**kwargs)
    assert uploader.creditsToDesc_v2() == [
        {'raw_text': 'plain description', 'biz_id': '', 'type': 1}
    ]


@pytest.mark.parametrize('sync', [False, True])
def test_expired_client_token_uses_local_account_and_refreshes_the_url(sync):
    module = bili_webup_sync if sync else bili_webup
    with module.BiliBili(module.Data(title='test')) as uploader:
        uploader.account = {'username': 'offline', 'password': 'offline'}
        uploader.access_token = 'old-token'
        urls = []

        def post(url, **kwargs):
            urls.append(url)
            return SimpleNamespace(json=lambda: {'code': -101 if len(urls) == 1 else 0})

        uploader._BiliBili__session.post = post
        refreshed = []

        def login(**account):
            refreshed.append(account)
            uploader.access_token = 'new-token'

        uploader.login_by_password = login
        uploader.store = lambda: None
        result = uploader.submit_client({}) if sync else uploader.submit_client()
        assert result == {'code': 0}
        assert refreshed == [uploader.account]
        assert urls[0].endswith('access_key=old-token')
        assert urls[1].endswith('access_key=new-token')


@pytest.mark.parametrize('sync', [False, True])
def test_client_token_refresh_is_bounded_and_requires_an_account(sync):
    module = bili_webup_sync if sync else bili_webup
    with module.BiliBili(module.Data(title='test')) as uploader:
        uploader.access_token = 'expired'
        uploader._BiliBili__session.post = lambda *args, **kwargs: SimpleNamespace(
            json=lambda: {'code': -101}
        )
        submit = lambda: uploader.submit_client({}) if sync else uploader.submit_client()
        with pytest.raises(RuntimeError, match='no account'):
            submit()
        uploader.account = {'username': 'offline', 'password': 'offline'}
        attempts = []
        uploader.login_by_password = lambda **kwargs: attempts.append(kwargs)
        uploader.store = lambda: None
        assert submit() == {'code': -101}
        assert len(attempts) == 1
