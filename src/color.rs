use lazy_static::lazy_static;
use regex::{Regex, Replacer};
use std::{num::ParseIntError, str::FromStr};

// Some code in this module derived from the below link. See link for license.
// https://github.com/tmux/tmux/blob/8554b80b8b9e70b641847a8534af6d5fbc1a39c7/colour.c

pub const CLEAR_COLOR: &str = "\x1b[m";

lazy_static! {
    static ref COLOR_TAG: Regex = Regex::new(
        r#"(?P<escape>\|\|)|\|(?P<byte>(1?[0-9]{1,2})|(2[0-4][0-9])|(25[0-5]))\||\|#(?P<true>[[:xdigit:]]{6})\||\|(?P<name>[[:alnum:]]+)\||(?P<clear>\|/\|)"#,
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
    let mut closed = true;
    let replacer = ColorReplacer::new(color_support, &mut closed);
    let mut message = COLOR_TAG.replace_all(message, replacer).to_string();
    if !closed {
        message.push_str(CLEAR_COLOR)
    }
    message
}

struct ColorReplacer<'a> {
    color_support: ColorSupport,
    closed: &'a mut bool,
}

impl<'a> ColorReplacer<'a> {
    fn new(color_support: ColorSupport, closed: &'a mut bool) -> Self {
        ColorReplacer {
            color_support,
            closed,
        }
    }
}

impl<'a> Replacer for ColorReplacer<'a> {
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
                *self.closed = true;
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
                *self.closed = true;
            } else {
                tracing::warn!("Failed to capture matched true color: {}", m.as_str());
            }
        } else if let Some(_name) = caps.name("name") {
            tracing::info!("Unimplemented: named colors");
            *self.closed = true;
        } else if caps.name("clear").is_some() {
            match self.color_support {
                ColorSupport::None => todo!(),
                ColorSupport::Colors16 | ColorSupport::Colors256 | ColorSupport::TrueColor => {
                    dst.push_str(CLEAR_COLOR);
                }
            }
            *self.closed = false;
        } else {
            tracing::warn!("Unknown color capture occurred.");
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Create a new gray using a single color
    fn new_gray(v: u8) -> Self {
        Color::new(v, v, v)
    }

    /// Calculate the distance between this color and another by squaring
    /// and adding the component colors.
    fn distance_squared(&self, color: Color) -> u32 {
        ((self.r as i32 - color.r as i32) * (self.r as i32 - color.r as i32)
            + (self.g as i32 - color.g as i32) * (self.g as i32 - color.g as i32)
            + (self.b as i32 - color.b as i32) * (self.b as i32 - color.b as i32)) as u32
    }

    /// Average the color into a gray color value.
    fn gray_average(&self) -> u8 {
        ((self.r as u16 + self.g as u16 + self.b as u16) / 3) as u8
    }

    /// Return the closest color cube indices in an (R, G, B) tuple.
    fn six_cube_indices(&self) -> (u8, u8, u8) {
        (
            color_to_cube_index(self.r),
            color_to_cube_index(self.g),
            color_to_cube_index(self.b),
        )
    }
}

impl From<Color256> for Color {
    fn from(color: Color256) -> Self {
        Color::from(COLORS_256[color.0 as usize])
    }
}

impl From<u32> for Color {
    fn from(c: u32) -> Self {
        Color::new(
            ((c >> 16) & 0xff) as u8,
            ((c >> 8) & 0xff) as u8,
            (c & 0xff) as u8,
        )
    }
}

impl FromStr for Color {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Color::from(u32::from_str_radix(s, 16)?))
    }
}

impl ToString for Color {
    fn to_string(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }
}

pub struct Color256(u8);

impl Color256 {
    // Create a new 256 color using a specific palette index.
    fn new(color: u8) -> Self {
        Color256(color)
    }

    /// Create a new 256 color from color cube indices.
    fn new_cube(r: u8, g: u8, b: u8) -> Self {
        Color256(CUBE_OFFSET + (36 * r) + (6 * b) + g)
    }
}

