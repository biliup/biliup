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