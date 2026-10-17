use super::layout::{Phase, ResolvedView};
use super::special::SpecialDanmaku;
use crate::{ConvertOptions, block};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DanmakuKind {
    RightToLeft,
    LeftToRight,
    Top,
    Bottom,
    Special,
    Unknown(u16),
}

impl DanmakuKind {
    fn from_xml(value: i32) -> (Self, bool) {
        // The frozen ABI first converts to signed short, then maps positive
        // XML values. Negative values already use the internal type mapping.
        let raw = value as i16 as i32;
        let internal = match raw {
            6 => 2,
            5 => 3,
            7 => 5,
            other => other,
        };
        let kind = match internal.abs() {
            1 => Self::RightToLeft,
            2 => Self::LeftToRight,
            3 => Self::Top,
            4 => Self::Bottom,
            5 => Self::Special,
            _ => Self::Unknown(internal.unsigned_abs() as u16),
        };
        (kind, internal < 0)
    }

    pub(crate) fn block_flag(self) -> u32 {
        match self {
            Self::RightToLeft => block::R2L,
            Self::LeftToRight => block::L2R,
            Self::Top => block::TOP,
            Self::Bottom => block::BOTTOM,
            Self::Special => block::SPECIAL,
            Self::Unknown(_) => 0,
        }
    }

    pub(crate) fn is_normal(self) -> bool {
        matches!(
            self,
            Self::RightToLeft | Self::LeftToRight | Self::Top | Self::Bottom
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockReason {
    Input,
    Type,
    Color,
    Density,
    Repeat,
    NoPosition,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Danmaku {
    pub input_sequence: u32,
    pub source_time_ms: i64,
    pub text: Box<str>,
    pub(crate) phase: Phase,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NormalizedDanmaku {
    pub kind: DanmakuKind,
    pub blocked: Option<BlockReason>,
    pub file_font_size: i16,
    pub color: i32,
    pub special: Option<Box<SpecialDanmaku>>,
    pub start_ms: i32,
}

impl Danmaku {
    pub(crate) fn new(sequence: usize, time: i64, raw_kind: i32, size: i32, color: i32) -> Self {
        let (kind, blocked) = DanmakuKind::from_xml(raw_kind);
        Self {
            input_sequence: u32::try_from(sequence)
                .expect("sequence is bounded by the comment budget"),
            source_time_ms: time,
            text: Box::default(),
            phase: Phase::Normalized(NormalizedDanmaku {
                kind,
                blocked: blocked.then_some(BlockReason::Input),
                file_font_size: size as i16,
                color,
                special: None,
                start_ms: 0,
            }),
        }
    }

    pub(crate) fn normalized(&self) -> &NormalizedDanmaku {
        match &self.phase {
            Phase::Normalized(value) => value,
            _ => panic!("normalization requested after layout"),
        }
    }

    pub(crate) fn normalized_mut(&mut self) -> &mut NormalizedDanmaku {
        match &mut self.phase {
            Phase::Normalized(value) => value,
            _ => panic!("normalization requested after layout"),
        }
    }

    pub(crate) fn kind(&self) -> DanmakuKind {
        self.phase.kind()
    }
    pub(crate) fn blocked(&self) -> Option<BlockReason> {
        self.phase.blocked()
    }
    pub(crate) fn color(&self) -> i32 {
        self.phase.color()
    }
    pub(crate) fn resolved(&self) -> ResolvedView<'_> {
        self.phase.resolved()
    }
}

pub(crate) fn normalize_and_sort(pool: &mut [Danmaku], options: &ConvertOptions) {
    for item in pool.iter_mut() {
        if item.blocked().is_some() || item.kind() == DanmakuKind::Unknown(0) {
            continue;
        }
        if options.block_mode & block::COLOR != 0
            && item.kind() != DanmakuKind::Special
            && item.color() != 0xffffff
        {
            item.normalized_mut().blocked = Some(BlockReason::Color);
        } else if options.block_mode & item.kind().block_flag() != 0 {
            item.normalized_mut().blocked = Some(BlockReason::Type);
        }
    }
    let positive_special =
        |item: &Danmaku| item.kind() == DanmakuKind::Special && item.blocked().is_none();
    let normalize = options.font_size_strict
        || (options.font_size_norm
            && pool.iter().any(|item| {
                !positive_special(item)
                    && (item.normalized().file_font_size <= 0
                        || i64::from(item.normalized().file_font_size) >= i64::from(options.height))
            }));
    if normalize {
        for item in pool.iter_mut().filter(|item| !positive_special(item)) {
            item.normalized_mut().file_font_size = 25;
        }
    }
    // sort_by_key is stable. The immutable pre-ASS key retains millisecond
    // ordering even when several events later share one centisecond.
    pool.sort_by_key(|item| item.source_time_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_is_complete_stable_and_uses_milliseconds() {
        for times in [
            vec![2000, 1000, 3000],
            vec![2000, 1000],
            vec![3000, 2000, 1000],
            vec![1004, 1001, 1001],
        ] {
            let mut pool: Vec<_> = times
                .iter()
                .enumerate()
                .map(|(i, &t)| Danmaku::new(i, t, 1, 25, 0xffffff))
                .collect();
            normalize_and_sort(&mut pool, &ConvertOptions::default());
            let mut independent: Vec<_> = times.iter().copied().enumerate().collect();
            independent.sort_by_key(|&(sequence, time)| (time, sequence));
            assert_eq!(
                pool.iter()
                    .map(|d| (d.input_sequence as usize, d.source_time_ms))
                    .collect::<Vec<_>>(),
                independent
            );
        }
        let mut seed = 0x3813e93bu64;
        for _ in 0..128 {
            let mut pool: Vec<_> = (0..256)
                .map(|sequence| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    Danmaku::new(sequence, ((seed >> 32) % 17) as i64, 1, 25, 0xffffff)
                })
                .collect();
            normalize_and_sort(&mut pool, &ConvertOptions::default());
            assert!(
                pool.windows(2)
                    .all(|p| p[0].source_time_ms < p[1].source_time_ms
                        || p[0].source_time_ms == p[1].source_time_ms
                            && p[0].input_sequence < p[1].input_sequence)
            );
        }
    }

    #[test]
    fn negative_and_alias_types_keep_frozen_meanings() {
        assert_eq!(DanmakuKind::from_xml(2), (DanmakuKind::LeftToRight, false));
        assert_eq!(DanmakuKind::from_xml(3), (DanmakuKind::Top, false));
        assert_eq!(DanmakuKind::from_xml(5), (DanmakuKind::Top, false));
        assert_eq!(DanmakuKind::from_xml(-5), (DanmakuKind::Special, true));
        assert_eq!(
            DanmakuKind::from_xml(-32768),
            (DanmakuKind::Unknown(32768), true)
        );
    }

    #[test]
    fn blocked_special_participates_in_font_normalization() {
        let mut pool = vec![
            Danmaku::new(0, 0, 7, 0, 0),
            Danmaku::new(1, 0, 1, 30, 0xffffff),
        ];
        let mut options = ConvertOptions {
            font_size_norm: true,
            ..Default::default()
        };
        normalize_and_sort(&mut pool, &options);
        assert_eq!(pool[1].normalized().file_font_size, 30);
        options.block_mode = block::SPECIAL;
        normalize_and_sort(&mut pool, &options);
        assert_eq!(pool[0].blocked(), Some(BlockReason::Type));
        assert_eq!(pool[0].normalized().file_font_size, 25);
        assert_eq!(pool[1].normalized().file_font_size, 25);
    }
}
