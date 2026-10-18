//! 控制台首页的系统状态：CPU、内存、录制目录所在磁盘、网速。
//!
//! CPU 占用和网速都是相邻两次刷新的差值，必须有一个常驻的采样方：服务启动时起一个采样任务，
//! 每 [`SAMPLE_INTERVAL`] 采一次，最近 [`HISTORY`] 的采样留在内存里，首页一打开就能画出
//! 最近几分钟的曲线。`GET /v1/system-stats?since=` 只返回比 `since` 新的采样，轮询只传增量。
//!
//! 各项数字的口径：
//! - CPU：整机所有逻辑核的平均占用。容器里读到的也是宿主机的数，与容器内 `top` 一致。
//! - 内存：已用 = 总量 - 可用，可回收的页缓存不算已用。Linux 容器设了内存上限时改报 cgroup 的
//!   上限与匿名内存：录制会把页缓存写满到上限，按 `memory.current` 算会一直显示将满。
//! - 磁盘：录制文件都写在工作目录下，直接对它调 statvfs / GetDiskFreeSpaceExW，
//!   录制目录挂在 NFS / SMB 上也能反映；已用比例与 `df` 同口径（前端按 used / (used + available) 算）。
//! - 网速：参与统计的网卡收发字节差 / 实测时间差。docker0、veth、网桥、bond 与物理网卡走的是
//!   同一份流量，全加起来会重复计数，所以只统计物理网卡；一块物理网卡都没有时
//!   （bridge 网络的容器里只有 veth）退回统计全部非回环网卡。

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use sysinfo::{CGroupLimits, IpNetwork, Networks, System};
#[cfg(target_os = "linux")]
use sysinfo::{DiskRefreshKind, Disks};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tracing::{debug, error, warn};

/// 采样间隔
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
/// 服务端保留的历史时长
pub const HISTORY: Duration = Duration::from_secs(5 * 60);
/// 单次磁盘查询的等待上限。挂住的网络盘不能拖住 CPU / 网速的采样
const DISK_PROBE_TIMEOUT: Duration = Duration::from_secs(1);
/// 磁盘读数超过这个时长没有刷新（查询一直卡着）就不再报告，免得界面停在一个过期的数上
const DISK_STALE_AFTER: Duration = Duration::from_secs(30);

/// 没有 sysfs 可查的平台上按名字认出的虚拟网卡：Hyper-V 虚拟交换机（WSL / Docker Desktop）、
/// VMware / VirtualBox 的主机网卡，以及 macOS 的 AWDL（隔空投送）、低延迟 WLAN、网桥与 anpi。
/// 它们转发的流量同时算在物理网卡上。
const VIRTUAL_NIC_PREFIXES: &[&str] = &[
    "vEthernet",
    "VMware",
    "VirtualBox",
    "awdl",
    "llw",
    "bridge",
    "anpi",
];

/// 一次采样
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// 采样时刻，Unix 毫秒；严格递增，可直接当 `since` 用
    pub ts: i64,
    /// 整机 CPU 占用，0–100
    pub cpu: f32,
    /// 下行速率（字节/秒）
    pub rx: u64,
    /// 上行速率（字节/秒）
    pub tx: u64,
    /// 录制目录所在设备的读写速率；不支持、首次采样或计数器重置时为 null
    #[serde(default)]
    pub disk_read_bytes_per_sec: Option<u64>,
    #[serde(default)]
    pub disk_write_bytes_per_sec: Option<u64>,
}

/// 内存用量（字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub used: u64,
    pub total: u64,
    /// 报的是容器（cgroup）的内存上限而不是整机内存
    pub limited: bool,
}

/// 录制目录所在文件系统的容量（字节）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskUsage {
    /// 录制目录
    pub path: String,
    pub total: u64,
    /// 已用（总量 - 空闲）
    pub used: u64,
    /// 本进程还能写入的空间，不含文件系统给 root 的保留块
    pub available: u64,
}

/// CPU 规格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuInfo {
    /// 逻辑核数
    pub logical: usize,
    /// 物理核数；平台读不到时为 `null`
    pub physical: Option<usize>,
}

/// `GET /v1/system-stats` 的响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// 服务端当前时刻，Unix 毫秒；前端据此判断采样是否已经停了
    pub ts: i64,
    /// 采样间隔（毫秒）
    pub interval_ms: u64,
    /// 服务端保留的历史时长（毫秒）
    pub history_ms: u64,
    /// 首次采样前为 `null`
    pub cpu: Option<CpuInfo>,
    /// 最近一次采样的内存用量；首次采样前为 `null`
    pub memory: Option<MemoryUsage>,
    /// 查询失败或一直卡住时为 `null`
    pub disk: Option<DiskUsage>,
    /// 参与网速统计的网卡
    pub interfaces: Vec<String>,
    /// 比 `since` 新的采样，按时间升序
    pub samples: Vec<Sample>,
}

/// 一次刷新读出的数值
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reading {
    pub(crate) cpu: f32,
    pub(crate) rx: u64,
    pub(crate) tx: u64,
    pub(crate) disk_read_bytes_per_sec: Option<u64>,
    pub(crate) disk_write_bytes_per_sec: Option<u64>,
    pub(crate) memory: Option<MemoryUsage>,
    pub(crate) interfaces: Vec<String>,
    pub(crate) cores: CpuInfo,
}

#[derive(Default)]
struct State {
    samples: VecDeque<Sample>,
    cpu: Option<CpuInfo>,
    memory: Option<MemoryUsage>,
    disk: Option<DiskUsage>,
    interfaces: Vec<String>,
}