// Translates an RGB color into an xterm-compatible 256 color.
// The xterm 256 color palette is organized:
// 0-15: 16 system colors
// 16-231: a 6x6x6 color cube with r * 36, g * 6, and b
// 232-255: grays
//
// The curve from cube index to color value is defined in
// CUBE_TO_COLOR_VALUE.
impl From<Color> for Color256 {
    fn from(c: Color) -> Self {
        // Find the closest matching cube color.
        let (qr, qg, qb) = c.six_cube_indices();
        let (cr, cg, cb) = (
            CUBE_TO_COLOR_VALUE[qr as usize],
            CUBE_TO_COLOR_VALUE[qg as usize],
            CUBE_TO_COLOR_VALUE[qb as usize],
        );

        // If the cube has an exact color match, return early.
        if c.r == cr && c.g == cg && c.b == cb {
            return Color256::new_cube(qr, qg, qb);
        }

        // Find the closest matching gray.
        let gray_average = c.gray_average();
        let gray_index = if gray_average > 238 {
            23
        } else {
            (gray_average - 3) / 10
        };
        let gray = Color::new_gray(8 + (10 * gray_index));

        // Determine if the color is closer to the cube color or the gray.
        let cube_color = Color::new(cr, cg, cg);
        let color_distance = c.distance_squared(cube_color);
        let gray_distance = c.distance_squared(gray);
        if gray_distance < color_distance {
            Color256::new(GRAY_OFFSET + gray_index)
        } else {
            Color256::new_cube(qr, qg, qb)
        }
    }
}

impl FromStr for Color256 {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Color256::new(s.parse::<u8>()?))
    }
}

impl ToString for Color256 {
    fn to_string(&self) -> String {
        format!("\x1b[38;5;{}m", self.0)
    }
}

pub struct Color16(u8);

impl From<Color256> for Color16 {
    fn from(c: Color256) -> Self {
        Color16(COLORS_256_TO_16[c.0 as usize])
    }
}

impl ToString for Color16 {
    fn to_string(&self) -> String {
        if self.0 < 8 {
            let code = self.0 + 30;
            format!("\x1b[1;{}m", code)
        } else {
            let code = self.0 - 8 + 90;
            format!("\x1b[1;{}m", code)
        }
    }
}

fn color_to_cube_index(v: u8) -> u8 {
    if v < 48 {
        0
    } else if v < 114 {
        1
    } else {
        (v - 35) / 40
    }
}

const CUBE_OFFSET: u8 = 16;
const GRAY_OFFSET: u8 = 232;

const CUBE_TO_COLOR_VALUE: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

