//! The TABLE and HISTOGRAM ASS overlays from DanmakuFactory.

use std::io::{self, Write};

use crate::ass::{end_time, write_time};
use crate::model::Danmaku;

const STYLE: &str = "danmakuFactory_stat,,0000,0000,0000,,";
const CLIP_TABLE: &str = "m 20 290 b 20 284 29 275 35 275 l 430 275 b 436 275 445 284 445 290 l 445 370 b 445 376 436 385 430 385 l 35 385 b 29 385 20 376 20 370 l 20 290";
const CLIP_HISTOGRAM: &str = "m 20 35 b 20 29 29 20 35 20 l 430 20 b 436 20 445 29 445 35 l 445 115 b 445 121 436 130 430 130 l 35 130 b 29 130 20 121 20 115 l 20 35";

fn prefix(output: &mut impl Write, layer: &str, start: i32, end: i32) -> io::Result<()> {
    write!(output, "\nDialogue:{layer},")?;
    write_time(output, start)?;
    write_time(output, end)
}

fn data(
    output: &mut impl Write,
    start: i32,
    end: i32,
    x: i32,
    y: i32,
    effect: &str,
    value: impl std::fmt::Display,
) -> io::Result<()> {
    prefix(output, "10", start, end)?;
    write!(
        output,
        "{STYLE}{{\\pos({x},{y})\\b1{effect}\\fs25\\1c&HFFFFFF}}{value}"
    )
}

/// `roll_time` and `hold_time` are milliseconds, as in the original core API.
/// Out-of-range histogram bins and negative type indices are safely bounded;
/// the C implementation could corrupt memory for these inputs.
pub fn write_ass_stat_part(
    output: &mut impl Write,
    pool: &[Danmaku],
    mode: i32,
    roll_time: i32,
    hold_time: i32,
    density: i32,
    block_mode: i32,
) -> io::Result<()> {
    let pool: Vec<_> = pool.iter().collect();
    write_ass_stat_refs(
        output, &pool, mode, roll_time, hold_time, density, block_mode,
    )
}

