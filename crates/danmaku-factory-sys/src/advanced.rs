//! Message boxes, gift combinations and their enter/leave/scroll animation.

use std::io::{self, Write};

use crate::ass::write_time;
use crate::model::{Config, Danmaku, MSG_GIFT, MSG_GUARD, MSG_SUPER_CHAT, rounded_milliseconds};

const ANIMATION: i32 = 250;
const LAST_TIME: i32 = 2_147_483_640;

fn same_gift(left: &Danmaku, right: &Danmaku) -> bool {
    left.is_type(MSG_GIFT)
        && right.is_type(MSG_GIFT)
        && matches!((&left.user, &right.user, &left.gift, &right.gift), (Some(a), Some(b), Some(c), Some(d)) if a.uid == b.uid && a.name == b.name && c.name == d.name)
}

/// Keeps the slice-owned pool intact while representing C's node removals as
/// active indices. Removed gifts no longer contribute to blocked statistics.
pub(crate) fn merge_simultaneous_gifts(pool: &mut [Danmaku]) -> Vec<usize> {
    let mut active = vec![true; pool.len()];
    for current in 0..pool.len() {
        if !active[current] || !pool[current].is_type(MSG_GIFT) || pool[current].gift.is_none() {
            continue;
        }
        for next in current + 1..pool.len() {
            if pool[next].time != pool[current].time {
                break;
            }
            if !active[next] || !same_gift(&pool[current], &pool[next]) {
                continue;
            }
            let gift = pool[next].gift.as_ref().unwrap().clone();
            let current_gift = pool[current].gift.as_mut().unwrap();
            current_gift.price = current_gift.price.saturating_add(gift.price);
            current_gift.count = current_gift.count.saturating_add(gift.count);
            current_gift.duration = current_gift.duration.max(gift.duration);
            active[next] = false;
            pool[next].type_ = 0;
        }
    }
    active
        .into_iter()
        .enumerate()
        .filter_map(|(index, present)| present.then_some(index))
        .collect()
}

#[derive(Clone, Debug)]
struct Animation {
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    start_time: i32,
    end_time: i32,
    clipped: bool,
}

impl Animation {
    fn new(start: (i32, i32), end: (i32, i32), start_time: i32, end_time: i32) -> Self {
        Self {
            start_x: start.0,
            start_y: start.1,
            end_x: end.0,
            end_y: end.1,
            start_time,
            end_time,
            clipped: false,
        }
    }

    fn duration(&self) -> i32 {
        self.end_time.saturating_sub(self.start_time)
    }
    fn stationary(&self) -> bool {
        self.start_y == self.end_y
    }
}

#[derive(Debug)]
struct Message {
    combos: Vec<usize>,
    animations: Vec<Animation>,
    shown: bool,
    up_moved: bool,
    height: i32,
    y: i32,
}

impl Message {
    fn append(&mut self, start: (i32, i32), end: (i32, i32), from: i32, until: i32) {
        if from >= until {
            return;
        }
        if let Some(last) = self.animations.last_mut() {
            if (last.start_x, last.start_y, last.end_x, last.end_y)
                == (start.0, start.1, end.0, end.1)
            {
                last.end_time = until;
                return;
            }
        }
        self.animations
            .push(Animation::new(start, end, from, until));
    }

    fn append_end(&mut self, config: &Config, msg_start: i32, start: i32, end: i32, up: i32) {
        let x = config.msg_box_pos_x;
        if end <= msg_start || self.up_moved {
            self.append((x, self.y), (x, self.y), start, end);
        } else {
            self.append((x, self.y), (x, self.y), start, msg_start);
            self.y = self.y.saturating_sub(up);
            self.up_moved = true;
            self.append((x, self.y), (x, self.y), msg_start, end);
        }
    }
}

/// Stable arena IDs avoid raw linked pointers while preserving list order.
pub(crate) struct MessageQueue {
    arena: Vec<Option<Message>>,
    free: Vec<usize>,
    order: Vec<usize>,
    last_end: i32,
    clip: String,
}

