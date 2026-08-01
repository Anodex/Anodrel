#![forbid(unsafe_code)]

//! A deliberately small, strict JSON codec built into Anodrel.
//!
//! The host protocol is JSON, so keeping parsing here avoids making a general
//! serialization framework part of the shipped native runtime. This module
//! rejects duplicate object keys, malformed Unicode escapes, trailing bytes,
//! and excessively deep input.

use std::collections::BTreeMap;

pub const DEFAULT_MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn parse(input: &str) -> Result<Self, JsonError> {
        Parser::new(input).parse()
    }

    pub fn to_json(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output);
        output
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u16(&self) -> Option<u16> {
        let Self::Number(value) = self else {
            return None;
        };
        if value.starts_with('-') || value.contains(['.', 'e', 'E']) {
            return None;
        }
        value.parse().ok()
    }

    fn write_json(&self, output: &mut String) {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => output.push_str(value),
            Self::String(value) => write_string(output, value),
            Self::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    value.write_json(output);
                }
                output.push(']');
            }
            Self::Object(fields) => {
                output.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    write_string(output, key);
                    output.push(':');
                    value.write_json(output);
                }
                output.push('}');
            }
        }
    }
}

fn write_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1F}' => {
                use std::fmt::Write as _;
                let _ = write!(output, "\\u{:04X}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    message: &'static str,
    offset: usize,
}

impl JsonError {
    fn new(message: &'static str, offset: usize) -> Self {
        Self { message, offset }
    }

    pub const fn message(&self) -> &'static str {
        self.message
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for JsonError {}

struct Parser<'input> {
    input: &'input [u8],
    offset: usize,
    max_depth: usize,
}

impl<'input> Parser<'input> {
    fn new(input: &'input str) -> Self {
        Self {
            input: input.as_bytes(),
            offset: 0,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    fn parse(mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        let value = self.parse_value(0)?;
        self.skip_whitespace();
        if self.offset != self.input.len() {
            return Err(self.error("unexpected trailing data"));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > self.max_depth {
            return Err(self.error("JSON nesting exceeds the host limit"));
        }
        match self.peek() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            _ => Err(self.error("expected a JSON value")),
        }
    }

    fn parse_literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.input.get(self.offset..self.offset + literal.len()) == Some(literal) {
            self.offset += literal.len();
            Ok(value)
        } else {
            Err(self.error("invalid JSON literal"))
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.consume(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.try_consume(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            self.skip_whitespace();
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.try_consume(b']') {
                return Ok(JsonValue::Array(values));
            }
            self.consume(b',')?;
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.consume(b'{')?;
        self.skip_whitespace();
        let mut fields = BTreeMap::new();
        if self.try_consume(b'}') {
            return Ok(JsonValue::Object(fields));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.consume(b':')?;
            self.skip_whitespace();
            let value = self.parse_value(depth)?;
            if fields.insert(key, value).is_some() {
                return Err(self.error("duplicate object field"));
            }
            self.skip_whitespace();
            if self.try_consume(b'}') {
                return Ok(JsonValue::Object(fields));
            }
            self.consume(b',')?;
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.consume(b'"')?;
        let mut output = String::new();
        let mut plain_start = self.offset;
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error("unterminated JSON string"));
            };
            match byte {
                b'"' => {
                    self.append_plain(&mut output, plain_start, self.offset)?;
                    self.offset += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.append_plain(&mut output, plain_start, self.offset)?;
                    self.offset += 1;
                    let escaped = self.parse_escape()?;
                    output.push(escaped);
                    plain_start = self.offset;
                }
                0..=0x1F => return Err(self.error("control character in JSON string")),
                _ => self.offset += 1,
            }
        }
    }

    fn append_plain(&self, output: &mut String, start: usize, end: usize) -> Result<(), JsonError> {
        let bytes = self
            .input
            .get(start..end)
            .ok_or_else(|| self.error("invalid string range"))?;
        let text =
            std::str::from_utf8(bytes).map_err(|_| self.error("invalid UTF-8 in JSON string"))?;
        output.push_str(text);
        Ok(())
    }

    fn parse_escape(&mut self) -> Result<char, JsonError> {
        let escaped = self
            .next()
            .ok_or_else(|| self.error("unterminated JSON escape"))?;
        match escaped {
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\u{08}'),
            b'f' => Ok('\u{0C}'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => self.parse_unicode_escape(),
            _ => Err(self.error("invalid JSON escape")),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonError> {
        let first = self.parse_hex_quad()?;
        if (0xD800..=0xDBFF).contains(&first) {
            if !self.try_consume(b'\\') || !self.try_consume(b'u') {
                return Err(self.error("high surrogate without a low surrogate"));
            }
            let second = self.parse_hex_quad()?;
            if !(0xDC00..=0xDFFF).contains(&second) {
                return Err(self.error("invalid low surrogate"));
            }
            let code_point =
                0x1_0000 + (((first - 0xD800) as u32) << 10) + (second - 0xDC00) as u32;
            return char::from_u32(code_point)
                .ok_or_else(|| self.error("invalid Unicode code point"));
        }
        if (0xDC00..=0xDFFF).contains(&first) {
            return Err(self.error("low surrogate without a high surrogate"));
        }
        char::from_u32(first as u32).ok_or_else(|| self.error("invalid Unicode code point"))
    }

    fn parse_hex_quad(&mut self) -> Result<u16, JsonError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self
                .next()
                .ok_or_else(|| self.error("truncated Unicode escape"))?;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error("invalid Unicode escape")),
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.offset;
        self.try_consume(b'-');
        match self.peek() {
            Some(b'0') => self.offset += 1,
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => return Err(self.error("invalid JSON number")),
        }
        if self.try_consume(b'.') {
            let decimal_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if decimal_start == self.offset {
                return Err(self.error("invalid JSON number fraction"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if exponent_start == self.offset {
                return Err(self.error("invalid JSON number exponent"));
            }
        }
        let number = std::str::from_utf8(&self.input[start..self.offset])
            .map_err(|_| self.error("invalid JSON number"))?
            .to_owned();
        Ok(JsonValue::Number(number))
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), JsonError> {
        if self.try_consume(expected) {
            Ok(())
        } else {
            Err(self.error("unexpected JSON token"))
        }
    }

    fn try_consume(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.offset += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.offset).copied()
    }

    fn error(&self, message: &'static str) -> JsonError {
        JsonError::new(message, self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::JsonValue;

    #[test]
    fn parses_nested_values_and_surrogate_pairs() {
        let parsed =
            JsonValue::parse(r#"{"items":[true,null,"\uD83D\uDE80"]}"#).expect("JSON is valid");
        let JsonValue::Object(fields) = parsed else {
            panic!("top-level value is an object");
        };
        let JsonValue::Array(items) = &fields["items"] else {
            panic!("items is an array");
        };
        assert_eq!(items[2].as_string(), Some("\u{1F680}"));
    }

    #[test]
    fn rejects_duplicate_fields_and_trailing_data() {
        assert!(JsonValue::parse(r#"{"one":1,"one":2}"#).is_err());
        assert!(JsonValue::parse("null false").is_err());
    }

    #[test]
    fn escapes_control_characters_when_serializing() {
        assert_eq!(
            JsonValue::String("line\nquote\"".to_owned()).to_json(),
            "\"line\\nquote\\\"\""
        );
    }
}
