//! Type filtering, font normalisation and legacy ordering.

use crate::model::{BOTTOM, Config, Danmaku, L2R, R2L, SPECIAL, TOP};
use crate::{
    DF_BLOCK_BOTTOM, DF_BLOCK_COLOR, DF_BLOCK_L2R, DF_BLOCK_R2L, DF_BLOCK_SPECIAL, DF_BLOCK_TOP,
};
use std::io::Write;
use std::ptr::NonNull;

pub fn block_by_type(danmaku: &mut [Danmaku], mode: i32) {
    for item in danmaku {
        let color = mode & DF_BLOCK_COLOR != 0 && !item.is_special() && item.color != 0xFFFFFF;
        let kind = [
            (DF_BLOCK_R2L, R2L),
            (DF_BLOCK_L2R, L2R),
            (DF_BLOCK_TOP, TOP),
            (DF_BLOCK_BOTTOM, BOTTOM),
            (DF_BLOCK_SPECIAL, SPECIAL),
        ]
        .into_iter()
        .any(|(bit, kind)| mode & bit != 0 && item.is_type(kind));
        if color || kind {
            item.block();
        }
    }
}

/// The original blacklist's three non-regex forms: `<uid>`, `[username]`,
/// and a text substring. Matching is byte based, including C string ends.
pub fn block_by_keywords(danmaku: &mut [Danmaku], keys: &[Vec<u8>]) {
    enum Keyword<'a> {
        Uid(u64),
        Name(&'a [u8]),
        Text(&'a [u8]),
    }
    let keys: Vec<_> = keys
        .iter()
        .map(|key| {
            let key = crate::string::nul_prefix(key);
            if key.len() >= 3 && key[0] == b'<' && key[key.len() - 1] == b'>' {
                Keyword::Uid(crate::string::parse_i32(&key[1..key.len() - 1]) as i64 as u64)
            } else if key.len() >= 3 && key[0] == b'[' && key[key.len() - 1] == b']' {
                Keyword::Name(&key[1..key.len() - 1])
            } else {
                Keyword::Text(key)
            }
        })
        .collect();
    for item in danmaku {
        // Upstream never checks any blacklist form when text is NULL, even
        // when user metadata could otherwise match a UID or username.
        if item.text_is_null {
            continue;
        }
        let text = crate::string::nul_prefix(&item.text);
        let blocked = keys.iter().any(|key| match key {
            Keyword::Uid(uid) => item
                .user
                .as_ref()
                .is_some_and(|user| user.uid != 0 && user.uid == *uid),
            Keyword::Name(name) => item.user.as_ref().is_some_and(|user| {
                let user_name = crate::string::nul_prefix(&user.name);
                !user_name.is_empty() && user_name == *name
            }),
            Keyword::Text(key) => {
                key.is_empty() || text.windows(key.len()).any(|part| part == *key)
            }
        });
        if blocked {
            item.block();
        }
    }
}

struct Regex {
    code: NonNull<pcre2_rust::pcre2_code_8>,
    data: NonNull<pcre2_rust::pcre2_match_data_8>,
}

impl Drop for Regex {
    fn drop(&mut self) {
        // SAFETY: these are independently owned PCRE2 allocations. No match
        // or callback outlives the synchronous filtering call.
        unsafe {
            pcre2_rust::pcre2_match_data_free_8(self.data.as_ptr());
            pcre2_rust::pcre2_code_free_8(self.code.as_ptr());
        }
    }
}

fn regex_error(code: i32) -> Vec<u8> {
    let mut message = [0u8; 256];
    // SAFETY: the error formatter receives the entire writable buffer.
    let length =
        unsafe { pcre2_rust::pcre2_get_error_message_8(code, message.as_mut_ptr(), message.len()) };
    if length >= 0 {
        message[..length as usize].to_vec()
    } else {
        b"unknown regular expression error".to_vec()
    }
}

fn report_regex_error(prefix: &[u8], key: &[u8], suffix: &[u8]) {
    // Preserve the legacy diagnostic's raw bytes, including invalid UTF-8.
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(b"\nERROR\n");
    let _ = stderr.write_all(prefix);
    let _ = stderr.write_all(key);
    let _ = stderr.write_all(b"': ");
    let _ = stderr.write_all(suffix);
    let _ = stderr.write_all(b"\n");
}

/// The complete PCRE2 UTF engine backs the legacy regex blacklist. Invalid
/// patterns are reported and omitted from the compact compiled list, exactly
/// as in the original core; keys after that list retain literal/UID/name rules.
pub fn block_by_regex_keywords(
    danmaku: &mut [Danmaku],
    keys: &[Vec<u8>],
) -> Result<(), crate::model::CoreError> {
    let oom = || {
        crate::model::CoreError::new(
            crate::DF_ERR_OUT_OF_MEMORY,
            0,
            0,
            "out of memory while compiling the regex blacklist",
        )
    };
    let mut patterns = Vec::new();
    patterns.try_reserve(keys.len()).map_err(|_| oom())?;
    for (index, key) in keys.iter().enumerate() {
        let key = crate::string::nul_prefix(key);
        let mut error = 0;
        let mut offset = 0;
        // SAFETY: the pattern slice remains live for compilation; lengths are
        // explicit, and the context is PCRE2's default allocator/configuration.
        let code = unsafe {
            pcre2_rust::pcre2_compile_8(
                key.as_ptr(),
                key.len(),
                pcre2_rust::PCRE2_UTF,
                &mut error,
                &mut offset,
                std::ptr::null_mut(),
            )
        };
        let Some(code) = NonNull::new(code) else {
            let prefix = format!("Failed to compile regex expression {index} '");
            report_regex_error(prefix.as_bytes(), key, &regex_error(error));
            continue;
        };
        // SAFETY: code is an owned successful compilation.
        let data = unsafe {
            pcre2_rust::pcre2_match_data_create_from_pattern_8(code.as_ptr(), std::ptr::null_mut())
        };
        let Some(data) = NonNull::new(data) else {
            // SAFETY: the code has not been transferred to a Regex yet.
            unsafe { pcre2_rust::pcre2_code_free_8(code.as_ptr()) };
            return Err(oom());
        };
        patterns.push(Regex { code, data });
    }
    for item in danmaku {
        if item.text_is_null {
            continue;
        }
        let text = crate::string::nul_prefix(&item.text);
        let mut matched = false;
        for (index, pattern) in patterns.iter_mut().enumerate() {
            // SAFETY: the subject is borrowed for this call, code remains
            // owned, and each pattern has exclusive reusable match data.
            let result = unsafe {
                pcre2_rust::pcre2_match_8(
                    pattern.code.as_ptr(),
                    text.as_ptr(),
                    text.len(),
                    0,
                    0,
                    pattern.data.as_ptr(),
                    std::ptr::null_mut(),
                )
            };
            if result >= 0 {
                matched = true;
                break;
            }
            if result != pcre2_rust::PCRE2_ERROR_NOMATCH {
                report_regex_error(
                    b"Regex match failed for '",
                    crate::string::nul_prefix(&keys[index]),
                    &regex_error(result),
                );
            }
        }
        if matched {
            item.block();
        } else if patterns.len() < keys.len() {
            block_by_keywords(std::slice::from_mut(item), &keys[patterns.len()..]);
        }
    }
    Ok(())
}

pub fn normalize_font_size(danmaku: &mut [Danmaku], config: &Config) {
    // Upstream excludes only positive SPECIAL here. A blocked special follows
    // the normalisation branch, which is observable in its ASS override.
    let normalize = config.font_size_norm
        && danmaku.iter().any(|item| {
            item.type_ != SPECIAL
                && (item.font_size <= 0 || i32::from(item.font_size) >= config.resolution_y)
        });
    if config.font_size_strict || normalize {
        for item in danmaku {
            if item.type_ != SPECIAL {
                item.font_size = 25;
            }
        }
    }
}

pub fn sort_list(danmaku: &mut [Danmaku]) {
    // Preserve the original early exit, including its omitted first pair.
    if danmaku
        .get(1..)
        .is_none_or(|tail| tail.windows(2).all(|pair| pair[0].time <= pair[1].time))
    {
        return;
    }
    // Bucket insertion puts every later equal-time item before earlier ones.
    danmaku.reverse();
    danmaku.sort_by_key(|item| item.time);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(time: i32, text: &[u8]) -> Danmaku {
        Danmaku {
            time,
            text: text.to_vec(),
            type_: R2L,
            color: 0xFFFFFF,
            font_size: 25,
            ..Default::default()
        }
    }

    #[test]
    fn ordering_preserves_legacy_first_pair_and_reverses_sorted_ties() {
        let mut first = vec![item(2, b"a"), item(1, b"b"), item(3, b"c")];
        sort_list(&mut first);
        assert_eq!(
            first.iter().map(|item| item.time).collect::<Vec<_>>(),
            [2, 1, 3]
        );
        let mut unsorted = vec![item(2, b"a"), item(3, b"b"), item(2, b"c"), item(1, b"d")];
        sort_list(&mut unsorted);
        assert_eq!(
            unsorted
                .iter()
                .map(|item| item.text.as_slice())
                .collect::<Vec<_>>(),
            [b"d", b"c", b"a", b"b"]
        );
    }

    #[test]
    fn color_filter_excludes_special_and_font_normalization_preserves_signed_rule() {
        let mut items = vec![
            item(0, b"normal"),
            Danmaku {
                type_: SPECIAL,
                color: 0,
                font_size: 40,
                ..item(1, b"special")
            },
        ];
        block_by_type(&mut items, DF_BLOCK_COLOR);
        assert_eq!(items[1].type_, SPECIAL);
        block_by_type(&mut items, DF_BLOCK_SPECIAL);
        normalize_font_size(
            &mut items,
            &Config {
                font_size_strict: true,
                ..Default::default()
            },
        );
        assert_eq!((items[1].type_, items[1].font_size), (-SPECIAL, 25));
    }

    #[test]
    fn blacklists_match_uid_username_and_literal_but_skip_null_text() {
        let user = crate::model::UserPart {
            uid: 42,
            name: b"test user".to_vec(),
            ..Default::default()
        };
        for key in [b"<42>".as_slice(), b"[test user]", b"message"] {
            let mut items = vec![
                Danmaku {
                    user: Some(Box::new(user.clone())),
                    ..item(0, b"a message")
                },
                Danmaku {
                    user: Some(Box::new(user.clone())),
                    text_is_null: true,
                    ..item(0, b"a message")
                },
            ];
            block_by_keywords(&mut items, &[key.to_vec()]);
            assert_eq!(items[0].type_, -R2L);
            assert_eq!(items[1].type_, R2L);
        }
        let mut items = vec![item(0, b"hello\0hidden")];
        block_by_keywords(&mut items, &[b"hidden".to_vec()]);
        assert_eq!(items[0].type_, R2L);
        block_by_keywords(&mut items, &[Vec::new()]);
        assert_eq!(items[0].type_, -R2L);
    }

    #[test]
    fn regex_blacklist_uses_full_pcre_and_utf_without_implicit_ucp() {
        for (pattern, matching, other) in [
            (br"^(.)\1$".as_slice(), "中中", "中文"),
            (br"(?<=hello )world", "hello world", "other world"),
            (br"^\h$", "\u{00a0}", "x"),
            (br"^\w+$", "ASCII_123", "中文"),
        ] {
            let mut items = vec![item(0, matching.as_bytes()), item(1, other.as_bytes())];
            block_by_regex_keywords(&mut items, &[pattern.to_vec()]).unwrap();
            assert_eq!(items[0].type_, -R2L);
            assert_eq!(items[1].type_, R2L);
        }
    }

    #[test]
    fn regex_blacklist_preserves_invalid_pattern_compaction_and_null_text() {
        let mut items = vec![
            item(0, b"x"),
            item(1, b"prefix ^x$ suffix"),
            item(2, b"other"),
            Danmaku {
                text_is_null: true,
                ..item(3, b"x")
            },
        ];
        block_by_regex_keywords(&mut items, &[b"[".to_vec(), b"^x$".to_vec()]).unwrap();
        assert_eq!(
            items.iter().map(|item| item.type_).collect::<Vec<_>>(),
            [-R2L, -R2L, R2L, R2L]
        );
    }
}
