use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::{Regex, Replacer};
use std::{collections::HashMap, num::ParseIntError, str::FromStr};

// Some code in this module derived from the below link. See link for license.
// https://github.com/tmux/tmux/blob/8554b80b8b9e70b641847a8534af6d5fbc1a39c7/colour.c

pub const CLEAR_COLOR: &str = "\x1b[m";

static COLOR_TAG_MATCHER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?P<escape>\|\|)|\|(?P<byte>(1?[0-9]{1,2})|(2[0-4][0-9])|(25[0-5]))\||\|#(?P<true>[[:xdigit:]]{6})\||\|(?P<name>[[:alnum:]]+)\||(?P<clear>\|-\|)"#,
    ).unwrap()
});

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorSupport {
    None,
    Colors16,
    Colors256,
    TrueColor,
}

impl ColorSupport {
    fn supports_color(&self) -> bool {
        match self {
            ColorSupport::None => false,
            ColorSupport::Colors16 | ColorSupport::Colors256 | ColorSupport::TrueColor => true,
        }
    }

    fn supported_from_true(&self, color: ColorTrue) -> Option<Color> {
        match self {
            ColorSupport::None => None,
            ColorSupport::Colors16 => Some(Color16::from(Color256::from(color)).into()),
            ColorSupport::Colors256 => Some(Color256::from(color).into()),
            ColorSupport::TrueColor => Some(color.into()),
        }
    }

    fn supported_from_256(&self, color: Color256) -> Option<Color> {
        match self {
            ColorSupport::None => None,
            ColorSupport::Colors16 => Some(Color16::from(color).into()),
            ColorSupport::Colors256 | ColorSupport::TrueColor => Some(color.into()),
        }
    }
}

pub fn colorize_web(message: &str) -> String {
    let mut open = 0;
    let replacer = WebReplacer::new(&mut open);
    let mut message = COLOR_TAG_MATCHER.replace_all(message, replacer).to_string();
    while open > 0 {
        message.push_str("</span>");
        open -= 1;
    }
    message
}

struct WebReplacer<'a> {
    stack: Vec<ColorTrue>,
    open: &'a mut i32,
}

impl<'a> WebReplacer<'a> {
    fn new(open: &'a mut i32) -> Self {
        WebReplacer {
            stack: Vec::new(),
            open,
        }
    }
}

