//! Owned conversion data shared by the XML reader, filters and ASS writer.

use std::io;

pub const EPS: f64 = 1.0e-4;
pub const MAX_TEXT_LENGTH: usize = 1024;
pub const R2L: i16 = 1;
pub const L2R: i16 = 2;
pub const TOP: i16 = 3;
pub const BOTTOM: i16 = 4;
pub const SPECIAL: i16 = 5;
pub const MSG_SUPER_CHAT: i16 = 11;
pub const MSG_GUARD: i16 = 12;
pub const MSG_GIFT: i16 = 13;

/// All options on the project's conversion path, with owned font data.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub resolution_x: i32,
    pub resolution_y: i32,
    pub display_area: f32,
    pub scroll_area: f32,
    pub scroll_time: f32,
    pub fix_time: f32,
    pub density: i32,
    pub line_spacing: i32,
    pub top_margin: i32,
    pub bottom_margin: i32,
    pub font_size: i32,
    pub font_size_strict: bool,
    pub font_size_norm: bool,
    pub font_name: String,
    pub opacity: i32,
    pub outline: f32,
    pub outline_blur: f32,
    pub outline_opacity: i32,
    pub shadow: f32,
    pub bold: bool,
    pub save_blocked: bool,
    pub block_mode: i32,
    pub show_user_names: bool,
    pub show_msg_box: bool,
    pub msg_box_size_x: i32,
    pub msg_box_size_y: i32,
    pub msg_box_pos_x: i32,
    pub msg_box_pos_y: i32,
    pub msgbox_font_size: i32,
    pub msgbox_duration: f32,
    pub gift_min_price: f32,
    pub stat_mode: i32,
    /// Entries retain their original bytes, including UID/name delimiters.
    pub blocklist: Option<Vec<Vec<u8>>>,
    pub blocklist_regex_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resolution_x: 1920,
            resolution_y: 1080,
            display_area: 1.0,
            scroll_area: 1.0,
            scroll_time: 12.0,
            fix_time: 5.0,
            density: 0,
            line_spacing: 0,
            top_margin: 0,
            bottom_margin: 0,
            font_size: 38,
            font_size_strict: false,
            font_size_norm: false,
            font_name: "Microsoft YaHei".into(),
            opacity: 180,
            outline: 0.0,
            outline_blur: 0.0,
            outline_opacity: 255,
            shadow: 1.0,
            bold: false,
            save_blocked: true,
            block_mode: 0,
            show_user_names: false,
            show_msg_box: false,
            msg_box_size_x: 500,
            msg_box_size_y: 1080,
            msg_box_pos_x: 20,
            msg_box_pos_y: 0,
            msgbox_font_size: 38,
            msgbox_duration: 0.0,
            gift_min_price: 0.0,
            stat_mode: 0,
            blocklist: None,
            blocklist_regex_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserPart {
    pub uid: u64,
    pub level: i32,
    pub name: Vec<u8>,
    pub medal_name: Vec<u8>,
    pub medal_level: i32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GiftPart {
    pub name: Vec<u8>,
    pub price: i32,
    pub count: i32,
    pub duration: i32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecialPart {
    pub exist_time: i32,
    pub move_time: i32,
    pub pause_time: i32,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub fade_start: i32,
    pub fade_end: i32,
    pub fr_y: i32,
    pub fr_z: i32,
    pub font_name: Vec<u8>,
}

/// Text remains bytes: the legacy format truncates at byte boundaries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Danmaku {
    pub time: i32,
    pub type_: i16,
    pub font_size: i16,
    pub color: i32,
    pub text: Vec<u8>,
    /// Gift and guard tags have no text allocation in the legacy reader.
    pub text_is_null: bool,
    pub user: Option<Box<UserPart>>,
    pub gift: Option<Box<GiftPart>>,
    pub special: Option<Box<SpecialPart>>,
}

impl Danmaku {
    pub fn is_type(&self, kind: i16) -> bool {
        self.type_ == kind || self.type_ == -kind
    }

    pub fn is_normal(&self) -> bool {
        [R2L, L2R, TOP, BOTTOM]
            .into_iter()
            .any(|kind| self.is_type(kind))
    }

    pub fn is_special(&self) -> bool {
        self.is_type(SPECIAL)
    }

    pub fn is_message(&self) -> bool {
        [MSG_SUPER_CHAT, MSG_GUARD, MSG_GIFT]
            .into_iter()
            .any(|kind| self.is_type(kind))
    }

    pub fn block(&mut self) {
        if self.type_ > 0 {
            self.type_ = -self.type_;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    pub status: i32,
    pub upstream_code: i32,
    pub os_error: i32,
    pub detail: String,
    pub danmaku_count: i32,
    pub blocked_count: i32,
}

impl CoreError {
    pub fn new(status: i32, upstream_code: i32, os_error: i32, detail: impl Into<String>) -> Self {
        Self {
            status,
            upstream_code,
            os_error,
            detail: detail.into(),
            danmaku_count: 0,
            blocked_count: 0,
        }
    }

    pub fn io(
        status: i32,
        upstream_code: i32,
        error: &io::Error,
        detail: impl Into<String>,
    ) -> Self {
        Self::new(
            status,
            upstream_code,
            error.raw_os_error().unwrap_or(0),
            detail,
        )
    }

    pub fn with_counts(mut self, danmaku_count: i32, blocked_count: i32) -> Self {
        self.danmaku_count = danmaku_count;
        self.blocked_count = blocked_count;
        self
    }
}

/// C's time macros deliberately round at different stages.
pub fn milliseconds(seconds: f64) -> i32 {
    (seconds * 1000.0 + 0.5) as i32
}

pub fn ass_milliseconds(seconds: f32) -> i32 {
    ((seconds * 100.0 + 0.55) as i32).saturating_mul(10)
}

pub fn rounded_milliseconds(milliseconds: i32) -> i32 {
    ((milliseconds as i64 + 5) / 10 * 10) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_nodes_do_not_inline_unused_message_or_special_payloads() {
        // Large recordings contain hundreds of thousands of ordinary nodes.
        // Keeping optional payloads inline previously quadrupled peak RSS.
        assert!(std::mem::size_of::<Danmaku>() <= 64);
    }
}
