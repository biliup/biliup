# bilibiliupload

支持自动录制各大直播平台，上传直播录像到bilibili。  
相关设置在config.yaml文件中，如直播间地址，b站账号密码

使用需要修改文件名**config(demo).yaml** ➡ **config.yaml**

>## Linux系统下使用方法：
>
>        启动：    ./Bilibili.py start
>
>        退出：    ./Bilibili.py stop
>
>        重启：    ./Bilibili.py restart
>
> `ps -A | grep .py` 查看进程是否启动成功
>***
>
>## Windows系统下使用方法：
>     图形界面版
>        在release中下载AutoTool.msi进行安装
>     命令行版
>        启动：    python Bilibili.py
>python3 FFmpeg QQ群：837362626

以daemon进程启动，结果写入日志文件。

直播监测、下载的自动化用youtube-dl，但是有一些网站不支持，或者支持不够好，通过抓包分析api解决。

上传的自动化需要解决视频分割和验证登陆的问题，有两种方案。

* 模拟人操作滑动验证码，先二值化、灰度处理找到缺口处，得知要移动多少像素，但是会发现拖到位置还是不能通过验证，它会提示拼图被怪物吃了。因为滑动验证码还会分析你拖动的行为，所以我们不能直接拖动到正确位置，要像真的人一样，有加速度，先快后慢，上下抖动，有时候还会拖过再拖回来，这样通过验证的成功率就很高了。

* 因为app是没有验证码的，所以还可以通过逆向app来分析验证过程。

通过线程池限制并发数，减少磁盘占满的可能性。同时检测下载情况，如果卡死或者三次下载超时会重新下载。保证了一定的可用性。

包含代码更新后在空闲时自动重启的功能，下载和上传是两个独立模块，都可以单独调用，下载模块是通过反射实现插件化的。

下载的基类都在`engine/plugins/__init__.py`中
拓展其他网站很容易，只要继承下载模块的基类就行了，只需要提供很少的代码

拓展上传的网站，或者修改上传的具体实现也很容易，修改`downloader.py`这个文件即可

同时实现了一套基于装饰器的事件驱动框架，用装饰器保证可读性和易用性。在现有的功能上增加其他功能就变得很容易，只需要注册发生对应事件时执行的函数，比如下载后转码等等

```python
# e.p.给函数注册事件
# block=True 会使用子线程来执行
# block=False 使用主线程执行
@event_manager.register("transcoding", block=True)
def modify(self, live_m):
    pass
```
