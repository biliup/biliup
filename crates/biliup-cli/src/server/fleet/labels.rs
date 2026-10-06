//! 节点标签与房间要求的标签（F4）。
//!
//! 标签是自由文本（例如「海外」「国内」「家庭宽带」），库里存成 JSON 字符串数组。
//! 约束只有一条：节点的标签必须包含房间要求的全部标签，区分大小写、按全文匹配。

/// 单个标签最多多少个字符
pub const MAX_LABEL_CHARS: usize = 32;
/// 一个节点 / 一个房间最多多少个标签
pub const MAX_LABELS: usize = 20;

/// 去掉首尾空白、丢掉空标签、按首次出现去重；超长或太多时报错，错误信息直接给界面看。
pub fn normalize(labels: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for label in labels {
        let label = label.trim();
        if label.is_empty() || out.iter().any(|seen| seen == label) {
            continue;
        }
        if label.chars().count() > MAX_LABEL_CHARS {
            return Err(format!("标签「{label}」太长，最多 {MAX_LABEL_CHARS} 个字"));
        }
        if label.chars().any(char::is_control) {
            return Err("标签里不能有换行等控制字符".into());
        }
        out.push(label.to_string());
    }
    if out.len() > MAX_LABELS {
        return Err(format!("标签最多 {MAX_LABELS} 个"));
    }
    Ok(out)
}

/// 读库里的 JSON 数组；读不懂（手改过库）按没有标签处理
pub fn parse(text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(text).unwrap_or_default()
}

pub fn to_json(labels: &[String]) -> String {
    serde_json::to_string(labels).unwrap_or_else(|_| "[]".into())
}

/// `required` 里 `have` 没有的标签，保持 `required` 的顺序
pub fn missing(have: &[String], required: &[String]) -> Vec<String> {
    required
        .iter()
        .filter(|label| !have.contains(label))
        .cloned()
        .collect()
}

/// 拒绝原因里用的「海外」「国内」写法
pub fn quoted(labels: &[String]) -> String {
    labels
        .iter()
        .map(|label| format!("「{label}」"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normalize_trims_dedups_and_drops_empty() {
        let labels = normalize(&strings(&[
            " 海外 ", "", "国内", "海外", "  ", "GPU", "gpu",
        ]))
        .unwrap();
        assert_eq!(labels, strings(&["海外", "国内", "GPU", "gpu"]));
        assert!(normalize(&[]).unwrap().is_empty());
    }

    #[test]
    fn normalize_rejects_long_many_or_control_labels() {
        let long = "长".repeat(MAX_LABEL_CHARS + 1);
        let error = normalize(&[long]).unwrap_err();
        assert!(error.contains("太长"), "{error}");
        assert!(normalize(&["长".repeat(MAX_LABEL_CHARS)]).is_ok());

        let many: Vec<String> = (0..=MAX_LABELS).map(|i| format!("l{i}")).collect();
        assert!(normalize(&many).unwrap_err().contains("最多"));
        assert!(normalize(&many[..MAX_LABELS]).is_ok());

        assert!(normalize(&strings(&["a\nb"])).is_err());
    }

    #[test]
    fn parse_tolerates_garbage() {
        assert_eq!(parse("[\"海外\"]"), strings(&["海外"]));
        assert!(parse("").is_empty());
        assert!(parse("{\"a\":1}").is_empty());
        assert_eq!(parse(&to_json(&strings(&["a", "b"]))), strings(&["a", "b"]));
    }

    #[test]
    fn missing_keeps_required_order() {
        let have = strings(&["国内", "家庭宽带"]);
        assert!(missing(&have, &[]).is_empty());
        assert!(missing(&have, &strings(&["家庭宽带"])).is_empty());
        assert_eq!(
            missing(&have, &strings(&["海外", "家庭宽带", "GPU"])),
            strings(&["海外", "GPU"])
        );
        assert_eq!(quoted(&strings(&["海外", "GPU"])), "「海外」「GPU」");
    }
}
