use std::str::FromStr;

use crate::color::{Color, Color16, Color256, CLEAR_COLOR};
use lazy_static::lazy_static;
use regex::{Regex, Replacer};

lazy_static! {
    static ref COLOR_TAG: Regex = Regex::new(
        r#"(?P<escape>\|\|)|\|(?P<byte>(1?[0-9]{1,2})|(2[0-4][0-9])|(25[0-5]))\||\|#(?P<true>[[:xdigit:]]{6})\||\|(?P<name>[[:alnum:]]+)\||(?P<clear>\|\-\|)"#,
    ).unwrap();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorSupport {
    None,
    Colors16,
    Colors256,
    TrueColor,
}

pub fn colorize(message: &str, color_support: ColorSupport) -> String {
    COLOR_TAG
        .replace_all(message, ColorReplacer::new(color_support))
        .to_string()
}

struct ColorReplacer {
    color_support: ColorSupport,
}

impl ColorReplacer {
    fn new(color_support: ColorSupport) -> Self {
        ColorReplacer { color_support }
    }
}

impl Replacer for ColorReplacer {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        if caps.name("escape").is_some() {
            dst.push('|')
        } else if let Some(m) = caps.name("byte") {
            if let Ok(color) = Color256::from_str(m.as_str()) {
                match self.color_support {
                    ColorSupport::None => (),
                    ColorSupport::Colors16 => {
                        let color = Color16::from(color);
                        dst.push_str(color.to_string().as_str());
                    }
                    ColorSupport::Colors256 | ColorSupport::TrueColor => {
                        dst.push_str(color.to_string().as_str());
                    }
                }
            } else {
                tracing::warn!("Failed to capture matched 256 color: {}", m.as_str());
            }
        } else if let Some(m) = caps.name("true") {
            if let Ok(color) = Color::from_str(m.as_str()) {
                match self.color_support {
                    ColorSupport::None => (),
                    ColorSupport::Colors16 => {
                        let color = Color16::from(Color256::from(color));
                        dst.push_str(color.to_string().as_str());
                    }
                    ColorSupport::Colors256 => {
                        let color = Color256::from(color);
                        dst.push_str(color.to_string().as_str());
                    }
                    ColorSupport::TrueColor => {
                        dst.push_str(color.to_string().as_str());
                    }
                }
            } else {
                tracing::warn!("Failed to capture matched true color: {}", m.as_str());
            }
        } else if let Some(_name) = caps.name("name") {
            tracing::info!("Unimplemented: named colors");
        } else if caps.name("clear").is_some() {
            match self.color_support {
                ColorSupport::None => todo!(),
                ColorSupport::Colors16 | ColorSupport::Colors256 | ColorSupport::TrueColor => {
                    dst.push_str(CLEAR_COLOR);
                }
            }
        } else {
            tracing::warn!("Unknown color capture occurred.");
        }
    }
}

pub fn word_list(mut words: Vec<String>) -> String {
    if words.is_empty() {
        String::new()
    } else if words.len() == 1 {
        words.pop().unwrap()
    } else if words.len() == 2 {
        words.join(" and ")
    } else {
        let last = words.pop().unwrap();
        let joined = words.join(", ");
        format!("{}, and {}", joined, last)
    }
}

// https://nitschinger.at/Text-Analysis-in-Rust-Tokenization/
pub struct Tokenizer<'a> {
    input: &'a str,
    byte_offset: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Tokenizer {
            input,
            byte_offset: 0,
        }
    }

    pub fn rest(&self) -> &'a str {
        &self.input[self.byte_offset..]
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        // Accounts for skipped whitespace at the beginning of the next token
        let mut skipped_bytes = 0;

        // Iterate through whitespace indices with various offsets
        for (byte_index, c) in self.input[self.byte_offset..]
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
        {
            let char_len = c.len_utf8();

            // Leading whitespace should be consumed
            if byte_index - skipped_bytes == 0 {
                skipped_bytes += char_len;
                continue;
            }

            // We found non-leading whitespace, return the token in between
            let slice =
                &self.input[self.byte_offset + skipped_bytes..self.byte_offset + byte_index];
            self.byte_offset += byte_index + char_len;
            return Some(slice);
        }

        // If there is no trailing whitespace, consume the rest as the final token
        if self.byte_offset < self.input.len() {
            let slice = &self.input[self.byte_offset + skipped_bytes..];
            self.byte_offset = self.input.len();
            Some(slice)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::COLOR_TAG;

    #[test]
    fn test_color_escape() {
        let caps = COLOR_TAG.captures("||").unwrap();
        assert_eq!(caps.name("escape").unwrap().as_str(), "||");
    }

    #[test]
    fn test_256_color() {
        let caps = COLOR_TAG.captures("|0|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "0");

        let caps = COLOR_TAG.captures("|255|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "255");

        let caps = COLOR_TAG.captures("|44|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "44");

        let caps = COLOR_TAG.captures("|197|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "197");

        let caps = COLOR_TAG.captures("|232|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "232");
    }

    #[test]
    fn test_true_color() {
        let caps = COLOR_TAG.captures("|#000000|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "000000");

        let caps = COLOR_TAG.captures("|#FfFfFf|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "FfFfFf");

        let caps = COLOR_TAG.captures("|#7152d1|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "7152d1");
    }

    #[test]
    fn test_named_color() {
        let caps = COLOR_TAG.captures("|white|").unwrap();
        assert_eq!(caps.name("name").unwrap().as_str(), "white");
    }

    #[test]
    fn test_clear_color() {
        let caps = COLOR_TAG.captures("|-|").unwrap();
        assert_eq!(caps.name("clear").unwrap().as_str(), "|-|");
    }
}
