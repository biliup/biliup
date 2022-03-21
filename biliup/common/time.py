from datetime import datetime, timezone, timedelta


def now():
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    return bj_dt


def format_time(zonetime, fmt='%Y.%m.%d'):
    if fmt is None:
        fmt = '%Y.%m.%d'
    now = zonetime.strftime(fmt.encode('unicode-escape').decode())
    return now.encode().decode('unicode-escape')