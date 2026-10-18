//! Pixel occupancy and motion decisions from the frozen DanmakuFactory core.
#![forbid(unsafe_code)]

use super::model::{BlockReason, Danmaku, DanmakuKind, NormalizedDanmaku};
use super::number::{ass_duration_ms, ass_start_ms, checked_i32, checked_i64};
use crate::{ConvertError, ConvertOptions, ErrorKind, block};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, hash_map::RandomState};
use std::hash::BuildHasher;
use std::ops::{Deref, Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Emission {
    Dialogue,
    Comment,
    Omit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    RightToLeft,
    LeftToRight,
    Top,
    Bottom,
    Special,
    Code,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoveTiming {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    None,
    Position {
        x: i64,
        y: i32,
    },
    Move {
        from_x: i64,
        from_y: i32,
        to_x: i64,
        to_y: i32,
        timing: Option<MoveTiming>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialFade {
    Opaque,
    Transition {
        start: i32,
        end: i32,
        transition_start_ms: i64,
        transition_end_ms: i64,
    },
}

pub(crate) type LayoutedDanmaku = Danmaku;

/// One allocation survives parsing, stable sorting and layout. The source key
/// and sequence are never overwritten; only the typed phase payload changes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutedPool {
    records: Vec<Danmaku>,
    pub(crate) blur: f32,
}
impl Deref for LayoutedPool {
    type Target = [Danmaku];
    fn deref(&self) -> &Self::Target {
        &self.records
    }
}
impl LayoutedPool {
    #[cfg(test)]
    pub(crate) fn into_records(self) -> Vec<Danmaku> {
        self.records
    }
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.records.capacity()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedSpecial {
    pub(crate) rotation_y: i32,
    pub(crate) rotation_z: i32,
    pub(crate) font_name: Box<str>,
    pub(crate) fade: SpecialFade,
    start_ms: i32,
    end_ms: i32,
    raw_lifetime_ms: i32,
    color: i32,
    font: FontOverride,
    placement: Placement,
    blocked: Option<BlockReason>,
}

/// A stack-only expansion of already resolved fields. No collision, rounding,
/// filtering or style choices occur when a writer requests this view.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedView<'a> {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
    pub(crate) emission: Emission,
    pub(crate) layer: u8,
    pub(crate) style: Style,
    pub(crate) color: i32,
    pub(crate) placement: Placement,
    pub(crate) pixel_rows: Option<Range<usize>>,
    pub(crate) font_size_override: Option<i32>,
    pub(crate) blur: bool,
    pub(crate) special: Option<&'a ResolvedSpecial>,
}

/// i32::MIN is outside the mapped font height [-32792,36838]. This sealed
/// representation saves a discriminant without restricting any source font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FontOverride(i32);
impl FontOverride {
    fn new(value: Option<i32>) -> Self {
        Self(value.unwrap_or(i32::MIN))
    }
    fn get(self) -> Option<i32> {
        (self.0 != i32::MIN).then_some(self.0)
    }
}

// A native enum records both the decided style and output disposition. Its
// unused discriminants also let Rust overlay smaller non-scroll phases within
// a 40-byte payload; no byte casts, packed structs or unsafe storage are used.
macro_rules! normal_dispositions {
    ($($kind:ident => ($visible:ident,$input:ident,$typ:ident,$color:ident,$density:ident,$repeat:ident,$position:ident);)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum NormalDisposition { $($visible,$input,$typ,$color,$density,$repeat,$position,)+ }
        impl NormalDisposition {
            fn new(kind: DanmakuKind, blocked: Option<BlockReason>) -> Self {
                match (kind,blocked) { $(
                    (DanmakuKind::$kind,None)=>Self::$visible,
                    (DanmakuKind::$kind,Some(BlockReason::Input))=>Self::$input,
                    (DanmakuKind::$kind,Some(BlockReason::Type))=>Self::$typ,
                    (DanmakuKind::$kind,Some(BlockReason::Color))=>Self::$color,
                    (DanmakuKind::$kind,Some(BlockReason::Density))=>Self::$density,
                    (DanmakuKind::$kind,Some(BlockReason::Repeat))=>Self::$repeat,
                    (DanmakuKind::$kind,Some(BlockReason::NoPosition))=>Self::$position,
                )+ _=>unreachable!("normal disposition requires a normal kind") }
            }
            fn kind(self) -> DanmakuKind {
                match self { $(Self::$visible|Self::$input|Self::$typ|Self::$color|Self::$density|Self::$repeat|Self::$position=>DanmakuKind::$kind,)+ }
            }
            fn blocked(self) -> Option<BlockReason> {
                match self { $(
                    Self::$visible=>None,
                    Self::$input=>Some(BlockReason::Input),
                    Self::$typ=>Some(BlockReason::Type),
                    Self::$color=>Some(BlockReason::Color),
                    Self::$density=>Some(BlockReason::Density),
                    Self::$repeat=>Some(BlockReason::Repeat),
                    Self::$position=>Some(BlockReason::NoPosition),
                )+ }
            }
        }
    }
}
normal_dispositions! {
    RightToLeft => (RightDialogue,RightInput,RightType,RightColor,RightDensity,RightRepeat,RightNoPosition);
    LeftToRight => (LeftDialogue,LeftInput,LeftType,LeftColor,LeftDensity,LeftRepeat,LeftNoPosition);
    Top => (TopDialogue,TopInput,TopType,TopColor,TopDensity,TopRepeat,TopNoPosition);
    Bottom => (BottomDialogue,BottomInput,BottomType,BottomColor,BottomDensity,BottomRepeat,BottomNoPosition);
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scroll {
    half_width: i64,
    start_ms: i32,
    end_ms: i32,
    color: i32,
    font: i16,
    screen_width: u16,
    y: i16,
    rows: Range<u16>,
    disposition: NormalDisposition,
    effects: ScrollEffects,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollEffects {
    None,
    Font,
    Blur,
    FontBlur,
}
impl ScrollEffects {
    fn new(font: bool, blur: bool) -> Self {
        match (font, blur) {
            (false, false) => Self::None,
            (true, false) => Self::Font,
            (false, true) => Self::Blur,
            (true, true) => Self::FontBlur,
        }
    }
    fn font(self) -> bool {
        matches!(self, Self::Font | Self::FontBlur)
    }
    fn blur(self) -> bool {
        matches!(self, Self::Blur | Self::FontBlur)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WideFontScroll {
    scroll: Scroll,
    font: i32,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fixed {
    start_ms: i32,
    end_ms: i32,
    color: i32,
    font: FontOverride,
    x: i16,
    y: i16,
    rows: Range<u16>,
    disposition: NormalDisposition,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Omitted {
    kind: DanmakuKind,
    blocked: Option<BlockReason>,
    start_ms: i32,
    special_lifetime_ms: i32,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Phase {
    Normalized(NormalizedDanmaku),
    Scroll(Scroll),
    WideFontScroll(Box<WideFontScroll>),
    Fixed(Fixed),
    Special(Box<ResolvedSpecial>),
    Metadata { kind: u16, start_ms: i32 },
    Omitted(Omitted),
}
impl Phase {
    pub(crate) fn kind(&self) -> DanmakuKind {
        match self {
            Self::Normalized(value) => value.kind,
            Self::Scroll(value) => value.disposition.kind(),
            Self::WideFontScroll(value) => value.scroll.disposition.kind(),
            Self::Fixed(value) => value.disposition.kind(),
            Self::Special(_) => DanmakuKind::Special,
            Self::Metadata { kind, .. } => DanmakuKind::Unknown(*kind),
            Self::Omitted(value) => value.kind,
        }
    }
    pub(crate) fn blocked(&self) -> Option<BlockReason> {
        match self {
            Self::Normalized(value) => value.blocked,
            Self::Scroll(value) => value.disposition.blocked(),
            Self::WideFontScroll(value) => value.scroll.disposition.blocked(),
            Self::Fixed(value) => value.disposition.blocked(),
            Self::Special(value) => value.blocked,
            Self::Metadata { .. } => None,
            Self::Omitted(value) => value.blocked,
        }
    }
    pub(crate) fn color(&self) -> i32 {
        match self {
            Self::Normalized(value) => value.color,
            Self::Scroll(value) => value.color,
            Self::WideFontScroll(value) => value.scroll.color,
            Self::Fixed(value) => value.color,
            Self::Special(value) => value.color,
            Self::Metadata { .. } | Self::Omitted(_) => 0,
        }
    }
    fn start_ms(&self) -> i64 {
        i64::from(match self {
            Self::Normalized(value) => value.start_ms,
            Self::Scroll(value) => value.start_ms,
            Self::WideFontScroll(value) => value.scroll.start_ms,
            Self::Fixed(value) => value.start_ms,
            Self::Special(value) => value.start_ms,
            Self::Metadata { start_ms, .. } => *start_ms,
            Self::Omitted(value) => value.start_ms,
        })
    }
    fn special_lifetime_ms(&self) -> Result<i64, ConvertError> {
        match self {
            Self::Normalized(value) => value
                .special
                .as_ref()
                .map(|value| value.exist_ms)
                .ok_or_else(|| invalid("missing special payload")),
            Self::Special(value) => Ok(i64::from(value.raw_lifetime_ms)),
            Self::Omitted(value) => Ok(i64::from(value.special_lifetime_ms)),
            _ => Err(invalid("special lifetime on ordinary record")),
        }
    }
    pub(crate) fn resolved(&self) -> ResolvedView<'_> {
        let mut view = ResolvedView {
            start_ms: self.start_ms(),
            end_ms: self.start_ms(),
            emission: Emission::Omit,
            layer: 0,
            style: style(self.kind()),
            color: self.color(),
            placement: Placement::None,
            pixel_rows: None,
            font_size_override: None,
            blur: false,
            special: None,
        };
        match self {
            Self::Scroll(_) | Self::WideFontScroll(_) => {
                let value = match self {
                    Self::Scroll(value) => value,
                    Self::WideFontScroll(value) => &value.scroll,
                    _ => unreachable!(),
                };
                view.end_ms = i64::from(value.end_ms);
                view.emission = if value.disposition.blocked().is_none() {
                    Emission::Dialogue
                } else {
                    Emission::Comment
                };
                // Decode stored endpoints losslessly. Width and position were
                // decided during layout; writer options never enter this view.
                let edge = i64::from(value.screen_width) + value.half_width;
                let opposite = -value.half_width;
                let (from_x, to_x) = if value.disposition.kind() == DanmakuKind::RightToLeft {
                    (edge, opposite)
                } else {
                    (opposite, edge)
                };
                view.placement = Placement::Move {
                    from_x,
                    from_y: i32::from(value.y),
                    to_x,
                    to_y: i32::from(value.y),
                    timing: None,
                };
                view.pixel_rows = Some(usize::from(value.rows.start)..usize::from(value.rows.end));
                view.font_size_override = match self {
                    Self::WideFontScroll(wide) => Some(wide.font),
                    _ => value.effects.font().then_some(i32::from(value.font)),
                };
                view.blur = value.effects.blur();
            }
            Self::Fixed(value) => {
                view.end_ms = i64::from(value.end_ms);
                let visible = value.disposition.blocked().is_none();
                view.emission = if visible {
                    Emission::Dialogue
                } else {
                    Emission::Comment
                };
                view.layer = u8::from(visible);
                view.placement = Placement::Position {
                    x: i64::from(value.x),
                    y: i32::from(value.y),
                };
                view.pixel_rows = Some(usize::from(value.rows.start)..usize::from(value.rows.end));
                view.font_size_override = value.font.get();
            }
            Self::Special(value) => {
                view.end_ms = i64::from(value.end_ms);
                view.emission = if value.blocked.is_none() {
                    Emission::Dialogue
                } else {
                    Emission::Comment
                };
                view.placement = value.placement;
                view.font_size_override = value.font.get();
                view.special = Some(value);
            }
            Self::Metadata { kind, .. } => {
                view.style = if *kind == 8 {
                    Style::Code
                } else {
                    Style::Unknown
                };
                view.emission = Emission::Comment;
            }
            Self::Omitted(_) => {}
            Self::Normalized(_) => panic!("writer requested a record before layout completed"),
        }
        view
    }
}

fn invalid(field: &str) -> ConvertError {
    ConvertError::new(
        ErrorKind::UnsupportedInput,
        format!("invalid or overflowing layout {field}"),
    )
}

fn int(value: i64, field: &str) -> Result<i32, ConvertError> {
    i32::try_from(value).map_err(|_| invalid(field))
}

fn add(left: i64, right: i64, field: &str) -> Result<i64, ConvertError> {
    let value = left.checked_add(right).ok_or_else(|| invalid(field))?;
    int(value, field)?;
    Ok(value)
}

/// Signed C integer division truncates towards zero, including for negative
/// special lifetimes. The addition occurs in i32 before division.
fn quantize_signed(ms: i64) -> Result<i64, ConvertError> {
    Ok(add(ms, 5, "special lifetime rounding")? / 10 * 10)
}

fn style(kind: DanmakuKind) -> Style {
    match kind {
        DanmakuKind::RightToLeft => Style::RightToLeft,
        DanmakuKind::LeftToRight => Style::LeftToRight,
        DanmakuKind::Top => Style::Top,
        DanmakuKind::Bottom => Style::Bottom,
        DanmakuKind::Special => Style::Special,
        DanmakuKind::Unknown(_) => Style::Unknown,
    }
}

fn metrics(source: &Danmaku, font_size: i32) -> Result<(i32, i64), ConvertError> {
    let height = font_size
        .checked_add(
            i32::from(source.normalized().file_font_size)
                .checked_sub(25)
                .ok_or_else(|| invalid("font height"))?,
        )
        .ok_or_else(|| invalid("font height"))?;
    let count = i64::try_from(source.text.chars().count()).map_err(|_| invalid("text length"))?;
    let product = count
        .checked_mul(i64::from(height))
        .ok_or_else(|| invalid("text width"))?;
    // IC-03 retains a full budgeted body instead of C's 1023-byte prefix.
    // At the 128 MiB decoded limit and maximum mapped height 36838 this
    // product is <5e12, fitting i64 and exactly representable in f64. Keep
    // the original double /1.2 and truncation in the entire legacy range.
    let width = checked_i64(product as f64 / 1.2, "text width")?;
    Ok((height, width))
}

fn density_end(event: &LayoutedDanmaku, roll_ms: i64, hold_ms: i64) -> Result<i64, ConvertError> {
    // C's no-position fallback marks every ordinary type as -R2L before
    // later sign-pointer scans. Preserve that lifetime consequence without
    // encoding a blocked state as a negative type or changing output style.
    if event.blocked() == Some(BlockReason::NoPosition) {
        return add(event.phase.start_ms(), roll_ms, "blocked density end time");
    }
    let duration = match event.kind() {
        DanmakuKind::RightToLeft | DanmakuKind::LeftToRight => roll_ms,
        DanmakuKind::Top | DanmakuKind::Bottom => hold_ms,
        DanmakuKind::Special => event.phase.special_lifetime_ms()?,
        DanmakuKind::Unknown(_) => return Ok(0),
    };
    add(event.phase.start_ms(), duration, "density end time")
}

/// The fallback intentionally chooses a single pixel's minimum, excludes the
/// last forward candidate, and may clear equal times even for blocked nodes.
fn find_min(array: &mut [i64], stop: usize, reverse: bool) -> usize {
    if !reverse {
        if stop == 0 {
            return 0;
        }
        let mut minimum = 0;
        for at in 1..stop {
            if array[at] < array[minimum] {
                minimum = at;
            }
        }
        if minimum == 0 && array[0] == array[stop - 1] {
            let value = array[0];
            for time in &mut array[..stop] {
                if *time == value {
                    *time = 0;
                }
            }
        }
        minimum
    } else {
        let rows = array.len();
        if stop >= rows {
            return rows;
        }
        let mut minimum = rows - 1;
        for at in (stop..rows - 1).rev() {
            if array[at] < array[minimum] {
                minimum = at;
            }
        }
        if minimum == rows - 1 && array[minimum] == array[stop] {
            let value = array[minimum];
            for time in &mut array[stop..] {
                if *time == value {
                    *time = 0;
                }
            }
        }
        minimum
    }
}

#[derive(Debug)]
struct ScrollOccupancy {
    entered: Vec<i64>,
    left: Vec<i64>,
}

impl ScrollOccupancy {
    fn new(rows: usize) -> Self {
        Self {
            entered: vec![0; rows],
            left: vec![0; rows],
        }
    }

    fn choose(
        &mut self,
        start: i64,
        height: usize,
        width: i64,
        screen_width: i64,
        roll_ms: i64,
    ) -> Result<Option<usize>, ConvertError> {
        let mut y = 0;
        let last = self.entered.len() - height;
        let mut crossing = None;
        while y <= last {
            let mut conflict = None;
            for offset in 0..height {
                let row = y + offset;
                let occupied = if start < self.entered[row] {
                    true
                } else {
                    // Preserve C's || short circuit. In particular a blocked
                    // node on fully busy rows need not evaluate its velocity.
                    let crossing = match crossing {
                        Some(value) => value,
                        None => {
                            let value = motion_delta(roll_ms, screen_width, width, screen_width)?;
                            crossing = Some(value);
                            value
                        }
                    };
                    add(start, crossing, "scroll crossing time")? < self.left[row]
                };
                if occupied {
                    conflict = Some(offset);
                    break;
                }
            }
            match conflict {
                Some(offset) => y += offset + 1,
                None => return Ok(Some(y)),
            }
        }
        Ok(None)
    }
}

fn motion_delta(
    roll_ms: i64,
    screen_width: i64,
    width: i64,
    distance: i64,
) -> Result<i64, ConvertError> {
    let denominator = screen_width
        .checked_add(width)
        .ok_or_else(|| invalid("scroll denominator"))?;
    // Keep every operation f32, in the C left-to-right order, including Q.
    let seconds = roll_ms as f32 / 1000.0_f32 * distance as f32 / denominator as f32;
    ass_duration_ms(seconds)
}

fn fixed_position(array: &[i64], start: i64, height: usize, bottom: bool) -> Option<usize> {
    if !bottom {
        let mut y = 0;
        while y <= array.len() - height {
            match (0..height).find(|&offset| start < array[y + offset]) {
                Some(offset) => y += offset + 1,
                None => return Some(y),
            }
        }
    } else {
        let mut y = array.len();
        while y >= height {
            match (0..height).find(|&offset| start < array[y - offset - 1]) {
                Some(offset) => y -= offset + 1,
                None => return Some(y),
            }
        }
    }
    None
}

/// Active indices avoid rescanning all preceding nodes for every normal event.
/// The sign pointer is still advanced as in C so its arithmetic checks retain
/// the legacy evaluation boundary. Text buckets own indices, never text copies.
#[derive(Default)]
struct Active {
    ends: BinaryHeap<Reverse<(i64, usize, Option<u64>)>>,
    texts: HashMap<u64, Vec<usize>>,
    hashes: RandomState,
    cursor: usize,
    sign: usize,
}

impl Active {
    fn hash(&self, text: &str) -> u64 {
        self.hashes.hash_one(text)
    }

    fn advance(
        &mut self,
        events: &[LayoutedDanmaku],
        now: usize,
        roll_ms: i64,
        hold_ms: i64,
        repeats: bool,
        stop_on_repeat: bool,
    ) -> Result<bool, ConvertError> {
        let start = events[now].phase.start_ms();
        while density_end(&events[self.sign], roll_ms, hold_ms)? < start {
            self.sign += 1;
        }
        while let Some(&Reverse((end, index, hash))) = self.ends.peek() {
            if end > start {
                break;
            }
            self.ends.pop();
            if let Some(hash) = hash
                && let Some(bucket) = self.texts.get_mut(&hash)
            {
                if let Some(position) = bucket.iter().position(|&at| at == index) {
                    bucket.swap_remove(position);
                }
                if bucket.is_empty() {
                    self.texts.remove(&hash);
                }
            }
        }
        self.cursor = self.cursor.max(self.sign);
        // Registered nodes precede cursor and their end arithmetic has already
        // been evaluated. A duplicate there makes C skip this node before it
        // reaches any newly pending prior special payloads.
        if stop_on_repeat && self.repeated(events, &events[now].text) {
            return Ok(true);
        }
        while self.cursor < now {
            let at = self.cursor;
            self.cursor += 1;
            let event = &events[at];
            if event.blocked().is_some() {
                continue;
            }
            let end = density_end(event, roll_ms, hold_ms)?;
            if end <= start || !(event.kind().is_normal() || event.kind() == DanmakuKind::Special) {
                continue;
            }
            let hash = if repeats && event.kind().is_normal() {
                let hash = self.hash(&event.text);
                self.texts.entry(hash).or_default().push(at);
                Some(hash)
            } else {
                None
            };
            self.ends.push(Reverse((end, at, hash)));
            if stop_on_repeat && event.kind().is_normal() && event.text == events[now].text {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn repeated(&self, events: &[LayoutedDanmaku], text: &str) -> bool {
        self.texts
            .get(&self.hash(text))
            .is_some_and(|bucket| bucket.iter().any(|&at| events[at].text.as_ref() == text))
    }
}

fn omitted(value: &NormalizedDanmaku) -> Result<Omitted, ConvertError> {
    Ok(Omitted {
        kind: value.kind,
        blocked: value.blocked,
        start_ms: value.start_ms,
        special_lifetime_ms: value
            .special
            .as_ref()
            .map(|special| int(special.exist_ms, "special lifetime"))
            .transpose()?
            .unwrap_or(0),
    })
}

fn take_normalized(event: &mut Danmaku) -> Result<NormalizedDanmaku, ConvertError> {
    let placeholder = Phase::Omitted(omitted(event.normalized())?);
    match std::mem::replace(&mut event.phase, placeholder) {
        Phase::Normalized(value) => Ok(value),
        _ => unreachable!("each normalized record is laid out once"),
    }
}

fn special(mut source: NormalizedDanmaku, options: &ConvertOptions) -> Result<Phase, ConvertError> {
    if !options.save_blocked {
        return Ok(Phase::Omitted(omitted(&source)?));
    }
    let special = *source
        .special
        .take()
        .ok_or_else(|| invalid("missing special payload"))?;
    let exist = quantize_signed(special.exist_ms)?;
    let end_ms = int(
        add(i64::from(source.start_ms), exist, "special end time")?,
        "special end time",
    )?;
    let (mut x, mut y, mut end_x, mut end_y) = (
        special.start_x,
        special.start_y,
        special.end_x,
        special.end_y,
    );
    if [x, y, end_x, end_y]
        .into_iter()
        .all(|value| f64::from(value) < 1.0 + 1e-4)
    {
        x *= options.width as f32;
        end_x *= options.width as f32;
        y *= options.height as f32;
        end_y *= options.height as f32;
    }
    // Decide fixed versus moving before integer truncation, just as C does.
    let fixed = x == end_x && y == end_y;
    let (x, y, end_x, end_y) = (
        checked_i32(f64::from(x), "special x")?,
        checked_i32(f64::from(y), "special y")?,
        checked_i32(f64::from(end_x), "special end x")?,
        checked_i32(f64::from(end_y), "special end y")?,
    );
    let placement = if fixed {
        Placement::Position { x: i64::from(x), y }
    } else {
        let timing = if special.pause_ms == 0 {
            (special.move_ms != 0).then_some(MoveTiming {
                start_ms: 0,
                end_ms: special.move_ms,
            })
        } else {
            Some(MoveTiming {
                start_ms: special.pause_ms,
                end_ms: add(special.pause_ms, special.move_ms, "special movement end")?,
            })
        };
        Placement::Move {
            from_x: i64::from(x),
            from_y: y,
            to_x: i64::from(end_x),
            to_y: end_y,
            timing,
        }
    };
    let font = FontOverride::new(
        (source.file_font_size != 25).then_some(i32::from(source.file_font_size)),
    );
    let fade = if special.fade_start == 0 && special.fade_end == 0 {
        SpecialFade::Opaque
    } else {
        SpecialFade::Transition {
            start: special.fade_start,
            end: special.fade_end,
            transition_start_ms: special.pause_ms,
            transition_end_ms: exist,
        }
    };
    Ok(Phase::Special(Box::new(ResolvedSpecial {
        rotation_y: special.rotation_y,
        rotation_z: special.rotation_z,
        font_name: special.font_name.into_boxed_str(),
        fade,
        start_ms: source.start_ms,
        end_ms,
        raw_lifetime_ms: int(special.exist_ms, "special lifetime")?,
        color: source.color,
        font,
        placement,
        blocked: source.blocked,
    })))
}

pub(crate) fn layout(
    mut pool: Vec<Danmaku>,
    options: &ConvertOptions,
) -> Result<LayoutedPool, ConvertError> {
    let roll_ms = ass_duration_ms(options.scroll_time)?;
    let hold_ms = ass_duration_ms(options.fix_time)?;
    let full_height = int(i64::from(options.height), "resolution")?;
    let screen_width = i64::from(int(i64::from(options.width), "resolution")?);
    let mut hold_rows =
        if f64::from(options.display_area) > 1e-4 && f64::from(options.display_area) < 1.0 - 1e-4 {
            checked_i32(
                f64::from(full_height as f32 * options.display_area),
                "display area",
            )?
        } else {
            full_height
        };
    let mut roll_rows = hold_rows;
    let area_height = full_height as f32 * options.scroll_area;
    if f64::from((options.scroll_area - 1.0_f32).abs()) > 1e-4 && area_height < roll_rows as f32 {
        roll_rows = checked_i32(f64::from(area_height), "scroll area")?;
    }
    hold_rows = hold_rows.max(1);
    roll_rows = roll_rows.max(1);
    let mut right = ScrollOccupancy::new(roll_rows as usize);
    let mut left = ScrollOccupancy::new(roll_rows as usize);
    let mut fixed = vec![0; hold_rows as usize];
    for source in &mut pool {
        source.normalized_mut().start_ms = int(ass_start_ms(source.source_time_ms)?, "start time")?;
    }
    let mut events = pool;
    let mut active = Active::default();
    let repeats = options.block_mode & block::REPEAT != 0;
    for now in 0..events.len() {
        if (options.density > 0 || repeats) && events[now].kind().is_normal() {
            let stop_on_repeat =
                repeats && !options.save_blocked && events[now].blocked().is_none();
            if active.advance(&events, now, roll_ms, hold_ms, repeats, stop_on_repeat)? {
                events[now].normalized_mut().blocked = Some(BlockReason::Repeat);
                take_normalized(&mut events[now])?;
                continue;
            }
            if repeats
                && events[now].blocked().is_none()
                && active.repeated(&events, &events[now].text)
            {
                events[now].normalized_mut().blocked = Some(BlockReason::Repeat);
                if !options.save_blocked {
                    take_normalized(&mut events[now])?;
                    continue;
                }
            }
            if events[now].blocked().is_none()
                && options.density > 0
                && active.ends.len() >= options.density as usize
            {
                events[now].normalized_mut().blocked = Some(BlockReason::Density);
                if !options.save_blocked {
                    take_normalized(&mut events[now])?;
                    continue;
                }
            }
        }
        // C measures all remaining nodes, including unknown/omitted specials.
        let (height, width) = metrics(&events[now], options.font_size)?;
        let line_height = height
            .checked_add(options.line_spacing)
            .ok_or_else(|| invalid("line height"))?;
        let kind = events[now].kind();
        if kind == DanmakuKind::Special {
            let normalized = take_normalized(&mut events[now])?;
            events[now].phase = special(normalized, options)?;
            continue;
        }
        if !kind.is_normal() {
            let normalized = take_normalized(&mut events[now])?;
            if normalized.blocked.is_none()
                && let DanmakuKind::Unknown(value) = kind
                && value > 0
            {
                events[now].phase = Phase::Metadata {
                    kind: value,
                    start_ms: normalized.start_ms,
                };
            }
            continue;
        }
        let scrolling = matches!(kind, DanmakuKind::RightToLeft | DanmakuKind::LeftToRight);
        let rows = if scrolling { roll_rows } else { hold_rows } as usize;
        let line_height = line_height.clamp(1, rows as i32) as usize;
        let start = events[now].phase.start_ms();
        let bottom = kind == DanmakuKind::Bottom;
        let position = if scrolling {
            let occupancy = if kind == DanmakuKind::RightToLeft {
                &mut right
            } else {
                &mut left
            };
            occupancy.choose(start, line_height, width, screen_width, roll_ms)?
        } else {
            fixed_position(&fixed, start, line_height, bottom)
        };
        let position = match position {
            Some(position) => position,
            None => {
                if options.density == -1 {
                    events[now].normalized_mut().blocked = Some(BlockReason::NoPosition);
                    if !options.save_blocked {
                        take_normalized(&mut events[now])?;
                        continue;
                    }
                }
                if scrolling {
                    let occupancy = if kind == DanmakuKind::RightToLeft {
                        &mut right
                    } else {
                        &mut left
                    };
                    find_min(&mut occupancy.entered, rows - line_height, false)
                } else {
                    find_min(
                        &mut fixed,
                        if bottom {
                            line_height
                        } else {
                            rows - line_height
                        },
                        bottom,
                    )
                }
            }
        };
        let range = if bottom {
            position - line_height..position
        } else {
            position..position + line_height
        };
        let visible = events[now].blocked().is_none();
        let duration = if scrolling { roll_ms } else { hold_ms };
        let end = add(start, duration, "event end time")?;
        if visible {
            if scrolling {
                let occupancy = if kind == DanmakuKind::RightToLeft {
                    &mut right
                } else {
                    &mut left
                };
                let entered = add(
                    start,
                    motion_delta(roll_ms, screen_width, width, width)?,
                    "scroll entry time",
                )?;
                occupancy.entered[range.clone()].fill(entered);
                occupancy.left[range.clone()].fill(end);
            } else {
                fixed[range.clone()].fill(end);
            }
        }
        let source = take_normalized(&mut events[now])?;
        let disposition = NormalDisposition::new(kind, source.blocked);
        let font = FontOverride::new((height != options.font_size).then_some(height));
        let rows = u16::try_from(range.start).map_err(|_| invalid("pixel range"))?
            ..u16::try_from(range.end).map_err(|_| invalid("pixel range"))?;
        events[now].phase = if scrolling {
            screen_width
                .checked_add(width / 2)
                .ok_or_else(|| invalid("scroll coordinate"))?;
            width
                .checked_neg()
                .ok_or_else(|| invalid("scroll coordinate"))?;
            let y = int(
                add(position as i64, i64::from(options.top_margin), "top margin")?,
                "top margin",
            )?;
            let override_font = font.get();
            let compact_font = override_font.map(i16::try_from).transpose();
            let scroll = Scroll {
                half_width: width / 2,
                screen_width: u16::try_from(screen_width)
                    .map_err(|_| invalid("scroll resolution"))?,
                y: i16::try_from(y).map_err(|_| invalid("ordinary y"))?,
                start_ms: source.start_ms,
                end_ms: int(end, "end time")?,
                color: source.color,
                font: compact_font.as_ref().ok().copied().flatten().unwrap_or(0),
                rows,
                disposition,
                effects: ScrollEffects::new(
                    override_font.is_some(),
                    kind == DanmakuKind::RightToLeft
                        && options.outline > 0.0
                        && options.outline_blur > 0.0,
                ),
            };
            match compact_font {
                Ok(_) => Phase::Scroll(scroll),
                Err(_) => Phase::WideFontScroll(Box::new(WideFontScroll {
                    scroll,
                    font: override_font.expect("wide override exists"),
                })),
            }
        } else {
            let y = if bottom {
                let y = position as i64 - line_height as i64;
                int(
                    y.checked_sub(i64::from(options.bottom_margin))
                        .ok_or_else(|| invalid("bottom margin"))?,
                    "bottom margin",
                )?
            } else {
                int(
                    add(position as i64, i64::from(options.top_margin), "top margin")?,
                    "top margin",
                )?
            };
            Phase::Fixed(Fixed {
                x: i16::try_from(screen_width / 2).map_err(|_| invalid("fixed x"))?,
                y: i16::try_from(y).map_err(|_| invalid("ordinary y"))?,
                start_ms: source.start_ms,
                end_ms: int(end, "end time")?,
                color: source.color,
                font,
                rows,
                disposition,
            })
        };
    }
    Ok(LayoutedPool {
        records: events,
        blur: options.outline_blur,
    })
}

#[cfg(test)]
mod tests {
    use super::super::model::normalize_and_sort;
    use super::super::special::parse;
    use super::*;

    fn options() -> ConvertOptions {
        ConvertOptions {
            width: 100,
            height: 20,
            font_size: 10,
            scroll_time: 1.0,
            fix_time: 1.0,
            ..Default::default()
        }
    }

    fn normal(sequence: usize, start: i64, mode: i32, text: &str) -> Danmaku {
        let mut item = Danmaku::new(sequence, start, mode, 25, 0xffffff);
        item.text = text.into();
        item
    }

    fn advanced(sequence: usize, start: i64, payload: &str) -> Danmaku {
        let (special, text) = parse(payload).unwrap();
        let mut item = normal(sequence, start, 7, &text);
        item.normalized_mut().special = Some(Box::new(special));
        item
    }

    fn run(mut pool: Vec<Danmaku>, options: &ConvertOptions) -> LayoutedPool {
        normalize_and_sort(&mut pool, options);
        layout(pool, options).unwrap()
    }

    #[test]
    fn ordinary_records_reuse_the_parsed_allocation_and_keep_immutable_source_keys() {
        // The measured regression was two live metadata allocations, not body
        // bytes. Check ownership reuse directly, without allocator/RSS noise.
        let mut pool = Vec::with_capacity(128);
        for sequence in 0..100 {
            pool.push(normal(sequence, (sequence % 7) as i64 * 3, 1, "full body"));
        }
        normalize_and_sort(&mut pool, &options());
        let before: Vec<_> = pool
            .iter()
            .map(|item| (item.input_sequence, item.source_time_ms, item.text.as_ptr()))
            .collect();
        let pointer = pool.as_ptr();
        let capacity = pool.capacity();
        let finalized = layout(pool, &options()).unwrap();
        assert_eq!(finalized.as_ptr(), pointer);
        assert_eq!(finalized.capacity(), capacity);
        assert_eq!(
            finalized
                .iter()
                .map(|item| (item.input_sequence, item.source_time_ms, item.text.as_ptr()))
                .collect::<Vec<_>>(),
            before
        );
        assert!(
            finalized
                .iter()
                .all(|item| !matches!(item.phase, Phase::Normalized(_)))
        );
        // Native enum niches make the current 64-bit ordinary record 64 B;
        // retain a size ceiling so future inline advanced fields cannot
        // quietly reintroduce the measured ~70 MiB near-limit regression.
        #[cfg(target_pointer_width = "64")]
        assert!(
            std::mem::size_of::<Danmaku>() <= 64,
            "record is {} bytes",
            std::mem::size_of::<Danmaku>()
        );
    }

    #[test]
    fn compact_and_wide_font_paths_preserve_boundary_overrides_and_coordinates() {
        use super::super::writer;
        for (base, file, height, half, wide) in [
            (1, -32744, -32768, -13653, false),
            (1, -32768, -32792, -13663, true),
            (4096, 32767, 36838, 15349, true),
        ] {
            let o = ConvertOptions {
                font_size: base,
                outline: 1.0,
                outline_blur: 2.5,
                ..Default::default()
            };
            let mut item = normal(0, 0, 1, "x");
            item.normalized_mut().file_font_size = file;
            let events = layout(vec![item], &o).unwrap();
            assert_eq!(matches!(events[0].phase, Phase::WideFontScroll(_)), wide);
            let view = events[0].resolved();
            assert_eq!(view.font_size_override, Some(height));
            assert!(view.blur);
            assert_eq!(
                view.placement,
                Placement::Move {
                    from_x: 1920 + half,
                    from_y: 0,
                    to_x: -half,
                    to_y: 0,
                    timing: None
                }
            );
            let mut output = Vec::new();
            writer::write(&mut output, &events, &o).unwrap();
            assert!(
                String::from_utf8(output)
                    .unwrap()
                    .contains(&format!("\\fs{height}\\blur2.5"))
            );
        }
        // A real override to font 0 is distinct from no font override, too.
        let o = options();
        let mut item = normal(0, 0, 6, "x");
        item.normalized_mut().file_font_size = 15;
        let events = layout(vec![item], &o).unwrap();
        assert_eq!(events[0].resolved().font_size_override, Some(0));
        assert!(!events[0].resolved().blur);
    }

    #[test]
    fn rolling_uses_entry_and_catch_up_boundaries_and_independent_directions() {
        // h=10, width=floor(12*10/1.2)=100. First is fully entered at
        // 500ms and fully gone at 1000ms. A width-200 follower would reach
        // the opposite edge in 330ms and must wait until 670ms to use row 0.
        let mut o = options();
        o.height = 10;
        o.density = -1;
        let events = run(
            vec![
                normal(0, 0, 1, "abcdefghijkl"),
                normal(1, 500, 1, "abcdefghijklmnopqrstuvwx"),
                normal(2, 500, 6, "abcdefghijkl"),
                normal(3, 660, 1, "abcdefghijklmnopqrstuvwx"),
                normal(4, 670, 1, "abcdefghijklmnopqrstuvwx"),
            ],
            &o,
        );
        assert_eq!(
            events
                .iter()
                .map(|event| event.resolved().emission)
                .collect::<Vec<_>>(),
            [
                Emission::Dialogue,
                Emission::Comment,
                Emission::Dialogue,
                Emission::Comment,
                Emission::Dialogue,
            ]
        );
        assert_eq!(events[4].resolved().pixel_rows, Some(0..10));
        assert_eq!(
            events[4].resolved().placement,
            Placement::Move {
                from_x: 200,
                from_y: 0,
                to_x: -100,
                to_y: 0,
                timing: None
            }
        );
        assert_eq!(events[0].source_time_ms, 0);
        assert_eq!(events[4].resolved().start_ms, 670);
    }

    #[test]
    fn fixed_types_share_rows_scan_opposite_directions_and_allow_exact_end() {
        let events = run(
            vec![
                normal(0, 0, 5, "top"),
                normal(1, 0, 4, "bottom"),
                normal(2, 1000, 5, "later"),
            ],
            &options(),
        );
        assert_eq!(events[0].resolved().pixel_rows, Some(0..10));
        assert_eq!(events[1].resolved().pixel_rows, Some(10..20));
        assert_eq!(events[2].resolved().pixel_rows, Some(0..10));
        assert_eq!(events[0].resolved().layer, 1);
        assert_eq!(
            events[1].resolved().placement,
            Placement::Position { x: 50, y: 10 }
        );
    }

    #[test]
    fn fallback_mutates_equal_rows_and_blocked_nodes_do_not_register_occupancy() {
        let mut o = options();
        o.height = 30;
        let events = run(
            vec![
                normal(0, 0, 5, "one"),
                normal(1, 0, 5, "two"),
                normal(2, 0, 5, "three"),
                normal(3, 0, -3, "blocked"),
                normal(4, 0, 5, "next"),
            ],
            &o,
        );
        assert_eq!(events[3].resolved().emission, Emission::Comment);
        assert_eq!(events[3].resolved().pixel_rows, Some(0..10));
        // The blocked fallback clears equal times in [0,20), allowing the
        // next visible node to reuse row 0 even though it never registers.
        assert_eq!(events[4].resolved().pixel_rows, Some(0..10));
        let mut forward = vec![9, 9, 9, 1];
        assert_eq!(find_min(&mut forward, 3, false), 0);
        assert_eq!(forward, [0, 0, 0, 1]);
        let mut reverse = vec![1, 9, 9, 9];
        assert_eq!(find_min(&mut reverse, 4, true), 4);
        assert_eq!(find_min(&mut reverse, 1, true), 3);
        assert_eq!(reverse, [1, 0, 0, 0]);
    }

    #[test]
    fn density_and_repeat_use_live_logical_text_and_special_raw_lifetime() {
        let mut o = options();
        o.density = 1;
        o.save_blocked = false;
        let events = run(
            vec![
                advanced(0, 0, r#"[0,0,"1-1",0.014,"sp",0,0,0,0,0,0,true,"",1]"#),
                normal(1, 10, 1, "ordinary"),
                normal(2, 20, 1, "later"),
            ],
            &o,
        );
        assert_eq!(events[0].resolved().emission, Emission::Omit);
        assert_eq!(events[1].blocked(), Some(BlockReason::Density));
        assert_eq!(events[1].resolved().emission, Emission::Omit);
        assert_eq!(events[2].resolved().emission, Emission::Dialogue);

        o.density = 0;
        o.block_mode = block::REPEAT;
        let events = run(
            vec![
                normal(0, 0, 1, "same"),
                normal(1, 0, 5, "same"),
                normal(2, 1000, 4, "same"),
            ],
            &o,
        );
        assert_eq!(events[1].blocked(), Some(BlockReason::Repeat));
        assert_eq!(events[2].blocked(), None);
    }

    #[test]
    fn save_blocked_and_unknown_types_keep_frozen_emission_rules() {
        let mut o = options();
        o.save_blocked = false;
        o.block_mode = block::COLOR;
        let mut color = normal(0, 0, 1, "red");
        color.normalized_mut().color = 0xff0000;
        let events = run(
            vec![
                color,
                normal(1, 0, -2, "negative"),
                normal(2, 0, 8, "code"),
                normal(3, 0, 9, "unknown"),
                normal(4, 0, 0, "zero"),
                normal(5, 0, -9, "negative unknown"),
            ],
            &o,
        );
        assert_eq!(
            events
                .iter()
                .map(|event| event.resolved().emission)
                .collect::<Vec<_>>(),
            [
                Emission::Comment,
                Emission::Comment,
                Emission::Comment,
                Emission::Comment,
                Emission::Omit,
                Emission::Omit,
            ]
        );
        assert_eq!(events[2].resolved().style, Style::Code);
        assert_eq!(events[3].input_sequence, 3);
    }

    #[test]
    fn special_layout_resolves_full_screen_motion_timing_fade_and_font() {
        let o = options();
        let event = run(
            vec![advanced(
                0,
                1004,
                r#"[-0.5,0.25,"1-0.5",1.235,"{\b1}raw",20,-30,0.75,1,700,200,true,"font",1]"#,
            )],
            &o,
        )
        .into_records()
        .pop()
        .unwrap();
        assert_eq!(event.resolved().start_ms, 1000);
        assert_eq!(event.resolved().end_ms, 2240);
        assert_eq!(
            event.resolved().placement,
            Placement::Move {
                from_x: -50,
                from_y: 5,
                to_x: 75,
                to_y: 20,
                timing: Some(MoveTiming {
                    start_ms: 200,
                    end_ms: 900
                })
            }
        );
        assert_eq!(
            event.resolved().special.map(|special| special.fade),
            Some(SpecialFade::Transition {
                start: 0,
                end: 127,
                transition_start_ms: 200,
                transition_end_ms: 1240
            })
        );
        assert_eq!(event.text.as_ref(), r"{\b1}raw");
        assert_eq!(event.resolved().special.unwrap().rotation_z, 20);
        assert_eq!(event.resolved().special.unwrap().rotation_y, -30);
        assert_eq!(event.resolved().special.unwrap().font_name.as_ref(), "font");
        assert_eq!(event.resolved().font_size_override, None);
    }

    #[test]
    fn special_decides_motion_before_integer_truncation_and_checks_only_used_sums() {
        let o = options();
        let events = run(
            vec![
                advanced(0, 0, r#"[2.1,3,"1-1",-0.02,"x",0,0,2.9,3,0,0,true,"",1]"#),
                advanced(1, 0, r#"[2,3,"1-1",1,"x",0,0,2,3,2147483647,1,true,"",1]"#),
            ],
            &o,
        );
        assert_eq!(
            events[0].resolved().placement,
            Placement::Move {
                from_x: 2,
                from_y: 3,
                to_x: 2,
                to_y: 3,
                timing: None
            }
        );
        assert_eq!(events[0].resolved().end_ms, -10);
        assert_eq!(
            events[1].resolved().placement,
            Placement::Position { x: 2, y: 3 }
        );
        let mut moving = vec![advanced(
            0,
            0,
            r#"[2,3,"1-1",1,"x",0,0,3,3,2147483647,1,true,"",1]"#,
        )];
        normalize_and_sort(&mut moving, &o);
        assert_eq!(
            layout(moving, &o).unwrap_err().kind,
            ErrorKind::UnsupportedInput
        );
    }

    #[test]
    fn display_area_line_spacing_margins_and_r2l_only_blur_are_resolved() {
        let mut o = options();
        o.display_area = 0.5;
        o.scroll_area = 0.25;
        o.line_spacing = -50;
        o.top_margin = -3;
        o.bottom_margin = 2;
        o.outline = 1.0;
        o.outline_blur = 2.5;
        let events = run(
            vec![
                normal(0, 0, 1, "one"),
                normal(1, 0, 6, "two"),
                normal(2, 0, 4, "bottom"),
            ],
            &o,
        );
        assert_eq!(events[0].resolved().pixel_rows, Some(0..1));
        assert_eq!(
            events[0].resolved().placement,
            Placement::Move {
                from_x: 112,
                from_y: -3,
                to_x: -12,
                to_y: -3,
                timing: None
            }
        );
        assert!(events[0].resolved().blur);
        assert!(!events[1].resolved().blur);
        assert_eq!(events.blur, 2.5);
        assert_eq!(
            events[2].resolved().placement,
            Placement::Position { x: 50, y: 7 }
        );
    }

    #[test]
    fn invalid_arithmetic_is_rejected_but_early_omission_preserves_short_circuit() {
        let mut o = options();
        o.height = 10;
        o.font_size = 4096;
        let huge = normal(0, 0, 1, "xx");
        // Width products intentionally use i64 for complete IC-03 text.
        // File sizes are stored in their native i16 domain; an invalid host
        // option instead exercises the still-checked i32 height arithmetic.
        let invalid = ConvertOptions {
            font_size: i32::MAX,
            ..o.clone()
        };
        let mut oversized_height = huge;
        oversized_height.normalized_mut().file_font_size = i16::MAX;
        assert!(layout(vec![oversized_height], &invalid).is_err());
        let mut late = normal(0, 2_147_483_000, 1, "x");
        assert!(layout(vec![late.clone()], &o).is_err());
        late.normalized_mut().blocked = Some(BlockReason::Input);
        o.save_blocked = false;
        // Preblocked ordinary nodes are still emitted and compute their end.
        assert!(layout(vec![late], &o).is_err());
        let mut skipped = advanced(
            0,
            2_147_483_000,
            r#"[1e20,0,"1-1",2147483,"x",0,0,1e20,0,0,0,true,"",1]"#,
        );
        skipped.source_time_ms = 2_147_483_000;
        assert_eq!(
            layout(vec![skipped], &o).unwrap()[0].resolved().emission,
            Emission::Omit
        );
    }

    #[test]
    fn complete_long_text_has_wide_width_denominator_and_horizontal_coordinates() {
        let mut o = ConvertOptions {
            font_size: 4096,
            ..Default::default()
        };
        let mut source = normal(0, 0, 1, "");
        source.text = "x".repeat(2 * 1024 * 1024).into_boxed_str();
        // Independent rational width: floor(2^21 * 4096 * 5 / 6).
        let events = layout(vec![source], &o).unwrap();
        assert_eq!(events[0].text.len(), 2_097_152);
        assert_eq!(
            events[0].resolved().placement,
            Placement::Move {
                from_x: 3_579_141_333,
                from_y: 0,
                to_x: -3_579_139_413,
                to_y: 0,
                timing: None,
            }
        );
        assert_eq!(events[0].resolved().end_ms, 12_000);
        // Type 7 retains the defined C i32 coordinate domain despite the
        // ordinary IC-03 geometry widening, including after ratio scaling.
        let mut special = advanced(
            0,
            0,
            r#"[2147483648,0,"1-1",1,"x",0,0,2147483648,0,0,0,true,"",1]"#,
        );
        o.save_blocked = true;
        assert_eq!(
            layout(vec![special.clone()], &o).unwrap_err().kind,
            ErrorKind::UnsupportedInput
        );
        special.normalized_mut().special.as_mut().unwrap().start_x = -2_000_000.0;
        special.normalized_mut().special.as_mut().unwrap().end_x = -2_000_000.0;
        assert_eq!(
            layout(vec![special], &o).unwrap_err().kind,
            ErrorKind::UnsupportedInput
        );
    }

    #[test]
    fn actual_large_and_raw_budget_boundary_xml_parse_layout_and_serialize_completely() {
        use super::super::{parse as xml, writer};
        use crate::limits::MAX_XML_BYTES;
        use std::io::{Read, Write};

        #[derive(Default)]
        struct CountingSink {
            bytes: u64,
            xs: usize,
            prefix: Vec<u8>,
        }
        impl Write for CountingSink {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.bytes += bytes.len() as u64;
                self.xs += bytes.iter().filter(|&&byte| byte == b'x').count();
                let remaining = 4096usize.saturating_sub(self.prefix.len());
                self.prefix
                    .extend_from_slice(&bytes[..remaining.min(bytes.len())]);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        const HEADER: &[u8] = b"<i><d p=\"0,1,25,16777215,0\">";
        const FOOTER: &[u8] = b"</d></i>";
        for (body_bytes, from_x, to_x) in [
            (64 * 1024 * 1024, 1_062_558_933, -1_062_557_013),
            (
                MAX_XML_BYTES as usize - HEADER.len() - FOOTER.len(),
                2_125_115_376,
                -2_125_113_456,
            ),
        ] {
            // No second input-sized String/Vec is materialized: the real XML
            // parser consumes a bounded repeat reader, retaining one body.
            let reader = HEADER
                .chain(std::io::repeat(b'x').take(body_bytes as u64))
                .chain(FOOTER);
            let mut pool = xml::read(reader, xml::Budget::RAW).unwrap();
            assert_eq!(pool.len(), 1);
            assert_eq!(pool[0].text.len(), body_bytes);
            assert!(pool[0].text.as_bytes().iter().all(|&byte| byte == b'x'));
            let o = ConvertOptions::default();
            normalize_and_sort(&mut pool, &o);
            let events = layout(pool, &o).unwrap();
            assert_eq!(
                events[0].resolved().placement,
                Placement::Move {
                    from_x,
                    from_y: 0,
                    to_x,
                    to_y: 0,
                    timing: None,
                }
            );
            assert_eq!(events[0].resolved().emission, Emission::Dialogue);
            let mut sink = CountingSink::default();
            writer::write(&mut sink, &events, &o).unwrap();
            // The frozen header's "Text" column contains the only other
            // lowercase x. Counting bytes verifies the full serialized body
            // without accumulating another full ASS in memory.
            assert_eq!(sink.xs, body_bytes + 1);
            assert!(sink.bytes > body_bytes as u64);
            let prefix = std::str::from_utf8(&sink.prefix).unwrap();
            assert!(prefix.contains(&format!("\\move({from_x},0,{to_x},0)")));
        }
    }

    #[test]
    fn repeat_omission_stops_before_later_prior_special_time_arithmetic() {
        let mut o = options();
        o.save_blocked = false;
        o.block_mode = block::REPEAT;
        let special = advanced(
            1,
            10,
            r#"[0,0,"1-1",2147483.647,"sp",0,0,0,0,0,0,true,"",1]"#,
        );
        // The special's density end would overflow. It is never evaluated:
        // signPtr stops at the first normal, whose same text triggers goto.
        let events = run(
            vec![
                normal(0, 0, 1, "same"),
                special.clone(),
                normal(2, 10, 5, "same"),
            ],
            &o,
        );
        assert_eq!(events[2].blocked(), Some(BlockReason::Repeat));
        assert_eq!(events[2].resolved().emission, Emission::Omit);

        let mut pool = vec![
            normal(0, 0, 1, "same"),
            special,
            normal(2, 10, 5, "different"),
        ];
        normalize_and_sort(&mut pool, &o);
        assert_eq!(
            layout(pool, &o).unwrap_err().kind,
            ErrorKind::UnsupportedInput
        );
    }

    #[test]
    fn deterministic_fixed_seed_density_repeat_and_stats_constraints() {
        let mut seed = 0x3813e93bu64;
        let mut pool = Vec::new();
        for sequence in 0..2000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let kind = [1, 6, 5, 4][(seed as usize >> 16) % 4];
            pool.push(normal(
                sequence,
                (sequence / 8) as i64 * 10,
                kind,
                &format!("text{}", seed % 16),
            ));
        }
        let mut o = options();
        o.density = 20;
        o.block_mode = block::REPEAT;
        normalize_and_sort(&mut pool, &o);
        let first = layout(pool.clone(), &o).unwrap();
        let second = layout(pool, &o).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 2000);
        assert!(
            first
                .iter()
                .filter(|event| event.blocked().is_none())
                .count()
                <= 16 * 3
        );
    }
}