pub(crate) fn write_ass_stat_refs(
    output: &mut impl Write,
    pool: &[&Danmaku],
    mode: i32,
    roll_time: i32,
    hold_time: i32,
    density: i32,
    block_mode: i32,
) -> io::Result<()> {
    if mode == 0 || pool.is_empty() {
        return Ok(());
    }
    let mut total = [0i32; 6];
    let mut last_start = 0;
    let mut last_end = 0;
    if mode & 2 != 0 {
        for item in pool {
            let kind = i32::from(item.type_).unsigned_abs() as usize;
            if kind <= 5 {
                total[kind] += 1;
            }
            last_start = item.time;
            last_end = end_time(item, roll_time, hold_time);
        }
        total[0] = total[1..].iter().sum();
        histogram(
            output,
            pool,
            mode & 1 != 0,
            roll_time,
            hold_time,
            last_start,
            last_end,
        )?;
    }
    if mode & 1 != 0 {
        // Upstream declines the table if any type has no table column.
        if pool
            .iter()
            .any(|item| !item.is_normal() && !item.is_special())
        {
            return Ok(());
        }
        if total[0] == 0 {
            for item in pool {
                total[i32::from(item.type_).unsigned_abs() as usize] += 1;
                last_start = item.time;
            }
            total[0] = total[1..].iter().sum();
        }
        table(
            output, pool, roll_time, hold_time, density, block_mode, &total, last_start,
        )?;
    }
    if mode & 3 != 0 {
        closing_notice(output, mode, last_start)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn histogram(
    output: &mut impl Write,
    pool: &[&Danmaku],
    with_table: bool,
    roll: i32,
    hold: i32,
    last_start: i32,
    last_end: i32,
) -> io::Result<()> {
    let shift = if with_table { 0 } else { -255 };
    let clip = if with_table {
        CLIP_TABLE
    } else {
        CLIP_HISTOGRAM
    };
    let end = last_start.saturating_add(30_000);
    prefix(output, " 5", 0, end)?;
    write!(output, "{STYLE}{{\\an7\\1c&HEAF3CE\\p1}}{clip}{{\\p0}}")?;
    prefix(output, " 6", 0, last_end)?;
    let y = 275 + shift;
    write!(
        output,
        "{STYLE}{{\\an7\\move(20, {y}, 444, {y})\\clip({clip})\\1c&HCECEF3\\p1}}m 0 0 l -425 0 l -425 110 l 0 110 l 0 0{{\\p0}}"
    )?;
    prefix(output, if with_table { " 6" } else { "6" }, last_end, end)?;
    write!(
        output,
        "{STYLE}{{\\an7\\pos(445, {y})\\clip({clip})\\1c&HCECEF3\\p1}}m 0 0 l -425 0 l -425 110 l 0 110 l 0 0{{\\p0}}"
    )?;
    let mut all = [0i32; 212];
    let mut blocked = [0i32; 212];
    let mut draw_blocked = false;
    if last_end > 0 {
        for item in pool {
            let start =
                (211i64 * i64::from(item.time) / i64::from(last_end)).clamp(0, 212) as usize;
            let end = (211i64 * i64::from(end_time(item, roll, hold)) / i64::from(last_end))
                .clamp(0, 212) as usize;
            for bin in start..end {
                all[bin] += 1;
                if item.type_ < 0 {
                    blocked[bin] += 1;
                    draw_blocked = true;
                }
            }
        }
    }
    let maximum = all.iter().copied().max().unwrap_or(0).max(1);
    for (bins, layer, color) in [(&all, "7", "FFFFFF"), (&blocked, " 8", "D3D3D3")] {
        if layer == " 8" && !draw_blocked {
            continue;
        }
        prefix(output, layer, 0, end)?;
        write!(
            output,
            "{STYLE}{{\\an7\\clip({clip})\\1c&H{color}\\p1}}m 20 {}",
            385 + shift
        )?;
        let height = |value: i32| 385 - (value as f32 / maximum as f32 * 110.0) as i32 + shift;
        let mut x = 21;
        for bin in 0..212 {
            if bin == 1 {
                write!(output, " l {x} {}", height(bins[bin]))?;
                x += 2;
            }
            write!(output, " l {x} {}", height(bins[bin]))?;
            if bin == 211 {
                write!(output, " l {x} {}", 385 + shift)?;
            } else {
                write!(output, " l {x} {}", height(bins[bin + 1]))?;
                x += 2;
            }
        }
        // The original closes at y=385 even for the shifted histogram.
        output.write_all(b" l 20 385{\\p0}")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn table(
    output: &mut impl Write,
    pool: &[&Danmaku],
    roll: i32,
    hold: i32,
    density: i32,
    block_mode: i32,
    total: &[i32; 6],
    last_start: i32,
) -> io::Result<()> {
    let end = last_start.saturating_add(30_000);
    output.write_all(b"\n")?;
    prefix(output, "3", 0, end)?;
    write!(
        output,
        "{STYLE}{{\\an7\\1c&HCECEF3\\p1}}m 20 35 b 20 26 26 20 35 20 l 430 20 b 439 20 445 29 445 35 l 445 50 l 20 50 l 20 35 m 20 80 l 445 80 l 445 110 l 20 110 l 20 110 l 20 80 m 20 140 l 445 140 l 445 170 l 20 170 l 20 170 l 20 140 m 20 200 l 445 200 l 445 230 l 20 230 l 20 230 l 20 200{{\\p0}}"
    )?;
    prefix(output, "4", 0, end)?;
    write!(
        output,
        "{STYLE}{{\\an7\\1c&HEAF3CE\\p1}}m 20 50 l 445 50 l 445 80 l 20 80 l 20 80 l 20 50 m 20 110 l 445 110 l 445 140 l 20 140 l 20 140 l 20 110 m 20 170 l 445 170 l 445 200 l 20 200 l 20 200 l 20 170 m 20 230 l 445 230 l 445 230 l 445 245 b 445 251 436 260 430 260 l 35 260 b 29 260 20 251 20 245 l 20 230{{\\p0}}"
    )?;
    for (x, label) in [
        (62, "type"),
        (147, "screen"),
        (232, "blockCnt"),
        (317, "count"),
        (402, "total"),
    ] {
        data(output, 0, end, x, 35, "", label)?;
    }
    prefix(output, "10", 0, end)?;
    write!(
        output,
        "{STYLE}{{\\an4\\pos(35,245)\\b1\\fs25\\1c&HFFFFFF}}DanmakuFactory by hkm"
    )?;
    for (index, label, bits) in [
        (0, "ALL", 31),
        (1, "R to L", 1),
        (2, "L to R", 2),
        (3, "TOP", 4),
        (4, "BOTTOM", 8),
        (5, "SPCIAL", 16),
    ] {
        data(
            output,
            0,
            end,
            62,
            65 + index * 30,
            if block_mode & bits == bits {
                "\\s1"
            } else {
                ""
            },
            label,
        )?;
    }
    for kind in 0..6 {
        data(output, 0, end, 402, 65 + kind as i32 * 30, "", total[kind])?;
        if total[kind] == 0 {
            for x in [147, 317, 232] {
                data(output, 0, end, x, 65 + kind as i32 * 30, "", 0)?;
            }
        }
    }
    if pool[0].time > 0 {
        for column in 0..3 {
            for kind in 0..6 {
                data(
                    output,
                    0,
                    pool[0].time,
                    147 + column * 85,
                    65 + kind * 30,
                    "",
                    0,
                )?;
            }
        }
    }
    output.write_all(b"\n")?;
    let mut first = 0;
    let mut count = [0i32; 6];
    let mut block = [0i32; 6];
    for (index, item) in pool.iter().enumerate() {
        while first < index && end_time(&pool[first], roll, hold) < item.time {
            first += 1;
        }
        let mut screen = [0i32; 6];
        for scan in &pool[first..=index] {
            if scan.type_ > 0 && end_time(scan, roll, hold) > item.time {
                screen[scan.type_ as usize] += 1;
            }
        }
        if item.type_ > 0 {
            count[item.type_ as usize] += 1;
        } else {
            block[-item.type_ as usize] += 1;
        }
        let limit = pool
            .get(index + 1)
            .map_or(item.time.saturating_add(30_000), |next| next.time);
        if item.time == limit {
            continue;
        }
        count[0] = count[1..].iter().sum();
        block[0] = block[1..].iter().sum();
        for kind in 0..6 {
            if total[kind] != 0 {
                data(
                    output,
                    item.time,
                    limit,
                    232,
                    65 + kind as i32 * 30,
                    "",
                    block[kind],
                )?;
                data(
                    output,
                    item.time,
                    limit,
                    317,
                    65 + kind as i32 * 30,
                    "",
                    count[kind],
                )?;
            }
        }
        let mut start = item.time;
        loop {
            let next_end = pool[first..=index]
                .iter()
                .map(|scan| end_time(scan, roll, hold))
                .filter(|&time| time > start && time < limit)
                .min()
                .unwrap_or(limit);
            if next_end == start {
                break;
            }
            screen[0] = screen[1..].iter().sum();
            if density > 0 {
                data(
                    output,
                    start,
                    next_end,
                    147,
                    65,
                    "",
                    format!(
                        "{}{}{}/{}",
                        screen[0],
                        if screen[0] > density { "^" } else { "" },
                        "",
                        density
                    ),
                )?;
            } else {
                data(output, start, next_end, 147, 65, "", screen[0])?;
            }
            for kind in 1..6 {
                if total[kind] != 0 {
                    data(
                        output,
                        start,
                        next_end,
                        147,
                        65 + kind as i32 * 30,
                        "",
                        screen[kind],
                    )?;
                }
            }
            for scan in &pool[first..=index] {
                if scan.type_ > 0 && end_time(scan, roll, hold) == next_end {
                    screen[scan.type_ as usize] -= 1;
                }
            }
            start = next_end;
        }
    }
    Ok(())
}

fn closing_notice(output: &mut impl Write, mode: i32, start: i32) -> io::Result<()> {
    let both = mode & 3 == 3;
    let table = mode & 1 != 0;
    let shift = if both {
        0
    } else if table {
        -125
    } else {
        -255
    };
    let end = start.saturating_add(30_000);
    prefix(output, " 9", start, start.saturating_add(200))?;
    let (style, entry) = if both {
        (
            STYLE,
            "\\move(20,377,20,423)\\iclip(m 20 340 l 445 340 l 445 385 l 20 385 l 20 340)",
        )
    } else if table {
        (
            STYLE,
            "\\move(20,252,20,288)\\iclip(m 20 245 l 445 470 l 445 515 l 20 515 l 20 470)",
        )
    } else {
        (
            "Default,,0000,0000,0000,,",
            "\\move(20,122,20,168)\\iclip(m 20 85 l 445 85 l 445 130 l 20 130 l 20 85)",
        )
    };
    write!(
        output,
        "{style}{{\\an4{entry}\\1c&HEAF3CE\\p1}}m 15 0 l 410 0 b 416 0 425 9 425 15 l 425 30 b 425 36 416 45 410 45 l 15 45 b 9 45 0 37 0 30 l 0 15 b 0 9 9 0 15 0{{\\p0}}"
    )?;
    let popup = if both {
        "m 35 400 l 430 400 b 436 400 445 409 445 415 l 445 430 b 445 436 436 445 430 445 l 35 445 b 29 445 20 437 20 430 l 20 415 b 20 409 29 400 35 400"
    } else if table {
        "m 35 275 l 430 275 b 436 275 445 284 445 290 l 445 305 b 445 311 436 320 430 320 l 35 320 b 29 320 20 312 20 305 l 20 290 b 20 280 29 275 35 275"
    } else {
        "m 35 145 l 430 145 b 436 145 445 154 445 160 l 445 175 b 445 181 436 190 430 190 l 35 190 b 29 190 20 182 20 175 l 20 160 b 20 154 29 145 35 145"
    };
    prefix(output, " 9", start.saturating_add(200), end)?;
    write!(output, "{STYLE}{{\\an7\\1c&HEAF3CE\\p1}}{popup}{{\\p0}}")?;
    prefix(output, " 10", start.saturating_add(200), end)?;
    let y = 400 + shift;
    write!(
        output,
        "{STYLE}{{\\an7\\move(20, {y}, 445, {y})\\clip({popup})\\1c&HCECEF3\\p1}}m 0 0 l -425 0 l -425 45 l 0 45 l 0 0{{\\p0}}"
    )?;
    for second in 0..30 {
        prefix(
            output,
            " 11",
            start.saturating_add(second * 1000),
            start.saturating_add((second + 1) * 1000),
        )?;
        write!(
            output,
            "{STYLE}{{\\an4\\pos(35,{})\\b1\\fs25\\1c&HFFFFFF}}These charts will close after {} s",
            422 + shift,
            30 - second
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{R2L, SPECIAL, SpecialPart};

    #[test]
    fn negative_types_and_long_lived_special_do_not_index_outside_histogram() {
        let pool = [
            Danmaku {
                time: 1000,
                type_: -SPECIAL,
                special: Some(Box::new(SpecialPart {
                    exist_time: 50_000,
                    ..Default::default()
                })),
                ..Default::default()
            },
            Danmaku {
                time: 2000,
                type_: R2L,
                ..Default::default()
            },
        ];
        let mut output = Vec::new();
        write_ass_stat_part(&mut output, &pool, 3, 12, 5, 2, 0).unwrap();
        assert!(
            output
                .windows(b"These charts will close after 1 s".len())
                .any(|part| part == b"These charts will close after 1 s")
        );
    }

    #[test]
    fn zero_duration_histogram_is_defined() {
        let mut output = Vec::new();
        write_ass_stat_part(
            &mut output,
            &[Danmaku {
                type_: R2L,
                ..Default::default()
            }],
            2,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        assert!(output.starts_with(b"\nDialogue: 5,"));
    }
}