impl<'a> Replacer for WebReplacer<'a> {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        if caps.name("escape").is_some() {
            dst.push('|')
        } else if let Some(m) = caps.name("byte") {
            if let Ok(color) = Color256::from_str(m.as_str()) {
                let color = ColorTrue::from(color);
                self.stack.push(color);
                *self.open += 1;
                dst.push_str(format!(r#"<span style="color: #{};">"#, color.as_hex()).as_str());
            } else {
                tracing::warn!("failed to capture matched 256 color: {}", m.as_str());
            }
        } else if let Some(m) = caps.name("true") {
            if let Ok(color) = ColorTrue::from_str(m.as_str()) {
                self.stack.push(color);
                *self.open += 1;
                dst.push_str(format!(r#"<span style="color: #{};">"#, color.as_hex()).as_str());
            } else {
                tracing::warn!("failed to capture matched true color: {}", m.as_str());
            }
        } else if let Some(name) = caps.name("name") {
            if let Some(index) = COLOR_NAME_MAP.get(name.as_str().to_lowercase().as_str()) {
                let color = ColorTrue::from(Color256::new(*index));
                self.stack.push(color);
                *self.open += 1;
                dst.push_str(format!(r#"<span style="color: #{};">"#, color.as_hex()).as_str());
            }
        } else if caps.name("clear").is_some() {
            if !self.stack.is_empty() {
                self.stack.pop();
                *self.open -= 1;
                dst.push_str("</span>");
            }
        } else {
            let capture = caps
                .iter()
                .flat_map(|m| m.map(|m| format!("'{}'", m.as_str())))
                .join(", ");
            tracing::warn!("unknown color tag(s) captured: {}", capture);
        }
    }
}

pub fn colorize_telnet(message: &str, color_support: ColorSupport) -> String {
    let mut closed = true;
    let replacer = TelnetReplacer::new(color_support, &mut closed);
    let mut message = COLOR_TAG_MATCHER.replace_all(message, replacer).to_string();
    if !closed {
        message.push_str(CLEAR_COLOR)
    }
    message
}

struct TelnetReplacer<'a> {
    color_support: ColorSupport,
    stack: Vec<Option<Color>>,
    closed: &'a mut bool,
}

impl<'a> TelnetReplacer<'a> {
    fn new(color_support: ColorSupport, closed: &'a mut bool) -> Self {
        TelnetReplacer {
            color_support,
            stack: Vec::new(),
            closed,
        }
    }
}

impl<'a> Replacer for TelnetReplacer<'a> {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        if caps.name("escape").is_some() {
            dst.push('|')
        } else if let Some(m) = caps.name("byte") {
            if let Ok(color) = Color256::from_str(m.as_str()) {
                if let Some(color) = self.color_support.supported_from_256(color) {
                    self.stack.push(Some(color));
                    dst.push_str(color.to_string().as_str());
                    *self.closed = false;
                }
            } else {
                self.stack.push(None);
                tracing::warn!("failed to capture matched 256 color: {}", m.as_str());
            }
        } else if let Some(m) = caps.name("true") {
            if let Ok(color) = ColorTrue::from_str(m.as_str()) {
                if let Some(color) = self.color_support.supported_from_true(color) {
                    self.stack.push(Some(color));
                    dst.push_str(color.to_string().as_str());
                    *self.closed = false;
                } else {
                    self.stack.push(None);
                }
            } else {
                self.stack.push(None);
                tracing::warn!("failed to capture matched true color: {}", m.as_str());
            }
        } else if let Some(name) = caps.name("name") {
            if let Some(index) = COLOR_NAME_MAP.get(name.as_str().to_lowercase().as_str()) {
                if let Some(color) = self.color_support.supported_from_256(Color256::new(*index)) {
                    self.stack.push(Some(color));
                    dst.push_str(color.to_string().as_str());
                    *self.closed = false;
                } else {
                    self.stack.push(None)
                }
            } else {
                self.stack.push(None)
            }
        } else if caps.name("clear").is_some() {
            if self.color_support.supports_color() && !self.stack.is_empty() {
                self.stack.pop();

                if let Some(color) = self.stack.last() {
                    if let Some(color) = color {
                        // Only resume coloring if there is a previous color and the opening color tag was valid
                        dst.push_str(color.to_string().as_str())
                    }
                } else {
                    *self.closed = true;
                    dst.push_str(CLEAR_COLOR);
                }
            }
        } else {
            let capture = caps
                .iter()
                .flat_map(|m| m.map(|m| format!("'{}'", m.as_str())))
                .join(", ");
            tracing::warn!("unknown color tag(s) captured: {}", capture);
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub enum Color {
    ColorTrue(ColorTrue),
    Color256(Color256),
    Color16(Color16),
}

impl From<ColorTrue> for Color {
    fn from(value: ColorTrue) -> Self {
        Color::ColorTrue(value)
    }
}

impl From<Color256> for Color {
    fn from(value: Color256) -> Self {
        Color::Color256(value)
    }
}

impl From<Color16> for Color {
    fn from(value: Color16) -> Self {
        Color::Color16(value)
    }
}

impl ToString for Color {
    fn to_string(&self) -> String {
        match self {
            Color::ColorTrue(c) => c.to_string(),
            Color::Color256(c) => c.to_string(),
            Color::Color16(c) => c.to_string(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorTrue {
    r: u8,
    g: u8,
    b: u8,
}

impl ColorTrue {
    fn new(r: u8, g: u8, b: u8) -> Self {
        ColorTrue { r, g, b }
    }

    pub fn as_hex(&self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Create a new gray using a single color
    fn new_gray(v: u8) -> Self {
        ColorTrue::new(v, v, v)
    }

    /// Calculate the distance between this color and another by squaring
    /// and adding the component colors.
    fn distance_squared(&self, color: ColorTrue) -> u32 {
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

impl From<Color256> for ColorTrue {
    fn from(color: Color256) -> Self {
        ColorTrue::from(COLORS_256[color.0 as usize])
    }
}

impl From<u32> for ColorTrue {
    fn from(c: u32) -> Self {
        ColorTrue::new(
            ((c >> 16) & 0xff) as u8,
            ((c >> 8) & 0xff) as u8,
            (c & 0xff) as u8,
        )
    }
}

impl FromStr for ColorTrue {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ColorTrue::from(u32::from_str_radix(s, 16)?))
    }
}

impl ToString for ColorTrue {
    fn to_string(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, Copy)]
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
impl From<ColorTrue> for Color256 {
    fn from(c: ColorTrue) -> Self {
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
        let gray = ColorTrue::new_gray(8 + (10 * gray_index));

        // Determine if the color is closer to the cube color or the gray.
        let cube_color = ColorTrue::new(cr, cg, cg);
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

#[derive(Debug, Clone, Copy)]
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
            format!("\x1b[{}m", code)
        } else {
            let code = self.0 - 8 + 90;
            format!("\x1b[{}m", code)
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

static COLOR_NAME_MAP: Lazy<HashMap<&'static str, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("aqua", 14);
    map.insert("aquamarine1", 79);
    map.insert("aquamarine2", 86);
    map.insert("aquamarine3", 122);
    map.insert("black", 0);
    map.insert("blue", 12);
    map.insert("blue1", 19);
    map.insert("blue2", 20);
    map.insert("blue3", 21);
    map.insert("blueviolet", 57);
    map.insert("cadetblue1", 72);
    map.insert("cadetblue2", 73);
    map.insert("chartreuse1", 64);
    map.insert("chartreuse2", 70);
    map.insert("chartreuse3", 76);
    map.insert("chartreuse4", 82);
    map.insert("chartreuse5", 112);
    map.insert("chartreuse6", 118);
    map.insert("cornflowerblue", 69);
    map.insert("cornsilk", 230);
    map.insert("cyan1", 43);
    map.insert("cyan2", 50);
    map.insert("cyan3", 51);
    map.insert("darkblue", 18);
    map.insert("darkcyan", 36);
    map.insert("darkgoldenrod", 136);
    map.insert("darkgreen", 22);
    map.insert("darkkhaki", 143);
    map.insert("darkmagenta1", 90);
    map.insert("darkmagenta2", 91);
    map.insert("darkolivegreen1", 107);
    map.insert("darkolivegreen2", 113);
    map.insert("darkolivegreen3", 149);
    map.insert("darkolivegreen4", 155);
    map.insert("darkolivegreen5", 191);
    map.insert("darkolivegreen6", 192);
    map.insert("darkorange1", 130);
    map.insert("darkorange2", 166);
    map.insert("darkorange3", 208);
    map.insert("darkred1", 52);
    map.insert("darkred2", 88);
    map.insert("darkseagreen1", 65);
    map.insert("darkseagreen2", 71);
    map.insert("darkseagreen3", 108);
    map.insert("darkseagreen3", 150);
    map.insert("darkseagreen4", 115);
    map.insert("darkseagreen5", 151);
    map.insert("darkseagreen6", 157);
    map.insert("darkseagreen7", 158);
    map.insert("darkseagreen8", 193);
    map.insert("darkslategray1", 87);
    map.insert("darkslategray2", 116);
    map.insert("darkslategray3", 123);
    map.insert("darkturquoise", 44);
    map.insert("darkviolet1", 92);
    map.insert("darkviolet2", 128);
    map.insert("deeppink1", 53);
    map.insert("deeppink2", 89);
    map.insert("deeppink3", 125);
    map.insert("deeppink4", 161);
    map.insert("deeppink5", 162);
    map.insert("deeppink6", 197);
    map.insert("deeppink7", 198);
    map.insert("deeppink8", 199);
    map.insert("deepskyblue1", 23);
    map.insert("deepskyblue2", 24);
    map.insert("deepskyblue3", 25);
    map.insert("deepskyblue4", 31);
    map.insert("deepskyblue5", 32);
    map.insert("deepskyblue6", 38);
    map.insert("deepskyblue7", 39);
    map.insert("dodgerblue1", 26);
    map.insert("dodgerblue2", 27);
    map.insert("dodgerblue3", 33);
    map.insert("fuchsia", 13);
    map.insert("gold1", 142);
    map.insert("gold2", 178);
    map.insert("gold3", 220);
    map.insert("gray", 8);
    map.insert("gray0", 16);
    map.insert("gray100", 231);
    map.insert("gray11", 234);
    map.insert("gray15", 235);
    map.insert("gray19", 236);
    map.insert("gray23", 237);
    map.insert("gray27", 238);
    map.insert("gray3", 232);
    map.insert("gray30", 239);
    map.insert("gray35", 240);
    map.insert("gray37", 59);
    map.insert("gray39", 241);
    map.insert("gray42", 242);
    map.insert("gray46", 243);
    map.insert("gray50", 244);
    map.insert("gray53", 102);
    map.insert("gray54", 245);
    map.insert("gray58", 246);
    map.insert("gray62", 247);
    map.insert("gray63", 139);
    map.insert("gray66", 248);
    map.insert("gray69", 145);
    map.insert("gray7", 233);
    map.insert("gray70", 249);
    map.insert("gray74", 250);
    map.insert("gray78", 251);
    map.insert("gray82", 252);
    map.insert("gray84", 188);
    map.insert("gray85", 253);
    map.insert("gray89", 254);
    map.insert("gray93", 255);
    map.insert("green", 2);
    map.insert("green1", 28);
    map.insert("green2", 34);
    map.insert("green3", 40);
    map.insert("green4", 46);
    map.insert("greenyellow", 154);
    map.insert("honeydew", 194);
    map.insert("hotpink1", 132);
    map.insert("hotpink2", 168);
    map.insert("hotpink3", 169);
    map.insert("hotpink5", 205);
    map.insert("hotpink6", 206);
    map.insert("indianred1", 131);
    map.insert("indianred2", 167);
    map.insert("indianred3", 203);
    map.insert("indianred4", 204);
    map.insert("khaki1", 185);
    map.insert("khaki2", 228);
    map.insert("lightcoral", 210);
    map.insert("lightcyan1", 152);
    map.insert("lightcyan2", 195);
    map.insert("lightgoldenrod1", 179);
    map.insert("lightgoldenrod2", 186);
    map.insert("lightgoldenrod3", 221);
    map.insert("lightgoldenrod4", 222);
    map.insert("lightgoldenrod5", 227);
    map.insert("lightgreen1", 119);
    map.insert("lightgreen2", 120);
    map.insert("lightpink1", 95);
    map.insert("lightpink2", 174);
    map.insert("lightpink3", 217);
    map.insert("lightsalmon1", 137);
    map.insert("lightsalmon2", 173);
    map.insert("lightsalmon3", 216);
    map.insert("lightseagreen", 37);
    map.insert("lightskyblue1", 109);
    map.insert("lightskyblue2", 110);
    map.insert("lightskyblue3", 153);
    map.insert("lightslateblue", 105);
    map.insert("lightslategrey", 103);
    map.insert("lightsteelblue1", 146);
    map.insert("lightsteelblue2", 147);
    map.insert("lightsteelblue3", 189);
    map.insert("lightyellow", 187);
    map.insert("lime", 10);
    map.insert("magenta1", 127);
    map.insert("magenta2", 163);
    map.insert("magenta3", 164);
    map.insert("magenta4", 165);
    map.insert("magenta5", 200);
    map.insert("magenta6", 201);
    map.insert("maroon", 1);
    map.insert("mediumorchid1", 133);
    map.insert("mediumorchid2", 134);
    map.insert("mediumorchid3", 171);
    map.insert("mediumorchid4", 207);
    map.insert("mediumpurple1", 60);
    map.insert("mediumpurple2", 97);
    map.insert("mediumpurple3", 98);
    map.insert("mediumpurple4", 104);
    map.insert("mediumpurple5", 135);
    map.insert("mediumpurple6", 140);
    map.insert("mediumpurple7", 141);
    map.insert("mediumspringgreen", 49);
    map.insert("mediumturquoise", 80);
    map.insert("mediumvioletred", 126);
    map.insert("mistyrose1", 181);
    map.insert("mistyrose2", 224);
    map.insert("navajowhite1", 144);
    map.insert("navajowhite2", 223);
    map.insert("navy", 4);
    map.insert("navyblue", 17);
    map.insert("olive", 3);
    map.insert("orange1", 58);
    map.insert("orange2", 94);
    map.insert("orange3", 172);
    map.insert("orange4", 214);
    map.insert("orangered", 202);
    map.insert("orchid1", 170);
    map.insert("orchid2", 212);
    map.insert("orchid3", 213);
    map.insert("palegreen1", 77);
    map.insert("palegreen2", 114);
    map.insert("palegreen3", 121);
    map.insert("palegreen4", 156);
    map.insert("paleturquoise1", 66);
    map.insert("paleturquoise2", 159);
    map.insert("palevioletred", 211);
    map.insert("pink1", 175);
    map.insert("pink2", 218);
    map.insert("plum", 96);
    map.insert("plum2", 176);
    map.insert("plum3", 183);
    map.insert("plum4", 219);
    map.insert("purple", 5);
    map.insert("purple1", 54);
    map.insert("purple2", 55);
    map.insert("purple3", 56);
    map.insert("purple4", 93);
    map.insert("purple5", 129);
    map.insert("red", 9);
    map.insert("red1", 124);
    map.insert("red2", 160);
    map.insert("red3", 196);
    map.insert("rosybrown", 138);
    map.insert("royalblue", 63);
    map.insert("salmon", 209);
    map.insert("sandybrown", 215);
    map.insert("seagreen1", 78);
    map.insert("seagreen2", 83);
    map.insert("seagreen3", 84);
    map.insert("seagreen4", 85);
    map.insert("silver", 7);
    map.insert("skyblue1", 74);
    map.insert("skyblue2", 111);
    map.insert("skyblue3", 117);
    map.insert("slateblue1", 61);
    map.insert("slateblue2", 62);
    map.insert("slateblue3", 99);
    map.insert("springgreen1", 29);
    map.insert("springgreen2", 35);
    map.insert("springgreen3", 41);
    map.insert("springgreen4", 42);
    map.insert("springgreen5", 47);
    map.insert("springgreen6", 48);
    map.insert("steelblue1", 67);
    map.insert("steelblue2", 68);
    map.insert("steelblue3", 75);
    map.insert("steelblue4", 81);
    map.insert("tan", 180);
    map.insert("teal", 6);
    map.insert("thistle1", 182);
    map.insert("thistle2", 225);
    map.insert("turquoise1", 30);
    map.insert("turquoise2", 45);
    map.insert("violet", 177);
    map.insert("wheat1", 101);
    map.insert("wheat2", 229);
    map.insert("white", 15);
    map.insert("yellow", 11);
    map.insert("yellow1", 100);
    map.insert("yellow2", 106);
    map.insert("yellow3", 148);
    map.insert("yellow4", 184);
    map.insert("yellow5", 190);
    map.insert("yellow6", 226);
    map
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::color::{ColorSupport, TelnetReplacer};

    use super::COLOR_TAG_MATCHER;

    #[test]
    fn test_color_escape() {
        let caps = COLOR_TAG_MATCHER.captures("||").unwrap();
        assert_eq!(caps.name("escape").unwrap().as_str(), "||");
    }

    #[test]
    fn test_256_color() {
        let caps = COLOR_TAG_MATCHER.captures("|0|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "0");

        let caps = COLOR_TAG_MATCHER.captures("|255|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "255");

        let caps = COLOR_TAG_MATCHER.captures("|44|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "44");

        let caps = COLOR_TAG_MATCHER.captures("|197|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "197");

        let caps = COLOR_TAG_MATCHER.captures("|232|").unwrap();
        assert_eq!(caps.name("byte").unwrap().as_str(), "232");
    }

    #[test]
    fn test_true_color() {
        let caps = COLOR_TAG_MATCHER.captures("|#000000|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "000000");

        let caps = COLOR_TAG_MATCHER.captures("|#FfFfFf|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "FfFfFf");

        let caps = COLOR_TAG_MATCHER.captures("|#7152d1|").unwrap();
        assert_eq!(caps.name("true").unwrap().as_str(), "7152d1");
    }

    #[test]
    fn test_named_color() {
        let caps = COLOR_TAG_MATCHER.captures("|white|").unwrap();
        assert_eq!(caps.name("name").unwrap().as_str(), "white");
    }

    #[test]
    fn test_clear_color() {
        let caps = COLOR_TAG_MATCHER.captures("|-|").unwrap();
        assert_eq!(caps.name("clear").unwrap().as_str(), "|-|");
    }

    #[test]
    fn test_replacer_closed_true() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::TrueColor, &mut closed);
        COLOR_TAG_MATCHER.replace_all("|0|text|-|", replacer);
        assert_eq!(closed, true);
    }

    #[test]
    fn test_replacer_closed_false() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::TrueColor, &mut closed);
        COLOR_TAG_MATCHER.replace("|0|text", replacer);
        assert_eq!(closed, false);
    }

    #[test]
    fn test_replacer_keeps_true() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::TrueColor, &mut closed);
        let result = COLOR_TAG_MATCHER.replace("|#123456|text", replacer);
        assert_eq!(result, Cow::from("\x1b[38;2;18;52;86mtext"))
    }

    #[test]
    fn test_replacer_lowers_true_to_256() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::Colors256, &mut closed);
        let result = COLOR_TAG_MATCHER.replace("|#123456|text", replacer);
        assert_eq!(result, Cow::from("\x1b[38;5;23mtext"))
    }

    #[test]
    fn test_replacer_lowers_true_to_16() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::Colors16, &mut closed);
        let result = COLOR_TAG_MATCHER.replace("|#123456|text", replacer);
        assert_eq!(result, Cow::from("\x1b[36mtext"))
    }

    #[test]
    fn test_replacer_keeps_256() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::Colors256, &mut closed);
        let result = COLOR_TAG_MATCHER.replace("|48|text", replacer);
        assert_eq!(result, Cow::from("\x1b[38;5;48mtext"))
    }

    #[test]
    fn test_replacer_lowers_256_to_16() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::Colors16, &mut closed);
        let result = COLOR_TAG_MATCHER.replace("|48|text", replacer);
        assert_eq!(result, Cow::from("\x1b[92mtext"))
    }

    #[test]
    fn test_replacer_removes_color() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::None, &mut closed);
        let result = COLOR_TAG_MATCHER.replace_all("|#123456|t|200|e|Red1|x|-|t", replacer);
        assert_eq!(result, Cow::from("text"));
    }

    #[test]
    fn test_replacer_translates_escape() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::None, &mut closed);
        let result = COLOR_TAG_MATCHER.replace_all("||text||", replacer);
        assert_eq!(result, Cow::from("|text|"));
    }

    #[test]
    fn test_replacer_nesting() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::TrueColor, &mut closed);
        let result =
            COLOR_TAG_MATCHER.replace_all("|#654321|some |#123456|pretty|-| text|-|", replacer);
        assert_eq!(
            result,
            Cow::from(
                "\x1b[38;2;101;67;33msome \x1b[38;2;18;52;86mpretty\x1b[38;2;101;67;33m text\x1b[m"
            )
        )
    }

    #[test]
    fn test_replacer_extra_close() {
        let mut closed = true;
        let replacer = TelnetReplacer::new(ColorSupport::TrueColor, &mut closed);
        let result = COLOR_TAG_MATCHER.replace_all("|#654321|text|-| |-|", replacer);
        assert_eq!(result, Cow::from("\x1b[38;2;101;67;33mtext\x1b[m "))
    }
}