impl MessageQueue {
    pub(crate) fn new(config: &Config) -> Self {
        let r = i64::from(config.msgbox_font_size / 2);
        let (x, y, width, height) = (
            i64::from(config.msg_box_pos_x),
            i64::from(config.msg_box_pos_y),
            i64::from(config.msg_box_size_x),
            i64::from(config.msg_box_size_y),
        );
        let clip = format!(
            "\\clip(m {x} {} b {x} {} {} {y} {} {y} l {} {y} b {} {y} {} {} {} {} l {} {} l {x} {})",
            y + r,
            y + r / 2,
            x + r / 2,
            x + r,
            x + width - r,
            x + width - r / 2,
            x + width,
            y + r / 2,
            x + width,
            y + r,
            x + width,
            y + height,
            y + height
        );
        Self {
            arena: Vec::new(),
            free: Vec::new(),
            order: Vec::new(),
            last_end: -ANIMATION,
            clip,
        }
    }

    pub(crate) fn push(
        &mut self,
        output: &mut impl Write,
        pool: &mut [Danmaku],
        index: usize,
        config: &Config,
    ) -> io::Result<()> {
        let duration = crate::model::ass_milliseconds(config.msgbox_duration);
        let gift = pool[index].gift.as_mut().expect("validated message gift");
        if gift.price >= 0 && gift.price < (config.gift_min_price * 1000.0) as i32 {
            return Ok(());
        }
        gift.duration = if duration >= 10 {
            duration
        } else {
            rounded_milliseconds(gift.duration)
        };
        let first_time = self.order.first().map_or(pool[index].time, |id| {
            pool[self.arena[*id].as_ref().unwrap().combos[0]].time
        });
        let id = self.free.pop().unwrap_or_else(|| {
            self.arena.push(None);
            self.arena.len() - 1
        });
        self.arena[id] = Some(Message {
            combos: vec![index],
            animations: Vec::new(),
            shown: false,
            up_moved: false,
            height: message_height(&pool[index], config.msgbox_font_size, config.msg_box_size_x)
                .saturating_add(config.msgbox_font_size / 6),
            y: 0,
        });
        self.order.insert(0, id);
        if pool[index].time <= first_time {
            return Ok(());
        }
        let end = pool[index].time.min(first_time.saturating_add(ANIMATION));
        self.alive(output, pool, config, 1, self.last_end, first_time, end)?;
        self.last_end = end;
        Ok(())
    }

