use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

use std::time::Duration;
use tracing::{error, info};

use super::extractor::CallbackFn;

#[derive(Debug)]
pub enum Segment {
    Time(Duration, Duration),
    Size(u64, u64),
    Never,
}
#[derive(Debug)]
pub struct Segmentable {
    time: Time,
    size: Size,
}
#[derive(Debug)]
struct Time {
    expected: Option<Duration>,
    start: Duration,
    current: Duration,
}
#[derive(Debug)]
struct Size {
    expected: Option<u64>,
    current: u64,
}

impl Segmentable {
    pub fn new(expected_time: Option<Duration>, expected_size: Option<u64>) -> Self {
        Self {
            time: Time {
                expected: expected_time,
                start: Duration::ZERO,
                current: Duration::ZERO,
            },
            size: Size {
                expected: expected_size,
                current: 0,
            },
        }
    }

    pub fn needed(&self) -> bool {
        if let Some(expected_time) = self.time.expected {
            return (self.time.current - self.time.start) >= expected_time;
        }
        if let Some(expected_size) = self.size.expected {
            return self.size.current > expected_size;
        }
        false
    }

    pub fn increase_time(&mut self, number: Duration) {
        self.time.current += number
    }

    pub fn set_time_position(&mut self, number: Duration) {
        self.time.current = number
    }

    pub fn set_start_time(&mut self, number: Duration) {
        self.time.start = number
    }

    pub fn increase_size(&mut self, number: u64) {
        self.size.current += number
    }

    pub fn set_size_position(&mut self, number: u64) {
        self.size.current = number
    }

    pub fn reset(&mut self) {
        self.size.current = 0;
        self.time.current = Duration::ZERO;
    }
}

impl Default for Segmentable {
    fn default() -> Self {
        Segmentable {
            time: Time {
                expected: None,
                start: Duration::ZERO,
                current: Duration::ZERO,
            },
            size: Size {
                expected: None,
                current: 0,
            },
        }
    }
}

pub struct LifecycleFile {
    pub fmt_file_name: String,
    pub file_name: String,
    pub path: PathBuf,
    pub hook: CallbackFn,
    pub extension: &'static str,
}

impl LifecycleFile {
    pub fn new(fmt_file_name: &str, extension: &'static str, hook: Option<CallbackFn>) -> Self {
        let hook: Box<dyn Fn(&str) + Send> = match hook {
            Some(hook) => hook,
            _ => Box::new(|_| {}),
        };
        Self {
            fmt_file_name: fmt_file_name.to_string(),
            file_name: "".to_string(),
            path: Default::default(),
            hook,
            extension,
        }
    }

    pub fn create(&mut self) -> Result<&Path, std::io::Error> {
        self.file_name = format!(
            "{}.{}",
            format_filename(&self.fmt_file_name),
            self.extension
        );
        self.path = PathBuf::from(&self.file_name);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?
        }
        // path.set_extension(&self.extension);
        self.path.set_extension(format!("{}.part", self.extension));
        info!("Save to {}", self.path.display());
        Ok(self.path.as_path())
    }

    pub fn rename(&self) {
        match fs::rename(&self.path, &self.file_name) {
            Ok(_) => (self.hook)(&self.file_name),
            Err(e) => {
                error!("drop {} {e}", self.path.display())
            }
        }
    }
}

pub fn format_filename(file_name: &str) -> String {
    let local: DateTime<Local> = Local::now();
    // let time_str = local.format("%Y-%m-%dT%H_%M_%S");
    let time_str = local.format(file_name);
    // format!("{file_name}{time_str}")
    time_str.to_string()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::{Path, PathBuf};

    #[test]
    fn it_works() -> Result<()> {
        let mut p = PathBuf::from("/feel/the");

        p.set_extension("force");
        assert_eq!(Path::new("/feel/the.force"), p.as_path());

        p.set_extension("");
        assert_eq!(Path::new("/feel/the"), p.as_path());

        Ok(())
    }
}