/// 系统状态的采样历史，所有请求共用一份。
pub struct SystemMonitor {
    interval: Duration,
    capacity: usize,
    state: Mutex<State>,
}

impl SystemMonitor {
    fn new(interval: Duration) -> Self {
        let capacity = (HISTORY.as_millis() / interval.as_millis().max(1)).max(1) as usize;
        Self {
            interval,
            capacity,
            state: Mutex::default(),
        }
    }

    /// 创建监视器并启动采样任务。任务只持有弱引用，监视器被丢弃（服务停止）后在下一个周期退出。
    pub fn spawn() -> Arc<Self> {
        Self::start(SAMPLE_INTERVAL).0
    }

    fn start(interval: Duration) -> (Arc<Self>, JoinHandle<()>) {
        let monitor = Arc::new(Self::new(interval));
        let task = tokio::spawn(run(Arc::downgrade(&monitor), interval));
        (monitor, task)
    }

    /// 最新的读数与比 `since`（Unix 毫秒）新的采样；`since` 为空时给出全部历史。
    pub fn snapshot(&self, since: Option<i64>) -> SystemStats {
        let state = self.state.lock().unwrap();
        SystemStats {
            ts: now_ms(),
            interval_ms: self.interval.as_millis() as u64,
            history_ms: HISTORY.as_millis() as u64,
            cpu: state.cpu,
            memory: state.memory,
            disk: state.disk.clone(),
            interfaces: state.interfaces.clone(),
            samples: state
                .samples
                .iter()
                .filter(|sample| since.is_none_or(|since| sample.ts > since))
                .copied()
                .collect(),
        }
    }

    pub(crate) fn record(&self, ts: i64, reading: Reading, disk: Option<DiskUsage>) {
        let mut state = self.state.lock().unwrap();
        // 系统时钟往回调时也保持递增，否则前端按 `since` 增量拉取会漏掉采样
        let ts = state.samples.back().map_or(ts, |last| ts.max(last.ts + 1));
        if state.samples.len() >= self.capacity {
            state.samples.pop_front();
        }
        state.samples.push_back(Sample {
            ts,
            cpu: reading.cpu,
            rx: reading.rx,
            tx: reading.tx,
            disk_read_bytes_per_sec: reading.disk_read_bytes_per_sec,
            disk_write_bytes_per_sec: reading.disk_write_bytes_per_sec,
        });
        state.cpu = Some(reading.cores);
        state.memory = reading.memory;
        state.interfaces = reading.interfaces;
        state.disk = disk;
    }
}

async fn run(weak: Weak<SystemMonitor>, interval: Duration) {
    // sysinfo 读 /proc、/sys 或调系统 API，都是阻塞调用，放到阻塞线程里做
    let mut sampler = match tokio::task::spawn_blocking(Sampler::new).await {
        Ok(sampler) => sampler,
        Err(e) => {
            error!(error = %e, "系统状态采样初始化失败");
            return;
        }
    };
    let mut disk = DiskProbe::new();
    let mut disk_io = DiskIoProbe::default();
    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // 第一次 tick 立即完成；基线刚刷新过，从下一次开始读差值
    tick.tick().await;
    loop {
        tick.tick().await;
        if weak.strong_count() == 0 {
            break;
        }
        let (returned, mut reading) = match tokio::task::spawn_blocking(move || {
            let reading = sampler.read();
            (sampler, reading)
        })
        .await
        {
            Ok(result) => result,
            Err(e) => {
                error!(error = %e, "系统状态采样失败，停止采样");
                return;
            }
        };
        sampler = returned;
        let ts = now_ms();
        let (disk_usage, rates) = tokio::join!(disk.poll(), disk_io.poll());
        reading.disk_read_bytes_per_sec = rates.read;
        reading.disk_write_bytes_per_sec = rates.write;
        let Some(monitor) = weak.upgrade() else {
            break;
        };
        monitor.record(ts, reading, disk_usage);
    }
    debug!("系统状态采样任务退出");
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// sysinfo 的状态；CPU 占用与网卡流量都是相邻两次刷新的差值。
struct Sampler {
    system: System,
    networks: Networks,
    refreshed_at: Instant,
    cores: CpuInfo,
}

impl Sampler {
    /// 刷新一次作为基线
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        let cores = CpuInfo {
            logical: system.cpus().len(),
            physical: System::physical_core_count(),
        };
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            refreshed_at: Instant::now(),
            cores,
        }
    }

    fn read(&mut self) -> Reading {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        let now = Instant::now();
        let elapsed = now.duration_since(self.refreshed_at);
        self.refreshed_at = now;

        let nics: Vec<NicInfo> = self
            .networks
            .iter()
            .map(|(name, data)| NicInfo {
                name,
                loopback: is_loopback(name, data.ip_networks()),
                has_mac: !data.mac_address().is_unspecified(),
                is_virtual: is_virtual_nic(name),
            })
            .collect();
        let interfaces = select_interfaces(&nics);
        let (mut received, mut transmitted) = (0u64, 0u64);
        for name in &interfaces {
            if let Some(data) = self.networks.get(*name) {
                received = received.saturating_add(data.received());
                transmitted = transmitted.saturating_add(data.transmitted());
            }
        }

        let cpu = self.system.global_cpu_usage();
        Reading {
            cpu: if cpu.is_finite() {
                (cpu.clamp(0.0, 100.0) * 10.0).round() / 10.0
            } else {
                0.0
            },
            rx: per_second(received, elapsed),
            tx: per_second(transmitted, elapsed),
            disk_read_bytes_per_sec: None,
            disk_write_bytes_per_sec: None,
            memory: memory_usage(
                self.system.total_memory(),
                self.system.available_memory(),
                self.system.cgroup_limits(),
            ),
            interfaces: interfaces.into_iter().map(str::to_owned).collect(),
            cores: self.cores,
        }
    }
}

