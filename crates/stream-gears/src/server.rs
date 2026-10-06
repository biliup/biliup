use biliup_cli::cli::Cli;
use biliup_cli::entry;
use biliup_cli::server::config::Config;
use biliup_cli::server::errors::AppResult;
use pyo3::prelude::PyAnyMethods;
use pyo3::prelude::PyDictMethods;
use pyo3::types::PyDict;
use pyo3::{Bound, PyAny, PyResult, Python};
use pyo3::{pyclass, pyfunction, pymethods};
use pythonize::pythonize;
use std::ops::Deref;
use std::sync::{Arc, LazyLock, RwLock};

#[pyclass]
#[derive(Debug, Clone)]
struct OnceConfig {
    // 用 PyObject 存，方便保持任意 Python 对象
    map: Config,
}

#[pymethods]
impl OnceConfig {
    /// 获取：config.get("k", default=None)
    /// - 若 key 存在，返回保存的对象
    /// - 若不存在，返回 default（默认 None）
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let guard = &self.map;
        // serde_json::to_value(guard.deref())
        if let Some(bound) = pythonize(py, guard)?
            .extract::<Bound<PyDict>>()?
            .get_item(key)?
        {
            if bound.is_none()
                && let Some(d) = default
            {
                return Ok(d);
            }
            // 尝试转换为字典并过滤
            return match bound.cast::<PyDict>() {
                Ok(dict) => {
                    let filtered = PyDict::new(py);
                    dict.iter()
                        .filter(|(_, v)| !v.is_none())
                        .try_for_each(|(k, v)| filtered.set_item(k, v))?;
                    Ok(filtered.into_any())
                }
                Err(_) => Ok(bound), // 不是字典，直接返回
            };
        };
        let Some(default) = default else {
            return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
                "object has no attribute '{key}'"
            )));
        };
        Ok(default)
    }
}

#[pyclass]
pub struct ConfigState {
    // 用 PyObject 存，方便保持任意 Python 对象
    map: Arc<RwLock<Config>>,
}

#[pymethods]
impl ConfigState {
    /// 获取：config.get("k", default=None)
    /// - 若 key 存在，返回保存的对象
    /// - 若不存在，返回 default（默认 None）
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let guard = self.map.read().unwrap();
        // serde_json::to_value(guard.deref())
        if let Some(bound) = pythonize(py, guard.deref())?
            .extract::<Bound<PyDict>>()?
            .get_item(key)?
        {
            if bound.is_none()
                && let Some(d) = default
            {
                return Ok(d);
            }
            // 尝试转换为字典并过滤
            return match bound.cast::<PyDict>() {
                Ok(dict) => {
                    let filtered = PyDict::new(py);
                    dict.iter()
                        .filter(|(_, v)| !v.is_none())
                        .try_for_each(|(k, v)| filtered.set_item(k, v))?;
                    Ok(filtered.into_any())
                }
                Err(_) => Ok(bound), // 不是字典，直接返回
            };
        };
        let Some(default) = default else {
            return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
                "object has no attribute '{key}'"
            )));
        };
        Ok(default)
    }
}

/// Deprecated: always returns the built-in default configuration; the running
/// server never reads or writes it. Kept for compatibility and will be removed.
#[pyfunction]
pub fn config_bindings() -> PyResult<ConfigState> {
    let state = ConfigState {
        map: cfg_arc().clone(),
    };
    // pythonize(py, &config)
    Ok(state)
}

// 进程级全局单例（安全）：OnceLock + Arc + RwLock
pub static CONFIG: LazyLock<Arc<RwLock<Config>>> = LazyLock::new(|| {
    Arc::new(RwLock::new(
        Config::builder().streamers(Default::default()).build(),
    ))
});

fn cfg_arc() -> &'static Arc<RwLock<Config>> {
    &CONFIG
}

#[tokio::main]
pub(crate) async fn run(cli: Cli) -> AppResult<()> {
    entry::run(cli).await
}
