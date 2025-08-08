use std::str::FromStr;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SubmitOption {
    // Client,
    App,
    // Web,
    BCutAndroid,
}

impl FromStr for SubmitOption {
    type Err = String;

    /// Parse a string into SubmitOption, compatible with both clap and manual parsing
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "app" => Ok(SubmitOption::App),
            "bcutandroid" | "b-cut-android" | "bcut_android" => Ok(SubmitOption::BCutAndroid),
            _ => Err(format!("Unknown submit option: {}", s))
        }
    }
}