fn per_second(bytes: u64, elapsed: Duration) -> u64 {
    if elapsed.is_zero() {
        return 0;
    }
    (bytes as f64 / elapsed.as_secs_f64()).round() as u64
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DiskRates {
    read: Option<u64>,
    write: Option<u64>,
}

#[derive(Default)]
struct DiskIoSampler {
    #[cfg(target_os = "linux")]
    disks: Disks,
    previous: Option<(String, Instant, u64, u64)>,
}

impl DiskIoSampler {
    fn read(&mut self) -> DiskRates {
        let counters = self.counters();
        self.record(counters, Instant::now())
    }

    fn record(&mut self, counters: Option<(String, u64, u64)>, now: Instant) -> DiskRates {
        let previous = self.previous.take();
        let Some((device, read, write)) = counters else {
            return DiskRates::default();
        };
        self.previous = Some((device.clone(), now, read, write));
        let Some((last_device, at, last_read, last_write)) = previous else {
            return DiskRates::default();
        };
        let elapsed = now.saturating_duration_since(at);
        if device != last_device || elapsed.is_zero() || read < last_read || write < last_write {
            return DiskRates::default();
        }
        DiskRates {
            read: Some(per_second(read - last_read, elapsed)),
            write: Some(per_second(write - last_write, elapsed)),
        }
    }

    #[cfg(target_os = "linux")]
    fn counters(&mut self) -> Option<(String, u64, u64)> {
        let path = std::env::current_dir().ok()?;
        // Include all mounts here: sysinfo skips network/tmpfs mounts by default.
        // Their enclosing local disk must never be reported as their IO device.
        let mounts = std::fs::read_to_string("/proc/self/mounts").ok()?;
        let mount = recording_mount(&path, &mounts)?;
        // refresh_specifics discovers mounts even when the initial list is empty.
        // Keep all enumeration inside the worker; no statvfs or implicit IO fallback.
        self.disks
            .refresh_specifics(true, DiskRefreshKind::nothing());
        let disk = self
            .disks
            .list()
            .iter()
            .find(|disk| disk.mount_point() == mount)?;
        let device = PathBuf::from(disk.name());
        let actual = std::fs::canonicalize(&device).unwrap_or(device);
        let name = actual.strip_prefix("/dev").ok()?.to_str()?;
        // Disk::usage returns zero when the OS counter is unavailable and does
        // not expose success. Read that same kernel source explicitly so a
        // failed counter refresh can never look like an idle device.
        let stats = std::fs::read_to_string("/proc/diskstats").ok()?;
        let (read, write) = disk_cumulative_bytes(&stats, name)?;
        Some((
            format!("{}:{}", actual.display(), mount.display()),
            read,
            write,
        ))
    }

    #[cfg(windows)]
    fn counters(&mut self) -> Option<(String, u64, u64)> {
        windows_disk_counters(&std::env::current_dir().ok()?).ok()
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    fn counters(&mut self) -> Option<(String, u64, u64)> {
        // sysinfo does not expose whether these platforms' IO query succeeded;
        // reporting its fallback zeros would turn unavailable counters into idle.
        None
    }
}

#[cfg(windows)]
fn windows_disk_counters(path: &Path) -> io::Result<(String, u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, GetVolumeNameForVolumeMountPointW,
            GetVolumePathNameW, OPEN_EXISTING,
        },
        System::{
            IO::DeviceIoControl,
            Ioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE},
        },
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut mount = vec![0u16; 32768];
    let mut volume = vec![0u16; 1024];
    // SAFETY: all input strings are NUL-terminated; output lengths match buffers.
    if unsafe { GetVolumePathNameW(wide.as_ptr(), mount.as_mut_ptr(), mount.len() as u32) } == 0
        || unsafe {
            GetVolumeNameForVolumeMountPointW(
                mount.as_ptr(),
                volume.as_mut_ptr(),
                volume.len() as u32,
            )
        } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let (identity, volume) = windows_volume_device(&volume)?;
    // Access 0 permits querying performance on ordinary user handles where the
    // OS/driver supports it. No write access or disk-perf configuration changes.
    let handle = unsafe {
        CreateFileW(
            volume.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut performance = std::mem::MaybeUninit::<DISK_PERFORMANCE>::zeroed();
    let mut returned = 0u32;
    // SAFETY: the ioctl writes only to a buffer sized for DISK_PERFORMANCE.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_PERFORMANCE,
            std::ptr::null(),
            0,
            performance.as_mut_ptr().cast(),
            std::mem::size_of::<DISK_PERFORMANCE>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    let error = (ok == 0).then(io::Error::last_os_error);
    unsafe { CloseHandle(handle) };
    if let Some(error) = error {
        return Err(error);
    }
    if returned < std::mem::size_of::<DISK_PERFORMANCE>() as u32 {
        return Err(io::Error::other("磁盘性能计数器返回了不完整的数据"));
    }
    let performance = unsafe { performance.assume_init() };
    let (read, write) = windows_performance_bytes(performance.BytesRead, performance.BytesWritten)?;
    Ok((identity, read, write))
}

#[cfg(any(windows, test))]
fn windows_volume_device(volume: &[u16]) -> io::Result<(String, Vec<u16>)> {
    let end = volume
        .iter()
        .position(|value| *value == 0)
        .ok_or_else(|| io::Error::other("卷 GUID 路径没有终止符"))?;
    let mut device = volume[..end].to_vec();
    if device.last() == Some(&u16::from(b'\\')) {
        device.pop();
    }
    let identity = String::from_utf16(&device).map_err(io::Error::other)?;
    if !identity.starts_with("\\\\?\\Volume{") || !identity.ends_with('}') {
        return Err(io::Error::other("没有可查询的本地卷 GUID"));
    }
    device.push(0);
    Ok((identity, device))
}

#[cfg(any(windows, test))]
fn windows_performance_bytes(read: i64, write: i64) -> io::Result<(u64, u64)> {
    Ok((
        u64::try_from(read).map_err(io::Error::other)?,
        u64::try_from(write).map_err(io::Error::other)?,
    ))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn recording_mount(path: &Path, mounts: &str) -> Option<PathBuf> {
    mounts
        .lines()
        .filter_map(|line| {
            let mount = line.split_whitespace().nth(1)?;
            let mount = PathBuf::from(
                mount
                    .replace("\\040", " ")
                    .replace("\\011", "\t")
                    .replace("\\012", "\n")
                    .replace("\\134", "\\"),
            );
            path.starts_with(&mount).then_some(mount)
        })
        .max_by_key(|mount| mount.components().count())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn disk_cumulative_bytes(stats: &str, name: &str) -> Option<(u64, u64)> {
    stats.lines().find_map(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.get(2) != Some(&name) {
            return None;
        }
        // Linux reports diskstats sectors in fixed 512-byte units regardless
        // of the physical/logical block size of the disk.
        Some((
            fields.get(5)?.parse::<u64>().ok()?.checked_mul(512)?,
            fields.get(9)?.parse::<u64>().ok()?.checked_mul(512)?,
        ))
    })
}

/// Capacity and IO probes run separately. A blocked remote filesystem keeps at
/// most one worker occupied and cannot stop CPU/network sampling.
#[derive(Default)]
struct DiskIoProbe {
    sampler: Option<DiskIoSampler>,
    in_flight: Option<JoinHandle<(DiskIoSampler, DiskRates)>>,
}

impl DiskIoProbe {
    async fn poll(&mut self) -> DiskRates {
        if self.in_flight.is_none() {
            let mut sampler = self.sampler.take().unwrap_or_default();
            self.in_flight = Some(tokio::task::spawn_blocking(move || {
                let rates = sampler.read();
                (sampler, rates)
            }));
        }
        let handle = self.in_flight.as_mut().unwrap();
        match tokio::time::timeout(Duration::from_millis(100), handle).await {
            Ok(result) => {
                self.in_flight = None;
                match result {
                    Ok((sampler, rates)) => {
                        self.sampler = Some(sampler);
                        rates
                    }
                    Err(error) => {
                        debug!(%error, "磁盘 IO 采样失败");
                        DiskRates::default()
                    }
                }
            }
            Err(_) => DiskRates::default(),
        }
    }
}

/// 容器设了内存上限（小于整机内存）时报 cgroup 口径，否则报整机口径。
fn memory_usage(total: u64, available: u64, cgroup: Option<CGroupLimits>) -> Option<MemoryUsage> {
    if let Some(limits) = cgroup
        && limits.total_memory > 0
        && limits.total_memory < total
    {
        return Some(MemoryUsage {
            used: limits.rss.min(limits.total_memory),
            total: limits.total_memory,
            limited: true,
        });
    }
    (total > 0).then(|| MemoryUsage {
        used: total.saturating_sub(available),
        total,
        limited: false,
    })
}

/// 选网卡用到的几个属性
struct NicInfo<'a> {
    name: &'a str,
    loopback: bool,
    /// 有硬件地址。回环与 tun / wg 这类三层隧道没有
    has_mac: bool,
    is_virtual: bool,
}

/// 参与网速统计的网卡（按名字排序）：优先物理网卡；一块都没有时退回全部非回环网卡。
fn select_interfaces<'a>(nics: &[NicInfo<'a>]) -> Vec<&'a str> {
    let mut physical: Vec<&str> = nics
        .iter()
        .filter(|nic| !nic.loopback && nic.has_mac && !nic.is_virtual)
        .map(|nic| nic.name)
        .collect();
    if physical.is_empty() {
        physical = nics
            .iter()
            .filter(|nic| !nic.loopback)
            .map(|nic| nic.name)
            .collect();
    }
    physical.sort_unstable();
    physical
}

