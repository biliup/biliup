//! `biliup node join <票据> / leave / status`：在 biliup 服务的工作目录下执行，读写 `data/node.json`。

use crate::cli::NodeAction;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::{FLEET_DB, NODE_FILE, node};
use error_stack::bail;
use std::path::Path;

pub async fn run(action: NodeAction) -> AppResult<()> {
    let node_file = Path::new(NODE_FILE);
    match action {
        NodeAction::Join {
            ticket,
            allow_hooks,
        } => {
            if Path::new(FLEET_DB).exists() {
                bail!(AppError::Custom(format!(
                    "当前目录下有 {FLEET_DB}，这台机器是控制面，不能再作为节点加入"
                )));
            }
            let file = node::join(&ticket, allow_hooks, node_file).await?;
            println!(
                "已加入控制面，节点 id {}，凭据写在 {NODE_FILE}（请妥善保管，不要外传）。",
                file.node_id
            );
            println!(
                "启动（或重启）biliup server 后开始向控制面上报状态；已经在运行的实例需要重启一次。"
            );
            if allow_hooks {
                println!("已允许控制面下发的房间使用钩子。");
            }
            Ok(())
        }
        NodeAction::Leave => {
            if !node_file.exists() {
                println!("这台机器没有加入任何控制面（{NODE_FILE} 不存在）。");
                return Ok(());
            }
            if node::leave(node_file).await? {
                println!("已通知控制面移除本节点，并删除了 {NODE_FILE}。");
            } else {
                println!(
                    "控制面不可达，只删除了本地凭据 {NODE_FILE}；请在控制面「节点」页手动移除这台节点。"
                );
            }
            println!("正在运行的 biliup server 会在断线后停止上报，重启后即为单机模式。");
            Ok(())
        }
        NodeAction::Status => {
            match node::status(node_file)? {
                None => println!("这台机器没有加入任何控制面（{NODE_FILE} 不存在）。"),
                Some(status) => println!(
                    "{}",
                    serde_json::to_string_pretty(&status).unwrap_or_default()
                ),
            }
            Ok(())
        }
    }
}
