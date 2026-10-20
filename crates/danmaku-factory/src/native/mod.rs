//! Safe native conversion pipeline. The frozen C oracle is external to it.
#![forbid(unsafe_code)]

pub(crate) mod escape;
pub(crate) mod layout;
pub(crate) mod model;
pub(crate) mod number;
pub(crate) mod parse;
pub(crate) mod special;
pub(crate) mod transaction;
pub(crate) mod writer;

#[cfg(test)]
mod differential;

use crate::{ConvertError, ConvertOptions, ErrorKind};

pub(crate) fn validate(options: &ConvertOptions) -> Result<(), ConvertError> {
    let between = |v: f32, low, high| v.is_finite() && v >= low && v <= high;
    let invalid = if !(1..=16384).contains(&options.width) || !(1..=16384).contains(&options.height)
    {
        Some("resolution")
    } else if !between(options.display_area, 0.0001, 1.0001) {
        Some("display_area")
    } else if !between(options.scroll_area, 0.0001, 1.0001) {
        Some("scroll_area")
    } else if !between(options.scroll_time, 0.0001, 3600.) {
        Some("scroll_time")
    } else if !between(options.fix_time, 0.0001, 3600.) {
        Some("fix_time")
    } else if options.density < -1 {
        Some("density")
    } else if !(-16384..=16384).contains(&options.line_spacing) {
        Some("line_spacing")
    } else if !(-16384..=16384).contains(&options.top_margin)
        || !(-16384..=16384).contains(&options.bottom_margin)
    {
        Some("margin")
    } else if !(1..=4096).contains(&options.font_size) {
        Some("font_size")
    } else if options.font_name.is_empty()
        || options.font_name.len() >= 128
        || options
            .font_name
            .bytes()
            .any(|b| b < 0x20 || b == 0x7f || b == b',')
    {
        Some("font_name")
    } else if options.opacity == 0 {
        Some("opacity")
    } else if !between(options.outline, 0., 4.) {
        Some("outline")
    } else if !between(options.outline_blur, 0., 100.) {
        Some("outline_blur")
    } else if !between(options.shadow, 0., 4.) {
        Some("shadow")
    } else if options.block_mode & !127 != 0 {
        Some("block_mode")
    } else {
        None
    };
    match invalid {
        Some(field) => Err(ConvertError::new(
            ErrorKind::InvalidArgument,
            format!("invalid {field}"),
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_validation_preserves_the_c_limits_including_float_epsilon() {
        let base = ConvertOptions::default();
        assert!(validate(&base).is_ok());
        for options in [
            ConvertOptions {
                width: 0,
                ..base.clone()
            },
            ConvertOptions {
                height: 16385,
                ..base.clone()
            },
            ConvertOptions {
                display_area: f32::NAN,
                ..base.clone()
            },
            ConvertOptions {
                scroll_area: f32::INFINITY,
                ..base.clone()
            },
            ConvertOptions {
                scroll_time: 3601.,
                ..base.clone()
            },
            ConvertOptions {
                fix_time: 0.,
                ..base.clone()
            },
            ConvertOptions {
                density: -2,
                ..base.clone()
            },
            ConvertOptions {
                line_spacing: 16385,
                ..base.clone()
            },
            ConvertOptions {
                bottom_margin: -16385,
                ..base.clone()
            },
            ConvertOptions {
                font_size: 4097,
                ..base.clone()
            },
            ConvertOptions {
                font_name: "字".repeat(43),
                ..base.clone()
            },
            ConvertOptions {
                font_name: "A,B".into(),
                ..base.clone()
            },
            ConvertOptions {
                font_name: "A\0B".into(),
                ..base.clone()
            },
            ConvertOptions {
                opacity: 0,
                ..base.clone()
            },
            ConvertOptions {
                outline: 4.1,
                ..base.clone()
            },
            ConvertOptions {
                outline_blur: 100.1,
                ..base.clone()
            },
            ConvertOptions {
                shadow: -0.1,
                ..base.clone()
            },
            ConvertOptions {
                block_mode: 128,
                ..base.clone()
            },
        ] {
            assert_eq!(
                validate(&options).unwrap_err().kind,
                ErrorKind::InvalidArgument,
                "{options:?}"
            );
        }
        assert!(
            validate(&ConvertOptions {
                display_area: 1.0001,
                scroll_area: 0.0001,
                scroll_time: 3600.,
                fix_time: 0.0001,
                line_spacing: -16384,
                top_margin: 16384,
                font_size: 4096,
                ..base
            })
            .is_ok()
        );
    }
}
