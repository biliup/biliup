//! 迁移文件的原始字节就是 `_sqlx_migrations` 校验和的来源：同一份 SQL 换一种换行，
//! 编出来的二进制就内嵌另一套摘要，老库碰上另一种换行的构建便以
//! `migration N was previously applied but has been modified` 起不来。仓库根目录的
//! `.gitattributes` 把 `*.sql` 固定成 LF，这里在编译期兜底：带 CR 的迁移进不了二进制。
//!
//! 主库的 `migrations/` 与控制面独立库 `data/fleet.sqlite3` 的 `fleet_migrations/` 同样对待。

use std::fs;
use std::path::Path;

const MIGRATION_DIRS: &[&str] = &["migrations", "fleet_migrations"];

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for name in MIGRATION_DIRS {
        // `sqlx::migrate!()` 只追踪已有的迁移文件，新增迁移时靠这一行触发重新编译（sqlx 官方做法）。
        println!("cargo:rerun-if-changed={name}");

        let dir = Path::new(&manifest_dir).join(name);
        for entry in fs::read_dir(&dir).unwrap_or_else(|err| panic!("无法读取 {name} 目录: {err}"))
        {
            let path = entry
                .unwrap_or_else(|err| panic!("无法读取 {name} 目录: {err}"))
                .path();
            if path.extension().is_none_or(|ext| ext != "sql") {
                continue;
            }
            let sql =
                fs::read(&path).unwrap_or_else(|err| panic!("无法读取 {}: {err}", path.display()));
            assert!(
                !sql.contains(&b'\r'),
                "{} 含 CR 换行。迁移文件必须是 LF（见仓库根目录 .gitattributes），否则不同平台编出来的\
                 二进制会记下不同的校验和。.gitattributes 生效前检出的文件不会自动转换：没有本地改动的话，\
                 删掉它后在仓库根目录执行 `git checkout -- crates/biliup-cli/{name}` 重新检出即可。",
                path.display()
            );
        }
    }
}
