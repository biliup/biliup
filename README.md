# bilibiliupload

自动录制上传星际选手直播录像

## Linux系统下使用方法：

        启动：    python3 AutoUpload.py start

        退出：    python3 AutoUpload.py stop

        重启：    python3 AutoUpload.py restart
***

### 线程管理：

```s
ps -T -p <PID>

top -H -p <PID>

pstree -p <PID>
```

### 查看系统文件句柄使用

`lsof -n |awk '{print $2}'|sort|uniq -c |sort -nr|more`

#### 错误记录：

requests.exceptions.ConnectionError: HTTPSConnectionPool(host='api.twitch.tv', port=443): Max retries exceeded with url: /kraken/streams/innovation_s2 (Caused by NewConnectionError('<requests.packages.urllib3.connection.VerifiedHTTPSConnection object at 0x7fedd8032a20>: Failed to establish a new connection: [Errno 110] Connection timed out',))

youtube_dl.utils.DownloadError: ERROR: Live stream is offline