    pub(crate) fn finish(
        &mut self,
        output: &mut impl Write,
        pool: &mut [Danmaku],
        config: &Config,
    ) -> io::Result<()> {
        if let Some(&id) = self.order.first() {
            let start = pool[self.arena[id].as_ref().unwrap().combos[0]].time;
            let end = start.saturating_add(ANIMATION);
            self.alive(output, pool, config, 0, self.last_end, start, end)?;
            self.alive(output, pool, config, 0, end, LAST_TIME, LAST_TIME)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn alive(
        &mut self,
        output: &mut impl Write,
        pool: &mut [Danmaku],
        config: &Config,
        exclude: usize,
        mut last_end: i32,
        start: i32,
        end: i32,
    ) -> io::Result<()> {
        let selected: Vec<_> = self.order.iter().skip(exclude).copied().collect();
        // New gifts may extend a previously displayed gift, keeping its box.
        for &id in &selected {
            let Some(message) = self.arena[id].as_ref() else {
                continue;
            };
            if message.shown {
                break;
            }
            let current = message.combos[0];
            if !pool[current].is_type(MSG_GIFT) {
                continue;
            }
            let previous_id = self
                .order
                .iter()
                .rev()
                .copied()
                .take_while(|old| {
                    self.arena[*old]
                        .as_ref()
                        .is_some_and(|message| message.shown)
                })
                .find(|old| {
                    let previous = *self.arena[*old].as_ref().unwrap().combos.last().unwrap();
                    same_gift(&pool[previous], &pool[current])
                        && pool[current].time.saturating_sub(pool[previous].time)
                            <= pool[previous].gift.as_ref().unwrap().duration
                });
            if let Some(previous_id) = previous_id {
                let previous = *self.arena[previous_id]
                    .as_ref()
                    .unwrap()
                    .combos
                    .last()
                    .unwrap();
                let previous_gift = pool[previous].gift.as_ref().unwrap().clone();
                let current_time = pool[current].time;
                let previous_time = pool[previous].time;
                let gift = pool[current].gift.as_mut().unwrap();
                gift.price = gift.price.saturating_add(previous_gift.price);
                gift.count = gift.count.saturating_add(previous_gift.count);
                if current_time.saturating_add(gift.duration)
                    < previous_time.saturating_add(previous_gift.duration)
                {
                    gift.duration = previous_time
                        .saturating_sub(current_time)
                        .saturating_add(previous_gift.duration);
                }
                pool[previous].gift.as_mut().unwrap().duration =
                    current_time.saturating_sub(previous_time);
                self.arena[previous_id]
                    .as_mut()
                    .unwrap()
                    .combos
                    .push(current);
                self.arena[id] = None;
                self.free.push(id);
                self.order.retain(|other| *other != id);
            }
        }
        let total_up: i32 = selected
            .iter()
            .filter_map(|id| self.arena[*id].as_ref())
            .take_while(|message| !message.shown)
            .map(|message| message.height)
            .fold(0i32, i32::saturating_add);
        let mut endings: Vec<_> = self
            .order
            .iter()
            .rev()
            .copied()
            .take_while(|id| {
                self.arena[*id]
                    .as_ref()
                    .is_some_and(|message| message.shown)
            })
            .map(|id| {
                let index = *self.arena[id].as_ref().unwrap().combos.last().unwrap();
                (
                    pool[index]
                        .time
                        .saturating_add(pool[index].gift.as_ref().unwrap().duration),
                    id,
                )
            })
            .collect();
        endings.sort_by_key(|&(time, _)| time);
        let mut position = 0;
        while position < endings.len() {
            let (expiration, id) = endings[position];
            if expiration > end {
                if let Some(message) = self.arena[id].as_mut() {
                    message.append_end(config, start, last_end, end, total_up);
                    message.up_moved = false;
                }
                position += 1;
                continue;
            }
            let mut removed = Vec::new();
            while position < endings.len() && endings[position].0 == expiration {
                let id = endings[position].1;
                if let Some(mut message) = self.arena[id].take() {
                    self.free.push(id);
                    message.append_end(config, start, last_end, expiration, total_up);
                    removed.push((message.y, message.height));
                    self.order.retain(|other| *other != id);
                    amend(&mut message, config);
                    print_message_history(output, pool, &message, config, &self.clip)?;
                }
                position += 1;
            }
            // Boxes above disappearing boxes move down to fill their space.
            for &id in &self.order {
                let message = self.arena[id].as_mut().unwrap();
                if !message.shown {
                    continue;
                }
                message.append_end(config, start, last_end, expiration, total_up);
                let shift: i32 = removed
                    .iter()
                    .filter(|&&(y, _)| y > message.y)
                    .map(|&(_, height)| height)
                    .fold(0i32, i32::saturating_add);
                message.y = message.y.saturating_add(shift);
            }
            last_end = expiration;
        }
        let mut height = config.msg_box_pos_y.saturating_add(config.msg_box_size_y);
        for id in selected {
            let Some(message) = self.arena[id].as_mut() else {
                continue;
            };
            if message.shown {
                break;
            }
            message.shown = true;
            height = height.saturating_sub(message.height);
            message.y = height;
            let x = config.msg_box_pos_x;
            message.append((x, message.y), (x, message.y), start, end);
        }
        Ok(())
    }
}

fn amend(message: &mut Message, config: &Config) {
    let segments = &mut message.animations;
    if segments.is_empty() {
        return;
    }
    let mut index = 0;
    while index + 1 < segments.len() {
        if segments[index].end_y < segments[index + 1].start_y {
            if segments[index + 1].duration() <= ANIMATION {
                segments[index + 1].start_x = segments[index].end_x;
                segments[index + 1].start_y = segments[index].end_y;
            } else {
                segments[index + 1].start_time += ANIMATION;
                let bridge = Animation::new(
                    (segments[index].end_x, segments[index].end_y),
                    (segments[index + 1].start_x, segments[index + 1].start_y),
                    segments[index].end_time,
                    segments[index + 1].start_time,
                );
                segments.insert(index + 1, bridge);
                index += 1;
            }
        } else if segments[index].end_y > segments[index + 1].start_y {
            if segments[index].duration() <= ANIMATION {
                segments[index].end_x = segments[index + 1].start_x;
                segments[index].end_y = segments[index + 1].start_y;
            } else {
                segments[index].end_time -= ANIMATION;
                let bridge = Animation::new(
                    (segments[index].end_x, segments[index].end_y),
                    (segments[index + 1].start_x, segments[index + 1].start_y),
                    segments[index].end_time,
                    segments[index + 1].start_time,
                );
                segments.insert(index + 1, bridge);
                index += 1;
            }
        }
        index += 1;
    }
    let mut index = 1;
    while index < segments.len() {
        if segments[index].start_y.saturating_add(message.height) < config.msg_box_pos_y
            && segments[index].end_y.saturating_add(message.height) < config.msg_box_pos_y
        {
            segments.remove(index);
        } else {
            index += 1;
        }
    }
    loop {
        let mut merged = false;
        let mut index = 0;
        while index + 1 < segments.len() {
            let (a, b) = (&segments[index], &segments[index + 1]);
            if (a.duration() < ANIMATION || b.duration() < ANIMATION)
                && !a.stationary()
                && !b.stationary()
                && a.end_time == b.start_time
            {
                let b = segments.remove(index + 1);
                segments[index].end_x = b.end_x;
                segments[index].end_y = b.end_y;
                segments[index].end_time = b.end_time;
                merged = true;
            } else {
                index += 1;
            }
        }
        let mut index = 0;
        while index + 1 < segments.len() {
            let (a, b) = (&segments[index], &segments[index + 1]);
            if (a.start_x, a.start_y, a.end_x, a.end_y) == (b.start_x, b.start_y, b.end_x, b.end_y)
                && a.end_time == b.start_time
            {
                let b = segments.remove(index + 1);
                segments[index].end_time = b.end_time;
                merged = true;
            } else {
                index += 1;
            }
        }
        let mut index = 0;
        while index + 1 < segments.len() {
            let (a, b) = (&segments[index], &segments[index + 1]);
            if a.duration() < ANIMATION
                && a.start_y < a.end_y
                && b.stationary()
                && a.end_time == b.start_time
            {
                merged = true;
                if a.duration().saturating_add(b.duration()) <= ANIMATION {
                    let b = segments.remove(index + 1);
                    segments[index].end_time = b.end_time;
                } else {
                    segments[index].end_time = segments[index].start_time + ANIMATION;
                    segments[index + 1].start_time = segments[index].end_time;
                }
            } else if b.duration() < ANIMATION
                && a.stationary()
                && b.start_y > b.end_y
                && a.end_time == b.start_time
            {
                merged = true;
                if a.duration().saturating_add(b.duration()) <= ANIMATION {
                    let b = segments.remove(index + 1);
                    segments[index].end_x = b.end_x;
                    segments[index].end_y = b.end_y;
                    segments[index].end_time = b.end_time;
                } else {
                    segments[index].end_time = segments[index + 1].end_time - ANIMATION;
                    segments[index + 1].start_time = segments[index].end_time;
                    index += 1;
                }
            }
            index += 1;
        }
        if !merged {
            break;
        }
    }
    for segment in segments.iter_mut() {
        segment.clipped =
            segment.start_y < config.msg_box_pos_y || segment.end_y < config.msg_box_pos_y;
    }
    let (mut start_x, mut start_y, mut end_x) = (
        segments[0].start_x,
        segments[0].start_y,
        segments.last().unwrap().end_x,
    );
    if config.msg_box_pos_x.saturating_add(config.msg_box_size_x) < config.resolution_x / 2 {
        start_x = config.msg_box_pos_x.saturating_sub(config.msg_box_size_x);
        end_x = start_x;
    } else if config.msg_box_pos_x > config.resolution_x / 2 {
        start_x = config.msg_box_pos_x.saturating_add(config.msg_box_size_x);
        end_x = start_x;
    } else if config.msg_box_pos_y.saturating_add(config.msg_box_size_y) < config.resolution_y / 2 {
        start_x = config.msg_box_pos_x.saturating_sub(config.msg_box_size_x);
        end_x = start_x;
    } else {
        start_y = config.msg_box_pos_y.saturating_add(config.msg_box_size_y);
    }
    if segments[0].stationary() {
        if segments[0].duration() < ANIMATION {
            segments[0].start_x = start_x;
            segments[0].start_y = start_y;
        } else {
            segments[0].start_time += ANIMATION;
            segments.insert(
                0,
                Animation::new(
                    (start_x, start_y),
                    (segments[0].end_x, segments[0].end_y),
                    segments[0].start_time - ANIMATION,
                    segments[0].start_time,
                ),
            );
        }
    } else {
        segments[0].start_y = config.msg_box_pos_y.saturating_add(config.msg_box_size_y);
    }
    segments[0].clipped = true;
    let last = segments.len() - 1;
    if segments[last].stationary() {
        if segments[last].duration() < ANIMATION {
            segments[last].end_x = end_x;
        } else {
            segments[last].end_time -= ANIMATION;
            segments.push(Animation::new(
                (segments[last].start_x, segments[last].start_y),
                (end_x, segments[last].end_y),
                segments[last].end_time,
                segments[last].end_time + ANIMATION,
            ));
        }
        segments.last_mut().unwrap().clipped = true;
    }
}

fn print_message_history(
    output: &mut impl Write,
    pool: &[Danmaku],
    message: &Message,
    config: &Config,
    clip: &str,
) -> io::Result<()> {
    let mut combo = 0;
    for segment in &message.animations {
        let (mut x, mut y, mut start) = (segment.start_x, segment.start_y, segment.start_time);
        while combo < message.combos.len()
            && pool[message.combos[combo]]
                .time
                .saturating_add(pool[message.combos[combo]].gift.as_ref().unwrap().duration)
                <= start
        {
            combo += 1;
        }
        let effect = if segment.clipped { clip } else { "" };
        while combo < message.combos.len() {
            let item = &pool[message.combos[combo]];
            let expires = item
                .time
                .saturating_add(item.gift.as_ref().unwrap().duration);
            if expires >= segment.end_time {
                print_message(
                    output,
                    (x, y),
                    (segment.end_x, segment.end_y),
                    start,
                    segment.end_time,
                    config.msg_box_size_x,
                    config.msgbox_font_size,
                    effect,
                    item,
                )?;
                break;
            }
            let ratio = expires.saturating_sub(start) as f32 / segment.duration() as f32;
            let next_x = x.saturating_add(
                (segment.end_x.saturating_sub(segment.start_x) as f32 * ratio) as i32,
            );
            let next_y = y.saturating_add(
                (segment.end_y.saturating_sub(segment.start_y) as f32 * ratio) as i32,
            );
            print_message(
                output,
                (x, y),
                (next_x, next_y),
                start,
                expires,
                config.msg_box_size_x,
                config.msgbox_font_size,
                effect,
                item,
            )?;
            x = next_x;
            y = next_y;
            start = expires;
            combo += 1;
        }
    }
    Ok(())
}

fn wrap_super_chat(text: &[u8], font: i32, width: i32) -> (Vec<u8>, i32) {
    let mut result = Vec::with_capacity(1024);
    let (mut lines, mut count) = (1, 0);
    for &byte in text.iter().take_while(|&&byte| byte != 0) {
        if byte >= 0xC0 || byte < 0x80 {
            count += 1;
        }
        if f64::from(count) * f64::from(font) / 1.2 + f64::from(font / 2) / 2.0 > f64::from(width) {
            if result.len() + 2 >= 1024 {
                break;
            }
            result.extend_from_slice(b"\\N");
            lines += 1;
            count = 0;
        }
        if result.len() >= 1023 {
            break;
        }
        result.push(byte);
    }
    (result, lines)
}

pub(crate) fn message_height(message: &Danmaku, font: i32, width: i32) -> i32 {
    let radius = font / 2;
    match message.type_ {
        MSG_GIFT => font,
        MSG_GUARD => (f64::from(font) + f64::from(font) * 0.8 + f64::from(radius)) as i32,
        MSG_SUPER_CHAT => {
            let mut lines = 1i32;
            let mut count = 0i32;
            for &byte in message.text.iter().take_while(|&&byte| byte != 0) {
                if byte >= 0xC0 || byte < 0x80 {
                    count = count.saturating_add(1);
                }
                if f64::from(count) * f64::from(font) / 1.2 + f64::from(radius) / 2.0
                    > f64::from(width)
                {
                    lines = lines.saturating_add(1);
                    count = 0;
                }
            }
            ((f64::from(font) + f64::from(font) * 0.8 + f64::from(radius) / 2.0) as i32)
                .saturating_add(lines.saturating_mul(font))
                .saturating_add(radius / 2)
        }
        _ => -1,
    }
}

fn action(start: (i32, i32), end: (i32, i32), shift: (i32, i32)) -> String {
    if start == end {
        format!(
            "\\pos({},{})",
            start.0.saturating_add(shift.0),
            start.1.saturating_add(shift.1)
        )
    } else {
        format!(
            "\\move({},{},{},{})",
            start.0.saturating_add(shift.0),
            start.1.saturating_add(shift.1),
            end.0.saturating_add(shift.0),
            end.1.saturating_add(shift.1)
        )
    }
}

fn message_prefix(output: &mut impl Write, layer: i32, start: i32, end: i32) -> io::Result<()> {
    write!(output, "\nDialogue: {layer},")?;
    write_time(output, start)?;
    write_time(output, end)?;
    output.write_all(b"MSG,,0000,0000,0000,,")
}

#[allow(clippy::too_many_arguments)]
fn print_message(
    output: &mut impl Write,
    start: (i32, i32),
    end: (i32, i32),
    from: i32,
    until: i32,
    width: i32,
    font: i32,
    effect: &str,
    message: &Danmaku,
) -> io::Result<()> {
    let radius = font / 2;
    let user = message.user.as_ref().expect("validated message user");
    let gift = message.gift.as_ref().expect("validated message gift");
    let move_main = action(start, end, (0, 0));
    match message.type_ {
        MSG_GIFT => {
            message_prefix(output, 0, from, until)?;
            write!(output, "{{{move_main}{effect}}}{{\\c&HBCACF7\\b1}}")?;
            output.write_all(&user.name)?;
            output.write_all(b": {\\c&HFFFFFF\\b0}")?;
            output.write_all(&gift.name)?;
            write!(output, " x{}", gift.count)
        }
        MSG_SUPER_CHAT => {
            let (text, lines) = wrap_super_chat(&message.text, font, width);
            let top_height =
                (f64::from(font) + f64::from(font) * 0.8 + f64::from(radius) / 2.0) as i32;
            let bottom_height = lines.saturating_mul(font).saturating_add(radius / 2);
            let (top_color, bottom_color, user_color) = match gift.price {
                2_000_000.. => ("D8D8FF", "321AAB", "1B0E5E"),
                1_000_000.. => ("E4E7FF", "4D4DE5", "333398"),
                500_000.. => ("D2EAFF", "4394E0", "2C6193"),
                100_000.. => ("C5F1FF", "2BB5E2", "1C7795"),
                50_000.. => ("FDFFDB", "9E7D42", "514022"),
                _ => ("FFF5ED", "B2602A", "653617"),
            };
            message_prefix(output, 0, from, until)?;
            write!(
                output,
                "{{{move_main}{effect}\\c&H{top_color}\\p1\\bord0\\shad0}}m 0 {radius} b 0 {} {} 0 {radius} 0 l {} 0 b {} 0 {width} {} {width} {radius} l {width} {top_height} l 0 {top_height}",
                radius / 2,
                radius / 2,
                width - radius,
                width - radius / 2,
                radius / 2
            )?;
            message_prefix(output, 0, from, until)?;
            write!(
                output,
                "{{{}{effect}\\p1\\c&H{bottom_color}\\bord0\\shad0}}m 0 0 l {width} 0 l {width} {} b {width} {} {} {bottom_height} {} {bottom_height} l {radius} {bottom_height} b {} {bottom_height} 0 {} 0 {}",
                action(start, end, (0, top_height)),
                bottom_height - radius,
                bottom_height - radius / 2,
                width - radius / 2,
                width - radius,
                radius / 2,
                bottom_height - radius / 2,
                bottom_height - radius
            )?;
            message_prefix(output, 1, from, until)?;
            write!(
                output,
                "{{{}{effect}\\c&H{user_color}\\b1\\bord0\\shad0}}",
                action(start, end, (radius / 2, radius / 3))
            )?;
            output.write_all(&user.name)?;
            message_prefix(output, 1, from, until)?;
            write!(
                output,
                "{{{}{effect}\\c&H313131\\fs{}\\bord0\\shad0}}SuperChat CNY {}",
                action(start, end, (radius / 2, font.saturating_add(radius / 3))),
                (f64::from(font) * 0.8) as i32,
                gift.price / 1000
            )?;
            message_prefix(output, 1, from, until)?;
            write!(
                output,
                "{{{}{effect}\\c&HFFFFFF\\bord0\\shad0}}",
                action(start, end, (radius / 2, top_height))
            )?;
            output.write_all(&text)
        }
        MSG_GUARD => {
            let height = (f64::from(font) + f64::from(font) * 0.8 + f64::from(radius)) as i32;
            let (box_color, user_color) = if gift.price >= 19_998_000 {
                ("E5E5FF", "0F0F75")
            } else if gift.price >= 1_998_000 {
                ("CAF9F8", "1A8B87")
            } else {
                ("FCE8D8", "8A3619")
            };
            message_prefix(output, 0, from, until)?;
            write!(
                output,
                "{{{move_main}{effect}\\c&H{box_color}\\p1\\bord0\\shad0}}m 0 {radius} b 0 {} {} 0 {radius} 0 l {} 0 b {} 0 {width} {} {width} {radius} l {width} {} b {width} {} {} {height} {} {height} l {radius} {height} b {} {height} 0 {} 0 {}",
                radius / 2,
                radius / 2,
                width - radius,
                width - radius / 2,
                radius / 2,
                height - radius,
                height - radius / 2,
                width - radius / 2,
                width - radius,
                radius / 2,
                height - radius / 2,
                height - radius
            )?;
            message_prefix(output, 1, from, until)?;
            write!(
                output,
                "{{{}{effect}\\c&H{user_color}\\bord0\\shad0}}",
                action(start, end, (radius / 2, radius / 3))
            )?;
            output.write_all(&user.name)?;
            message_prefix(output, 1, from, until)?;
            write!(
                output,
                "{{{}{effect}\\c&H313131\\fs{}\\bord0\\shad0}}Welcome new ",
                action(start, end, (radius / 2, font.saturating_add(radius / 3))),
                (f64::from(font) * 0.8) as i32
            )?;
            output.write_all(&gift.name)?;
            output.write_all(b"!")
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GiftPart, UserPart};

    fn gift(time: i32, count: i32) -> Danmaku {
        Danmaku {
            type_: MSG_GIFT,
            time,
            user: Some(Box::new(UserPart {
                uid: 42,
                name: b"User".to_vec(),
                ..Default::default()
            })),
            gift: Some(Box::new(GiftPart {
                name: b"Flower".to_vec(),
                price: 1000 * count,
                count,
                duration: 5000,
            })),
            ..Default::default()
        }
    }

    #[test]
    fn simultaneous_and_later_gifts_combine_without_losing_lifetime() {
        let mut pool = [gift(1000, 1), gift(1000, 2), gift(2000, 1)];
        let active = merge_simultaneous_gifts(&mut pool);
        assert_eq!(active, [0, 2]);
        assert_eq!(pool[0].gift.as_ref().unwrap().count, 3);
        let config = Config {
            show_msg_box: true,
            ..Default::default()
        };
        let mut queue = MessageQueue::new(&config);
        let mut output = Vec::new();
        for index in active {
            queue.push(&mut output, &mut pool, index, &config).unwrap();
        }
        queue.finish(&mut output, &mut pool, &config).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Flower x3"));
        assert!(text.contains("Flower x4"));
        assert_eq!(pool[2].gift.as_ref().unwrap().duration, 5000);
        assert!(queue.order.is_empty());
    }

    #[test]
    fn super_chat_wrap_is_bounded_when_long_text_needs_many_newlines() {
        let (text, lines) = wrap_super_chat(&vec![b'a'; 1023], 38, 1);
        assert!(text.len() <= 1023);
        assert!(lines > 1);
    }

    #[test]
    fn expired_messages_do_not_retain_one_arena_slot_per_record() {
        let config = Config {
            show_msg_box: true,
            ..Default::default()
        };
        let mut pool: Vec<_> = (0..1000)
            .map(|index| {
                let mut message = gift(index * 10000, 1);
                message.type_ = MSG_SUPER_CHAT;
                message.text = b"a message".to_vec();
                message
            })
            .collect();
        let mut queue = MessageQueue::new(&config);
        let mut output = io::sink();
        for index in 0..pool.len() {
            queue.push(&mut output, &mut pool, index, &config).unwrap();
        }
        queue.finish(&mut output, &mut pool, &config).unwrap();
        assert!(queue.order.is_empty());
        assert!(
            queue.arena.len() < 10,
            "expired message slots must be reused"
        );
    }

    #[test]
    fn dense_super_chats_do_not_overflow_total_box_height() {
        let config = Config {
            show_msg_box: true,
            msg_box_size_x: 1,
            msgbox_font_size: 4096,
            ..Default::default()
        };
        let mut pool: Vec<_> = (0..1000)
            .map(|_| {
                let mut message = gift(1000, 1);
                message.type_ = MSG_SUPER_CHAT;
                message.text = vec![b'x'; 1023];
                message
            })
            .collect();
        // 1000 boxes exceed the signed 32-bit pixel-height range.
        assert!(
            i64::from(message_height(&pool[0], 4096, 1)) * pool.len() as i64 > i64::from(i32::MAX)
        );
        let mut queue = MessageQueue::new(&config);
        let mut output = io::sink();
        for index in 0..pool.len() {
            queue.push(&mut output, &mut pool, index, &config).unwrap();
        }
        queue.finish(&mut output, &mut pool, &config).unwrap();
        assert!(queue.order.is_empty());
    }
}
