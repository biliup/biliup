//! STT (Serialized Text Transfer) codec for Douyu.
//!
//! STT is a simple text-based serialization format used by Douyu:
//! - Key-value pairs: `key@=value/`
//! - Nested structures use recursive parsing
//! - Special characters: `@A` -> `@`, `@S` -> `/`

use std::collections::HashMap;

/// Decode STT format to a nested structure.
pub fn decode(input: &str) -> SttValue {
    if input.contains('/') {
        let items: Vec<&str> = input.split('/').filter(|s| !s.is_empty()).collect();
        let mut dict = HashMap::new();
        let mut list = Vec::new();

        for item in items {
            let decoded = decode(item);
            if let SttValue::Map(map) = decoded {
                dict.extend(map);
            } else {
                list.push(decoded);
            }
        }

        if !list.is_empty() {
            SttValue::List(list)
        } else {
            SttValue::Map(dict)
        }
    } else if input.contains("@=") {
        if let Some((key, value)) = input.split_once("@=") {
            let mut map = HashMap::new();
            let key_decoded = decode_string(key);
            let value_decoded = decode(value);
            map.insert(key_decoded, value_decoded);
            SttValue::Map(map)
        } else {
            SttValue::String(decode_string(input))
        }
    } else {
        SttValue::String(decode_string(input))
    }
}

/// Decode special characters in STT string.
fn decode_string(s: &str) -> String {
    s.replace("@A", "@").replace("@S", "/")
}

/// Encode a string for STT format.
#[allow(dead_code)]
fn encode_string(s: &str) -> String {
    s.replace("@", "@A").replace("/", "@S")
}

/// STT value types.
#[derive(Debug, Clone)]
pub enum SttValue {
    String(String),
    Map(HashMap<String, SttValue>),
    List(Vec<SttValue>),
}

impl SttValue {
    /// Get as string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SttValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get value from map by key.
    pub fn get(&self, key: &str) -> Option<&SttValue> {
        match self {
            SttValue::Map(map) => map.get(key),
            _ => None,
        }
    }

    /// Get string value from map by key.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple() {
        let input = "type@=chatmsg/txt@=hello/";
        let result = decode(input);

        assert_eq!(result.get_str("type"), Some("chatmsg"));
        assert_eq!(result.get_str("txt"), Some("hello"));
    }

    #[test]
    fn test_decode_special_chars() {
        let input = "txt@=hello@Aworld@Stest/";
        let result = decode(input);

        assert_eq!(result.get_str("txt"), Some("hello@world/test"));
    }

    #[test]
    fn test_decode_nested() {
        let input = "type@=chatmsg/nn@=user1/txt@=test message/col@=1/";
        let result = decode(input);

        assert_eq!(result.get_str("type"), Some("chatmsg"));
        assert_eq!(result.get_str("nn"), Some("user1"));
        assert_eq!(result.get_str("txt"), Some("test message"));
        assert_eq!(result.get_str("col"), Some("1"));
    }
}
