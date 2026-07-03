#[cfg(feature = "cli")]
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SubmitOption {
    // Client,
    App,
    Web,
    BCutAndroid,
}

impl FromStr for SubmitOption {
    type Err = String;

    /// Parse a string into SubmitOption, compatible with both clap and manual parsing
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "app" => Ok(SubmitOption::App),
            "web" => Ok(SubmitOption::Web),
            "bcutandroid" | "b-cut-android" | "bcut_android" => Ok(SubmitOption::BCutAndroid),
            _ => Err(format!("Unknown submit option: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SubmitOption;
    use std::str::FromStr;

    #[test]
    fn parse_submit_options() {
        assert!(matches!(
            SubmitOption::from_str("app").unwrap(),
            SubmitOption::App
        ));
        assert!(matches!(
            SubmitOption::from_str("web").unwrap(),
            SubmitOption::Web
        ));
        assert!(matches!(
            SubmitOption::from_str("bcut_android").unwrap(),
            SubmitOption::BCutAndroid
        ));
        assert!(SubmitOption::from_str("unknown").is_err());
    }
}
