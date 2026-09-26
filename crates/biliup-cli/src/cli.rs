use biliup::uploader::bilibili::{Studio, Vid};
use biliup::uploader::util::SubmitOption;
use clap::{Parser, Subcommand};

use crate::UploadLine;
use crate::season_cli::SeasonArgs;
use crate::server::fleet::RelayListen;
use std::path::PathBuf;
use url::Url;

/// 扩展路径中的 ~ 为用户主目录
pub fn expand_path(path: PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        let expanded = shellexpand::tilde(path_str);
        return PathBuf::from(expanded.as_ref());
    }
    path
}

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    // /// Turn debugging information on
    // #[clap(short, long, parse(from_occurrences))]
    // debug: usize,
    #[clap(subcommand)]
    pub command: Commands,

    /// 配置代理
    #[arg(short, long, default_value = None)]
    pub proxy: Option<String>,

    /// 登录信息文件
    #[arg(short, long, default_value = "cookies.json")]
    pub user_cookie: PathBuf,

    /// 日志过滤规则，如 debug；不指定时读取环境变量 RUST_LOG，都没有则为 tower_http=debug,info
    #[arg(long)]
    pub rust_log: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 登录B站并保存登录信息
    Login,
    /// 手动验证并刷新登录信息
    Renew,
    /// 上传视频
    Upload {
        /// 提交接口
        #[arg(long)]
        submit: Option<SubmitOption>,

        // Optional name to operate on
        // name: Option<String>,
        /// 需要上传的视频路径,若指定配置文件投稿不需要此参数
        #[arg()]
        video_path: Vec<PathBuf>,

        /// Sets a custom config file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// 选择上传线路
        #[arg(short, long, value_enum)]
        line: Option<UploadLine>,

        /// 单视频文件最大并发数
        #[arg(long, default_value = "3")]
        limit: usize,

        #[command(flatten)]
        studio: Studio,
        // #[arg(required = false, last = true, default_value = "client")]
        // submit: Option<String>,
    },
    /// 是否要对某稿件追加视频
    Append {
        /// 提交接口
        #[arg(long)]
        submit: Option<SubmitOption>,

        // Optional name to operate on
        // name: Option<String>,
        /// vid为稿件 av 或 bv 号
        #[arg(short, long)]
        vid: Vid,
        /// 需要上传的视频路径,若指定配置文件投稿不需要此参数
        #[arg()]
        video_path: Vec<PathBuf>,

        /// 选择上传线路
        #[arg(short, long, value_enum)]
        line: Option<UploadLine>,

        /// 单视频文件最大并发数
        #[arg(long, default_value = "3")]
        limit: usize,

        #[command(flatten)]
        studio: Studio,
    },
    /// 打印视频详情
    Show {
        /// vid为稿件 av 或 bv 号
        // #[clap()]
        vid: Vid,
    },
    /// 查看视频评论
    Comments {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 排序方式，0为按时间，2为按热度
        #[arg(long, default_value = "0")]
        sort: u8,

        /// 页码
        #[arg(long, default_value = "1")]
        pn: u32,

        /// 每页条数
        #[arg(long, default_value = "20")]
        ps: u32,
    },
    /// 回复视频评论，默认只打印将要回复的内容
    Reply {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 评论 rpid
        rpid: u64,

        /// 回复内容
        message: String,

        /// 实际发送回复
        #[arg(long)]
        execute: bool,
    },
    /// 输出flv元数据
    DumpFlv {
        #[arg()]
        file_name: PathBuf,
    },
    /// 下载视频
    Download {
        url: String,

        /// Output filename template. e.p. "./video/%Y-%m-%dT%H_%M_%S{title}"
        #[arg(short, long, default_value = "{title}")]
        output: String,

        /// 按照大小分割视频
        #[arg(long, value_parser = human_size)]
        split_size: Option<u64>,

        /// 按照时间分割视频
        #[arg(long)]
        split_time: Option<humantime::Duration>,
    },
    /// 启动web服务，默认端口19159
    Server {
        /// Specify bind address
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,

        /// Port to use
        #[arg(short, long, default_value = "19159")]
        port: u16,

        /// 开启登录密码认证
        #[arg(long, default_value = "false")]
        auth: bool,

        /// 为会话 Cookie 附加 Secure 属性。仅当通过 HTTPS 反向代理访问 Web UI 时开启；
        /// 直接通过 HTTP 远程访问时开启会导致浏览器丢弃登录态
        #[arg(long, default_value = "false")]
        secure_session_cookie: bool,

        /// 使用 biliup 1.0.7 风格配置文件启动录制
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// 以 Fleet 控制面运行：接受其他 biliup 节点加入并显示它们的状态（数据在 data/fleet.sqlite3）
        #[arg(long)]
        controller: bool,

        /// 控制面内嵌 relay 的 TCP 监听地址（默认 0.0.0.0:19160），节点必须能连到它；
        /// off 表示不起内嵌 relay，此时必须用 --relay-url 指定外部 relay。只在 --controller 时生效
        #[arg(long, value_name = "ADDR|off")]
        relay_listen: Option<RelayListen>,

        /// 写进加入票据的 relay 地址，可重复；不给时自动列出本机网卡地址。
        /// 用于域名、端口转发、反向代理或外部 relay。只在 --controller 时生效
        #[arg(long, value_name = "URL")]
        relay_url: Vec<Url>,
    },
    /// 管理自己的合集：列合集、查小节、加入 / 移出稿件、排序
    Season(SeasonArgs),
    /// 管理 Web 界面的登录用户（在 biliup 服务的工作目录下执行，直接读写 data/data.sqlite3）
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// 把这台机器加入 Fleet 控制面或退出（在 biliup 服务的工作目录下执行，读写 data/node.json）
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
    /// 列出所有已上传的视频
    List {
        /// 只包含进行中的视频
        #[arg(long)]
        is_pubing: bool,

        /// 只包含已通过的视频
        #[arg(long)]
        pubed: bool,

        /// 只包含未通过的视频
        #[arg(long)]
        not_pubed: bool,

        /// 从第几页开始获取
        #[arg(short, long, default_value = "1")]
        from_page: u32,

        /// 最大获取页数
        #[arg(short, long)]
        max_pages: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum UserAction {
    /// 列出所有 Web 用户
    List,
    /// 重置某个 Web 用户的密码并让其所有会话失效；新密码从终端提示输入，或从标准输入读一行
    ResetPassword {
        /// 用户名（大小写不敏感）
        username: String,
    },
}

#[derive(Subcommand)]
pub enum NodeAction {
    /// 用控制面「添加节点」给出的票据加入；成功后重启 biliup server 生效
    Join {
        /// 以 bfleet 开头的加入票据
        ticket: String,
        /// 允许控制面下发的房间使用钩子（各 *processor，等于允许在本机执行命令）
        #[arg(long)]
        allow_hooks: bool,
    },
    /// 通知控制面移除本节点，并删除本地凭据 data/node.json
    Leave,
    /// 查看本机的节点凭据（不连控制面）
    Status,
}

fn human_size(s: &str) -> Result<u64, String> {
    let ret = match s.as_bytes() {
        [init @ .., b'K'] => parse_u8(init)? * 1000.0,
        [init @ .., b'M'] => parse_u8(init)? * 1000.0 * 1000.0,
        [init @ .., b'G'] => parse_u8(init)? * 1000.0 * 1000.0 * 1000.0,
        init => parse_u8(init)?,
    };
    Ok(ret as u64)
}

fn parse_u8(string: &[u8]) -> Result<f64, String> {
    let string = String::from_utf8_lossy(string);
    string
        .parse()
        .map_err(|e| format!("{string} is not ascii digit. {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::path::Path;

    #[test]
    fn server_defaults_to_loopback_and_default_cookie_file() {
        let cli = Cli::try_parse_from(["biliup", "server"]).unwrap();

        assert_eq!(cli.user_cookie, Path::new("cookies.json"));
        assert!(matches!(
            cli.command,
            Commands::Server {
                ref bind,
                auth: false,
                secure_session_cookie: false,
                ..
            } if bind == "127.0.0.1"
        ));
    }

    #[test]
    fn fleet_server_flags_parse() {
        use crate::server::fleet::RelayListen;
        let cli = Cli::try_parse_from(["biliup", "server"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Server {
                controller: false,
                relay_listen: None,
                ref relay_url,
                ..
            } if relay_url.is_empty()
        ));
        let cli = Cli::try_parse_from([
            "biliup",
            "server",
            "--controller",
            "--relay-listen",
            "off",
            "--relay-url",
            "http://relay.example.com:19160",
            "--relay-url",
            "http://10.0.0.2:19160",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Server {
                controller: true,
                relay_listen: Some(RelayListen::Off),
                ref relay_url,
                ..
            } if relay_url.len() == 2
        ));
        assert!(Cli::try_parse_from(["biliup", "server", "--relay-listen", "nope"]).is_err());
    }

    #[test]
    fn node_subcommands_parse() {
        let cli =
            Cli::try_parse_from(["biliup", "node", "join", "bfleetabc", "--allow-hooks"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Node {
                action: super::NodeAction::Join { ref ticket, allow_hooks: true }
            } if ticket == "bfleetabc"
        ));
        for (name, leave) in [("leave", true), ("status", false)] {
            let cli = Cli::try_parse_from(["biliup", "node", name]).unwrap();
            let Commands::Node { action } = cli.command else {
                panic!("not a node command")
            };
            assert_eq!(matches!(action, super::NodeAction::Leave), leave);
        }
    }

    #[test]
    fn user_subcommands_parse() {
        let cli = Cli::try_parse_from(["biliup", "user", "reset-password", "biliup"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::User {
                action: super::UserAction::ResetPassword { ref username }
            } if username == "biliup"
        ));
        let cli = Cli::try_parse_from(["biliup", "user", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::User {
                action: super::UserAction::List
            }
        ));
    }

    #[test]
    fn server_preserves_an_explicit_cookie_file() {
        let cli = Cli::try_parse_from([
            "biliup",
            "--user-cookie",
            "/tmp/private-account.json",
            "server",
        ])
        .unwrap();

        assert_eq!(cli.user_cookie, Path::new("/tmp/private-account.json"));
    }
}
