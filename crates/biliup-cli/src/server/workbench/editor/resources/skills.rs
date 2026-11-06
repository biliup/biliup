//! Editing recipes bundled with the application, independent of plugin installation.
use serde_json::Value;
use std::sync::LazyLock;

static BUILTIN: LazyLock<Vec<Value>> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/skills.json"
    )))
    .expect("bundled editing skills")
});

pub fn all() -> &'static [Value] {
    &BUILTIN
}

pub fn contains(id: &str) -> bool {
    all().iter().any(|skill| skill["id"] == id)
}