const COLORS_256: [u32; 256] = [
    0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xc0c0c0, 0x808080,
    0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0xffffff, 0x000000, 0x00005f,
    0x000087, 0x0000af, 0x0000d7, 0x0000ff, 0x005f00, 0x005f5f, 0x005f87, 0x005faf, 0x005fd7,
    0x005fff, 0x008700, 0x00875f, 0x008787, 0x0087af, 0x0087d7, 0x0087ff, 0x00af00, 0x00af5f,
    0x00af87, 0x00afaf, 0x00afd7, 0x00afff, 0x00d700, 0x00d75f, 0x00d787, 0x00d7af, 0x00d7d7,
    0x00d7ff, 0x00ff00, 0x00ff5f, 0x00ff87, 0x00ffaf, 0x00ffd7, 0x00ffff, 0x5f0000, 0x5f005f,
    0x5f0087, 0x5f00af, 0x5f00d7, 0x5f00ff, 0x5f5f00, 0x5f5f5f, 0x5f5f87, 0x5f5faf, 0x5f5fd7,
    0x5f5fff, 0x5f8700, 0x5f875f, 0x5f8787, 0x5f87af, 0x5f87d7, 0x5f87ff, 0x5faf00, 0x5faf5f,
    0x5faf87, 0x5fafaf, 0x5fafd7, 0x5fafff, 0x5fd700, 0x5fd75f, 0x5fd787, 0x5fd7af, 0x5fd7d7,
    0x5fd7ff, 0x5fff00, 0x5fff5f, 0x5fff87, 0x5fffaf, 0x5fffd7, 0x5fffff, 0x870000, 0x87005f,
    0x870087, 0x8700af, 0x8700d7, 0x8700ff, 0x875f00, 0x875f5f, 0x875f87, 0x875faf, 0x875fd7,
    0x875fff, 0x878700, 0x87875f, 0x878787, 0x8787af, 0x8787d7, 0x8787ff, 0x87af00, 0x87af5f,
    0x87af87, 0x87afaf, 0x87afd7, 0x87afff, 0x87d700, 0x87d75f, 0x87d787, 0x87d7af, 0x87d7d7,
    0x87d7ff, 0x87ff00, 0x87ff5f, 0x87ff87, 0x87ffaf, 0x87ffd7, 0x87ffff, 0xaf0000, 0xaf005f,
    0xaf0087, 0xaf00af, 0xaf00d7, 0xaf00ff, 0xaf5f00, 0xaf5f5f, 0xaf5f87, 0xaf5faf, 0xaf5fd7,
    0xaf5fff, 0xaf8700, 0xaf875f, 0xaf8787, 0xaf87af, 0xaf87d7, 0xaf87ff, 0xafaf00, 0xafaf5f,
    0xafaf87, 0xafafaf, 0xafafd7, 0xafafff, 0xafd700, 0xafd75f, 0xafd787, 0xafd7af, 0xafd7d7,
    0xafd7ff, 0xafff00, 0xafff5f, 0xafff87, 0xafffaf, 0xafffd7, 0xafffff, 0xd70000, 0xd7005f,
    0xd70087, 0xd700af, 0xd700d7, 0xd700ff, 0xd75f00, 0xd75f5f, 0xd75f87, 0xd75faf, 0xd75fd7,
    0xd75fff, 0xd78700, 0xd7875f, 0xd78787, 0xd787af, 0xd787d7, 0xd787ff, 0xd7af00, 0xd7af5f,
    0xd7af87, 0xd7afaf, 0xd7afd7, 0xd7afff, 0xd7d700, 0xd7d75f, 0xd7d787, 0xd7d7af, 0xd7d7d7,
    0xd7d7ff, 0xd7ff00, 0xd7ff5f, 0xd7ff87, 0xd7ffaf, 0xd7ffd7, 0xd7ffff, 0xff0000, 0xff005f,
    0xff0087, 0xff00af, 0xff00d7, 0xff00ff, 0xff5f00, 0xff5f5f, 0xff5f87, 0xff5faf, 0xff5fd7,
    0xff5fff, 0xff8700, 0xff875f, 0xff8787, 0xff87af, 0xff87d7, 0xff87ff, 0xffaf00, 0xffaf5f,
    0xffaf87, 0xffafaf, 0xffafd7, 0xffafff, 0xffd700, 0xffd75f, 0xffd787, 0xffd7af, 0xffd7d7,
    0xffd7ff, 0xffff00, 0xffff5f, 0xffff87, 0xffffaf, 0xffffd7, 0xffffff, 0x080808, 0x121212,
    0x1c1c1c, 0x262626, 0x303030, 0x3a3a3a, 0x444444, 0x4e4e4e, 0x585858, 0x626262, 0x6c6c6c,
    0x767676, 0x808080, 0x8a8a8a, 0x949494, 0x9e9e9e, 0xa8a8a8, 0xb2b2b2, 0xbcbcbc, 0xc6c6c6,
    0xd0d0d0, 0xdadada, 0xe4e4e4, 0xeeeeee,
];

const COLORS_256_TO_16: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 4, 4, 4, 12, 12, 2, 6, 4, 4, 12, 12,
    2, 2, 6, 4, 12, 12, 2, 2, 2, 6, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10, 14, 1, 5,
    4, 4, 12, 12, 3, 8, 4, 4, 12, 12, 2, 2, 6, 4, 12, 12, 2, 2, 2, 6, 12, 12, 10, 10, 10, 10, 14,
    12, 10, 10, 10, 10, 10, 14, 1, 1, 5, 4, 12, 12, 1, 1, 5, 4, 12, 12, 3, 3, 8, 4, 12, 12, 2, 2,
    2, 6, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10, 14, 1, 1, 1, 5, 12, 12, 1, 1, 1, 5,
    12, 12, 1, 1, 1, 5, 12, 12, 3, 3, 3, 7, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10, 14,
    9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 13, 12, 11, 11, 11, 11,
    7, 12, 10, 10, 10, 10, 10, 14, 9, 9, 9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 9, 9,
    9, 9, 9, 13, 9, 9, 9, 9, 9, 13, 11, 11, 11, 11, 11, 15, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 8, 8, 7,
    7, 7, 7, 7, 7, 15, 15, 15, 15, 15, 15,
];

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
        let caps = COLOR_TAG.captures("|/|").unwrap();
        assert_eq!(caps.name("clear").unwrap().as_str(), "|/|");
    }
}
