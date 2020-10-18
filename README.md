# bilibiliupload
![](https://img.shields.io/badge/python-v3.7%2B-blue)

支持自动录制各大直播平台，上传直播录像到bilibili。  
相关设置在config.yaml文件中，如直播间地址，b站账号密码

## docker一键使用 🔨 
```bash
cd bilibiliupload
sudo docker build . -t sc2
sudo docker run -d sc2
```
## 进入容器 📦
```bash
sudo docker ps (找到你的imageId)
sudo docker exec -it imageId /bin/bash     
```

## 其他使用
使用需要修改文件名**config(demo).yaml** ➡ **config.yaml**\
下载依赖ffmpeg，可以参考Lawliet大神的[安装教程](https://blog.csdn.net/major_zhang/article/details/88945455)
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

Linux下以daemon进程启动

结果写入日志文件

下载部分使用youtube-dl，不支持或者支持的不够好的网站，通过爬虫api解决。

上传的自动化需要解决视频分割和验证登陆的问题，有两种方案。

* 二值化、灰度处理找缺口计算移动像素，直接拖动发现拖到位置不能通过验证，提示：“拼图被怪物吃了”。滑动验证码会上传分析你的拖动行为，不能直接拖动，需要模拟人操作轨迹，加速度、抖动等等，以保证通过验证的成功率。

* 因为app是没有验证码的，所以还可以通过逆向app来分析验证过程。可参考comwrg/bilibiliupload的登陆部分

通过线程池限制并发数，减少磁盘占满的可能性。同时检测下载情况，如果卡死或者下载超时3次重新下载，保证可用性。

包含代码更新后在空闲时自动重启的功能，下载和上传是两个独立模块，都可以单独调用，下载模块是通过反射实现插件化的。

下载的基类都在`engine/plugins/__init__.py`中
拓展其他网站，需要继承下载模块的基类，仅需提供下载相关代码。

如需拓展上传的网站，或者修改上传的具体实现，修改`upload.py`文件

实现了一套基于装饰器的事件驱动框架，用装饰器保证可读性和易用性。在现有的功能上增加其他功能，需要注册发生对应事件时执行的函数，比如下载后转码

```python
# e.p.给函数注册事件
# block=True 会使用子线程来执行
# block=False 使用主线程执行
@event_manager.register("transcoding", block=True)
def modify(self, live_m):
    pass
```

## Credits
* Thanks `zhangn1985/ykdl` provides Douyu-downloader.

类似项目`ZhangMingZhao1/StreamerHelper`
