import stream_gears
import sys
import multiprocessing
import time


class Segment:
    pass


def download_worker():
    """在子进程中执行下载任务"""
    try:
        segment = Segment()
        segment.time = 60 * 1 # secs(1 min)
        # segment.size = 6000 * 1024 * 1024
        segment.size = 1000 * 1000 * 10 # bytes(10MB)
        url = ""

        stream_gears.download(
            url = url,
            header_map = {
                "referer": "https://live.bilibili.com",
                "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"
            },
            file_name = "new_test%Y-%m-%dT%H_%M_%S",
            segment = segment,
            proxy = None
        )
        return True
    except Exception as e:
        print(f"下载过程中发生错误: {e}")
        return False


def download_with_signal_handling():
    """使用多进程包装下载函数，支持 Ctrl+C 中断"""
    print("开始下载，按 Ctrl+C 可以退出...")

    # 创建下载进程
    download_process = multiprocessing.Process(target=download_worker)
    download_process.start()

    try:
        # 主进程等待下载进程完成，同时监听信号
        while download_process.is_alive():
            time.sleep(0.1)  # 短暂休眠，避免占用过多 CPU

        # 检查下载进程的退出状态
        download_process.join()
        if download_process.exitcode == 0:
            print("下载完成")
        else:
            print("下载进程异常退出")

    except KeyboardInterrupt:
        print("\n收到中断信号，正在终止下载...")
        # 终止下载进程
        download_process.terminate()
        # 等待进程真正结束
        download_process.join(timeout=5)
        if download_process.is_alive():
            print("强制杀死下载进程...")
            download_process.kill()
            download_process.join()
        print("下载已被用户中断")
        sys.exit(0)


if __name__ == '__main__':
    # Windows 上需要这个来支持 multiprocessing
    multiprocessing.freeze_support()
    download_with_signal_handling()
