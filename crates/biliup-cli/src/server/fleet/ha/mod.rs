//! 一主一备（HA Pair，ha-pair 方案 H1）。
//!
//! 主机 = 控制面 + 「本机」节点（F5），备机 = 一台被指定为备机的普通节点。主机「本机」持有的房间与模板
//! 自动镜像给备机；两台对每一场直播交换场次消息，决定「这一场谁投」，保证不出重复稿件、主机离线时不漏投。

pub mod key;
pub mod params;
pub mod store;
pub mod wire;