fn is_loopback(name: &str, networks: &[IpNetwork]) -> bool {
    name == "lo"
        || name == "lo0"
        || (!networks.is_empty() && networks.iter().all(|net| net.addr.is_loopback()))
}

fn is_virtual_nic(name: &str) -> bool {
    VIRTUAL_NIC_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
        || sysfs_is_virtual(name)
}

/// Linux 上的虚拟网卡（docker0、veth、网桥、bond、VLAN、tun、wg 等）在 sysfs 里挂在
/// `/sys/devices/virtual/net` 下，物理网卡挂在各自的总线设备下。
#[cfg(any(target_os = "linux", target_os = "android"))]
fn sysfs_is_virtual(name: &str) -> bool {
    std::fs::read_link(Path::new("/sys/class/net").join(name))
        .is_ok_and(|target| link_target_is_virtual(&target))
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn sysfs_is_virtual(_name: &str) -> bool {
    false
}

/// `/sys/class/net/<name>` 链接目标形如 `../../devices/virtual/net/docker0`
#[cfg_attr(not(any(target_os = "linux", target_os = "android")), allow(dead_code))]
fn link_target_is_virtual(target: &Path) -> bool {
    let parts: Vec<_> = target.components().map(|part| part.as_os_str()).collect();
    parts
        .windows(2)
        .any(|pair| pair[0] == "devices" && pair[1] == "virtual")
}

/// 录制目录的磁盘查询。查询放在阻塞线程里并限时等待；上一次还没返回（例如网络盘挂住）就不再发起，
/// 最多只有一个线程被卡住，阻塞线程池不会被一次次的查询占满。
struct DiskProbe {
    probe: fn() -> Option<DiskUsage>,
    timeout: Duration,
    stale_after: Duration,
    in_flight: Option<JoinHandle<Option<DiskUsage>>>,
    last: Option<(Instant, DiskUsage)>,
}

impl DiskProbe {
    fn new() -> Self {
        Self::with(probe_disk, DISK_PROBE_TIMEOUT, DISK_STALE_AFTER)
    }

    fn with(probe: fn() -> Option<DiskUsage>, timeout: Duration, stale_after: Duration) -> Self {
        Self {
            probe,
            timeout,
            stale_after,
            in_flight: None,
            last: None,
        }
    }

    async fn poll(&mut self) -> Option<DiskUsage> {
        let probe = self.probe;
        let handle = self
            .in_flight
            .get_or_insert_with(|| tokio::task::spawn_blocking(probe));
        // 超时就让它继续跑，下个周期接着等同一个查询
        if let Ok(joined) = tokio::time::timeout(self.timeout, handle).await {
            self.in_flight = None;
            self.last = match joined {
                Ok(usage) => usage.map(|usage| (Instant::now(), usage)),
                Err(e) => {
                    warn!(error = %e, "查询录制目录磁盘容量的任务异常退出");
                    None
                }
            };
        }
        self.last
            .as_ref()
            .filter(|(at, _)| at.elapsed() < self.stale_after)
            .map(|(_, usage)| usage.clone())
    }
}

/// 录制文件写在工作目录下（下载器的输出目录是 `.`），查它所在文件系统的容量。
fn probe_disk() -> Option<DiskUsage> {
    let dir = std::env::current_dir()
        .inspect_err(|e| debug!(error = %e, "读取工作目录失败"))
        .ok()?;
    disk_usage(&dir)
        .inspect_err(|e| debug!(error = %e, path = %dir.display(), "读取录制目录磁盘容量失败"))
        .ok()
}

fn disk_usage(path: &Path) -> io::Result<DiskUsage> {
    let space = disk_space(path)?;
    // 伪文件系统（proc、sysfs 等）报 0 容量
    if space.total == 0 {
        return Err(io::Error::other("文件系统没有报告容量"));
    }
    Ok(DiskUsage {
        path: path.display().to_string(),
        total: space.total,
        used: space.total.saturating_sub(space.free),
        available: space.available.min(space.total),
    })
}

/// 文件系统容量（字节）
struct Space {
    total: u64,
    /// 全部空闲，含给 root 的保留块
    free: u64,
    /// 本进程可用
    available: u64,
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // statvfs 各字段的宽度随平台不同（u32 / u64）
fn disk_space(path: &Path) -> io::Result<Space> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    loop {
        // SAFETY: c_path 以 NUL 结尾，statvfs 只写 stat 指向的结构体
        if unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0 {
            break;
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
    // SAFETY: statvfs 返回 0 时已填好整个结构体
    let stat = unsafe { stat.assume_init() };
    // 块数以 f_frsize 为单位；个别平台不填 f_frsize，按 POSIX 退回 f_bsize
    let block = if stat.f_frsize != 0 {
        stat.f_frsize as u64
    } else {
        stat.f_bsize as u64
    };
    Ok(Space {
        total: block.saturating_mul(stat.f_blocks as u64),
        free: block.saturating_mul(stat.f_bfree as u64),
        available: block.saturating_mul(stat.f_bavail as u64),
    })
}

#[cfg(windows)]
fn disk_space(path: &Path) -> io::Result<Space> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    // UNC 路径必须以反斜杠结尾（\\server\share\），普通目录带不带都行
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if !matches!(wide.last(), Some(&c) if c == u16::from(b'\\') || c == u16::from(b'/')) {
        wide.push(u16::from(b'\\'));
    }
    wide.push(0);
    let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: wide 以 NUL 结尾，三个输出指针都指向本函数里的 u64
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Space {
        total,
        free,
        available,
    })
}

#[cfg(not(any(unix, windows)))]
fn disk_space(_path: &Path) -> io::Result<Space> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn nic(name: &str, loopback: bool, has_mac: bool, is_virtual: bool) -> NicInfo<'_> {
        NicInfo {
            name,
            loopback,
            has_mac,
            is_virtual,
        }
    }

    /// 装了 docker 的宿主机：docker0 / veth / 网桥转发的流量同时算在物理网卡上，只统计物理网卡
    #[test]
    fn physical_nics_are_preferred_over_virtual_ones() {
        let nics = [
            nic("lo", true, false, true),
            nic("wlan0", false, true, false),
            nic("docker0", false, true, true),
            nic("veth1a2b3c", false, true, true),
            nic("br0", false, true, true),
            nic("eth0", false, true, false),
            nic("wg0", false, false, true),
        ];
        assert_eq!(select_interfaces(&nics), ["eth0", "wlan0"]);
    }

    /// bridge 网络的容器里只有 veth（eth0）和 lo：退回统计全部非回环网卡
    #[test]
    fn without_a_physical_nic_every_non_loopback_nic_counts() {
        let nics = [nic("lo", true, false, true), nic("eth0", false, true, true)];
        assert_eq!(select_interfaces(&nics), ["eth0"]);
        // 只有隧道网卡（没有硬件地址）时也一样
        let nics = [nic("lo", true, false, true), nic("wg0", false, false, true)];
        assert_eq!(select_interfaces(&nics), ["wg0"]);
    }

    /// macOS 的 utun（VPN）没有硬件地址，流量同时算在 en0 上
    #[test]
    fn nics_without_a_hardware_address_are_not_physical() {
        let nics = [
            nic("lo0", true, false, false),
            nic("en0", false, true, false),
            nic("utun3", false, false, false),
        ];
        assert_eq!(select_interfaces(&nics), ["en0"]);
    }

    #[test]
    fn loopback_is_never_counted() {
        let nics = [nic("lo", true, false, true)];
        assert!(select_interfaces(&nics).is_empty());
        assert!(select_interfaces(&[]).is_empty());
    }

    #[test]
    fn loopback_is_recognised_by_name_or_addresses() {
        let net = |addr: IpAddr| IpNetwork { addr, prefix: 8 };
        assert!(is_loopback("lo", &[]));
        assert!(is_loopback("lo0", &[]));
        assert!(is_loopback(
            "Loopback Pseudo-Interface 1",
            &[
                net(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                net(IpAddr::V6(Ipv6Addr::LOCALHOST))
            ]
        ));
        assert!(!is_loopback("eth0", &[]));
        assert!(!is_loopback(
            "eth0",
            &[
                net(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                net(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)))
            ]
        ));
    }

    #[test]
    fn sysfs_links_under_devices_virtual_are_virtual() {
        assert!(link_target_is_virtual(Path::new(
            "../../devices/virtual/net/docker0"
        )));
        assert!(!link_target_is_virtual(Path::new(
            "../../devices/pci0000:00/0000:00:1f.6/net/eth0"
        )));
        // 路径里某一段恰好叫 virtual 不算
        assert!(!link_target_is_virtual(Path::new(
            "../../devices/platform/virtual-eth/net/eth0"
        )));
    }

    #[test]
    fn known_virtual_adapters_are_recognised_by_name() {
        for name in [
            "vEthernet (WSL (Hyper-V firewall))",
            "VMware Network Adapter VMnet8",
            "VirtualBox Host-Only Network",
            "awdl0",
            "llw0",
            "bridge0",
        ] {
            assert!(is_virtual_nic(name), "{name}");
        }
        for name in ["以太网", "WLAN", "en0"] {
            assert!(!is_virtual_nic(name), "{name}");
        }
    }

    /// 录制会把页缓存写满到容器上限；报匿名内存而不是 memory.current
    #[test]
    fn a_container_memory_limit_replaces_the_host_numbers() {
        let gib = 1u64 << 30;
        let limits = CGroupLimits {
            total_memory: 2 * gib,
            free_memory: 0,
            rss: gib / 2,
            ..Default::default()
        };
        assert_eq!(
            memory_usage(64 * gib, 40 * gib, Some(limits)),
            Some(MemoryUsage {
                used: gib / 2,
                total: 2 * gib,
                limited: true
            })
        );
    }

    /// 没设上限时 cgroup 报的就是整机内存：按整机口径，已用 = 总量 - 可用
    #[test]
    fn without_a_limit_the_host_numbers_are_used() {
        let gib = 1u64 << 30;
        let unlimited = CGroupLimits {
            total_memory: 16 * gib,
            free_memory: gib,
            rss: 3 * gib,
            ..Default::default()
        };
        let host = MemoryUsage {
            used: 6 * gib,
            total: 16 * gib,
            limited: false,
        };
        assert_eq!(
            memory_usage(16 * gib, 10 * gib, Some(unlimited)),
            Some(host)
        );
        assert_eq!(memory_usage(16 * gib, 10 * gib, None), Some(host));
        assert_eq!(memory_usage(0, 0, None), None);
    }

    #[test]
    fn rates_divide_by_the_measured_interval() {
        assert_eq!(per_second(4_000, Duration::from_secs(2)), 2_000);
        assert_eq!(per_second(1_000, Duration::from_millis(2_500)), 400);
        assert_eq!(per_second(1_000, Duration::ZERO), 0);
    }

    #[test]
    fn the_disk_of_a_real_directory_has_consistent_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let usage = disk_usage(dir.path()).expect("临时目录所在磁盘应能查到容量");
        assert!(usage.total > 0);
        assert!(usage.used <= usage.total);
        assert!(usage.available <= usage.total);
        assert_eq!(usage.path, dir.path().display().to_string());
        assert!(disk_usage(&dir.path().join("does-not-exist")).is_err());
    }

    fn reading(cpu: f32) -> Reading {
        Reading {
            cpu,
            rx: 10,
            tx: 20,
            disk_read_bytes_per_sec: Some(30),
            disk_write_bytes_per_sec: Some(40),
            memory: Some(MemoryUsage {
                used: 1,
                total: 2,
                limited: false,
            }),
            interfaces: vec!["eth0".into()],
            cores: CpuInfo {
                logical: 8,
                physical: Some(4),
            },
        }
    }

    #[test]
    fn history_is_bounded_and_can_be_read_incrementally() {
        let monitor = SystemMonitor::new(SAMPLE_INTERVAL);
        assert_eq!(monitor.capacity, 150);
        let empty = monitor.snapshot(None);
        assert!(empty.samples.is_empty());
        assert_eq!(empty.cpu, None);
        assert_eq!(empty.memory, None);

        for i in 0..200 {
            monitor.record(1_000 + i, reading(i as f32 / 10.0), None);
        }
        let all = monitor.snapshot(None);
        assert_eq!(all.samples.len(), 150);
        assert_eq!(all.samples.first().unwrap().ts, 1_050);
        assert_eq!(all.samples.last().unwrap().ts, 1_199);
        assert_eq!(all.interval_ms, 2_000);
        assert_eq!(all.history_ms, 300_000);
        assert_eq!(all.interfaces, ["eth0"]);
        assert_eq!(all.cpu.unwrap().logical, 8);

        let newer = monitor.snapshot(Some(1_197));
        assert_eq!(
            newer.samples.iter().map(|s| s.ts).collect::<Vec<_>>(),
            [1_198, 1_199]
        );
        assert!(monitor.snapshot(Some(1_199)).samples.is_empty());
    }

    #[test]
    fn timestamps_keep_increasing_when_the_clock_steps_back() {
        let monitor = SystemMonitor::new(SAMPLE_INTERVAL);
        monitor.record(5_000, reading(1.0), None);
        monitor.record(4_000, reading(2.0), None);
        monitor.record(4_000, reading(3.0), None);
        let ts: Vec<_> = monitor
            .snapshot(None)
            .samples
            .iter()
            .map(|s| s.ts)
            .collect();
        assert_eq!(ts, [5_000, 5_001, 5_002]);
    }

    /// 真实采样：历史里陆续出现读数；监视器被丢弃后采样任务退出
    #[tokio::test]
    async fn the_sampler_fills_the_history_and_stops_with_the_monitor() {
        let (monitor, task) = SystemMonitor::start(Duration::from_millis(50));
        let stats = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let stats = monitor.snapshot(None);
                if stats.samples.len() >= 2 && stats.disk.is_some() {
                    break stats;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("采样任务应陆续写入读数");
        let cpu = stats.cpu.expect("采样后应有 CPU 规格");
        assert!(cpu.logical > 0);
        for sample in &stats.samples {
            assert!((0.0..=100.0).contains(&sample.cpu));
        }
        let memory = stats.memory.expect("采样后应有内存读数");
        assert!(memory.total > 0 && memory.used <= memory.total);

        drop(monitor);
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("监视器被丢弃后采样任务应退出")
            .unwrap();
    }

    /// 其他进程拿到的 `GET /v1/system-stats` 响应能原样读回，再序列化逐字节一致
    #[test]
    fn stats_survive_a_json_round_trip() {
        let full = SystemStats {
            ts: 1_700_000_000_123,
            interval_ms: 2_000,
            history_ms: 300_000,
            cpu: Some(CpuInfo {
                logical: 8,
                physical: None,
            }),
            memory: Some(MemoryUsage {
                used: 3 << 30,
                total: 16 << 30,
                limited: true,
            }),
            disk: Some(DiskUsage {
                path: "/srv/录播".into(),
                total: 1 << 40,
                used: 1 << 39,
                available: 1 << 38,
            }),
            interfaces: vec!["eth0".into(), "wlan0".into()],
            samples: vec![
                Sample {
                    ts: 1_700_000_000_000,
                    cpu: 12.3,
                    rx: 1_024,
                    tx: 0,
                    disk_read_bytes_per_sec: None,
                    disk_write_bytes_per_sec: Some(512),
                },
                Sample {
                    ts: 1_700_000_002_000,
                    cpu: 100.0,
                    rx: u64::MAX,
                    tx: 7,
                    disk_read_bytes_per_sec: Some(0),
                    disk_write_bytes_per_sec: None,
                },
            ],
        };
        let json = serde_json::to_string(&full).unwrap();
        assert_eq!(
            json,
            r#"{"ts":1700000000123,"interval_ms":2000,"history_ms":300000,"cpu":{"logical":8,"physical":null},"memory":{"used":3221225472,"total":17179869184,"limited":true},"disk":{"path":"/srv/录播","total":1099511627776,"used":549755813888,"available":274877906944},"interfaces":["eth0","wlan0"],"samples":[{"ts":1700000000000,"cpu":12.3,"rx":1024,"tx":0,"disk_read_bytes_per_sec":null,"disk_write_bytes_per_sec":512},{"ts":1700000002000,"cpu":100.0,"rx":18446744073709551615,"tx":7,"disk_read_bytes_per_sec":0,"disk_write_bytes_per_sec":null}]}"#
        );

        let empty = SystemMonitor::new(SAMPLE_INTERVAL).snapshot(None);
        for stats in [full, empty] {
            let json = serde_json::to_string(&stats).unwrap();
            let parsed: SystemStats = serde_json::from_str(&json).unwrap();
            assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
            assert_eq!(parsed.cpu, stats.cpu);
            assert_eq!(parsed.memory, stats.memory);
            assert_eq!(parsed.disk, stats.disk);
            assert_eq!(parsed.samples, stats.samples);
        }
    }

    #[test]
    fn disk_io_uses_measured_elapsed_and_rebaselines_changes_and_resets() {
        let mut sampler = DiskIoSampler::default();
        let now = Instant::now();
        let count = |device: &str, read, write| Some((device.to_string(), read, write));
        assert_eq!(
            sampler.record(count("nvme", 100, 200), now),
            DiskRates::default()
        );
        assert_eq!(
            sampler.record(count("nvme", 600, 1200), now + Duration::from_millis(2500)),
            DiskRates {
                read: Some(200),
                write: Some(400)
            }
        );
        assert_eq!(
            sampler.record(count("nvme", 600, 1200), now + Duration::from_secs(4)),
            DiskRates {
                read: Some(0),
                write: Some(0)
            }
        );
        assert_eq!(
            sampler.record(count("sda", 600, 1200), now + Duration::from_secs(6)),
            DiskRates::default()
        );
        assert_eq!(
            sampler.record(count("sda", 10, 20), now + Duration::from_secs(8)),
            DiskRates::default()
        );
        assert_eq!(
            sampler.record(None, now + Duration::from_secs(10)),
            DiskRates::default()
        );
        assert_eq!(
            sampler.record(count("sda", 20, 40), now + Duration::from_secs(12)),
            DiskRates::default()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn disk_io_enumeration_is_lazy_and_refresh_fills_an_initially_empty_list() {
        let mut sampler = DiskIoSampler::default();
        assert!(
            sampler.disks.list().is_empty(),
            "construction must not perform blocking IO"
        );
        let counters = sampler.counters();
        // sysinfo intentionally skips network filesystems, but local mounts are
        // discovered by refresh_specifics without a separate initialization call.
        let local_mounts = std::fs::read_to_string("/proc/self/mounts").unwrap();
        if local_mounts.lines().any(|line| {
            line.split_whitespace()
                .nth(2)
                .is_some_and(|fs| matches!(fs, "ext4" | "xfs" | "btrfs" | "overlay"))
        }) {
            assert!(!sampler.disks.list().is_empty());
        }
        if let Some((_, read, write)) = counters {
            assert_eq!(read % 512, 0);
            assert_eq!(write % 512, 0);
        }
    }

    #[test]
    fn windows_disk_performance_fixture_keeps_real_zero_and_rejects_invalid_data() {
        let volume = "\\\\?\\Volume{550e8400-e29b-41d4-a716-446655440000}\\";
        let wide: Vec<u16> = volume.encode_utf16().chain(Some(0)).collect();
        let (identity, device) = windows_volume_device(&wide).unwrap();
        assert_eq!(identity, volume.trim_end_matches('\\'));
        assert_eq!(
            String::from_utf16(&device[..device.len() - 1]).unwrap(),
            identity
        );
        assert_eq!(windows_performance_bytes(0, 0).unwrap(), (0, 0));
        assert_eq!(
            windows_performance_bytes(1234567, 9876543).unwrap(),
            (1234567, 9876543)
        );
        assert!(windows_performance_bytes(-1, 0).is_err());
        assert!(windows_volume_device(&volume.encode_utf16().collect::<Vec<_>>()).is_err());
        let unc: Vec<u16> = "\\\\server\\share\\"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        assert!(windows_volume_device(&unc).is_err());
    }

    #[test]
    fn deepest_recording_mount_includes_unsupported_and_escaped_mounts() {
        let mounts = "/dev/sda / ext4 rw 0 0\nserver:/nas /mnt/nas nfs4 rw 0 0\n/dev/sdb /mnt/recording\\040disk ext4 rw 0 0\n";
        assert_eq!(
            recording_mount(Path::new("/mnt/nas/live"), mounts),
            Some(PathBuf::from("/mnt/nas"))
        );
        assert_eq!(
            recording_mount(Path::new("/mnt/nas2/live"), mounts),
            Some(PathBuf::from("/"))
        );
        assert_eq!(
            recording_mount(Path::new("/mnt/recording disk/live"), mounts),
            Some(PathBuf::from("/mnt/recording disk"))
        );
        assert_eq!(
            disk_cumulative_bytes("8 0 sda 0 0 0 0 0 0 0 0", "sda"),
            Some((0, 0))
        );
        assert_eq!(
            disk_cumulative_bytes("8 0 sda 0 0 12 0 0 0 24 0", "sda"),
            Some((6144, 12288))
        );
        assert_eq!(
            disk_cumulative_bytes("8 0 sda 0 0 0 0 0 0 0 0", "sdb"),
            None
        );
        assert_eq!(
            disk_cumulative_bytes("8 0 sda 0 0 nope 0 0 0 0 0", "sda"),
            None
        );
    }

    static HUNG_PROBE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static HUNG_PROBE_RELEASED: AtomicBool = AtomicBool::new(false);

    /// 模拟挂住的网络盘：直到测试放行才返回
    fn hung_probe() -> Option<DiskUsage> {
        HUNG_PROBE_CALLS.fetch_add(1, Ordering::SeqCst);
        while !HUNG_PROBE_RELEASED.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        Some(DiskUsage {
            path: "/mnt/nas".into(),
            total: 100,
            used: 40,
            available: 60,
        })
    }

    /// 查询卡住时不重复发起（阻塞线程不会越积越多）；恢复后读数回来，过期后不再报告
    #[tokio::test]
    async fn a_hung_disk_probe_is_not_restarted_and_stale_readings_expire() {
        let mut probe = DiskProbe::with(
            hung_probe,
            Duration::from_millis(20),
            Duration::from_millis(300),
        );
        assert_eq!(probe.poll().await, None);
        // 等阻塞线程真正开始执行查询，再确认之后的周期不会另起一个
        tokio::time::timeout(Duration::from_secs(5), async {
            while HUNG_PROBE_CALLS.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("查询应已开始");
        assert_eq!(probe.poll().await, None);
        assert_eq!(probe.poll().await, None);
        assert_eq!(
            HUNG_PROBE_CALLS.load(Ordering::SeqCst),
            1,
            "上一次查询没返回前不应再发起"
        );

        HUNG_PROBE_RELEASED.store(true, Ordering::SeqCst);
        let usage = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(usage) = probe.poll().await {
                    break usage;
                }
            }
        })
        .await
        .expect("放行后应拿到读数");
        assert_eq!(usage.available, 60);

        // 之后的查询一直拿不到新值：旧读数过了有效期就不再报告
        probe.in_flight = Some(tokio::spawn(std::future::pending()));
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert_eq!(probe.poll().await, None, "超过有效期的旧读数不再报告");
    }
}
