#[cfg(feature = "cli")]
use clap::ValueEnum;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SubmitOption {
    // Client,
    App,
    // Web,
    BCutAndroid,
}

impl SubmitOption {
    /// Parse a string into SubmitOption, compatible with both clap and manual parsing
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "app" => Ok(SubmitOption::App),
            "bcutandroid" | "b-cut-android" | "bcut_android" => Ok(SubmitOption::BCutAndroid),
            _ => Err(format!("Unknown submit option: {}", s))
        }
    }
}