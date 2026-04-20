use std::collections::BTreeMap;

/// Represents a character encoding for simple (8-bit) fonts.
/// ISO 32000-2:2020 Clause 9.6.6 - Character Encoding
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    Standard,
    MacRoman,
    WinAnsi,
    Symbol,
    ZapfDingbats,
    /// Custom encoding with optional differences
    Custom {
        base: Option<Box<Encoding>>,
        differences: BTreeMap<u8, String>,
    },
}

use std::sync::OnceLock;

impl Encoding {
    /// Resolves a byte to a Unicode string.
    pub fn to_unicode(&self, byte: u8) -> Option<String> {
        match self {
            Self::WinAnsi => get_win_ansi_map().get(&byte).map(|&c| c.to_string()),
            Self::MacRoman => get_mac_roman_map().get(&byte).map(|&c| c.to_string()),
            Self::Standard => get_standard_map().get(&byte).map(|&c| c.to_string()),
            Self::Custom { base, differences } => {
                if let Some(name) = differences.get(&byte) {
                    Self::glyph_name_to_unicode(name)
                } else if let Some(b) = base {
                    b.to_unicode(byte)
                } else {
                    None
                }
            }
            _ => {
                // Default to ASCII for safety if within printable range
                if (32..=126).contains(&byte) { Some((byte as char).to_string()) } else { None }
            }
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "StandardEncoding" => Some(Self::Standard),
            "MacRomanEncoding" => Some(Self::MacRoman),
            "WinAnsiEncoding" => Some(Self::WinAnsi),
            "Symbol" => Some(Self::Symbol),
            "ZapfDingbats" => Some(Self::ZapfDingbats),
            _ => None,
        }
    }

    /// Maps a glyph name to a Unicode string using a basic mapping.
    /// In a full implementation, this would use the Adobe Glyph List (AGL).
    pub fn glyph_name_to_unicode(name: &str) -> Option<String> {
        match name {
            "space" => Some(" ".into()),
            "exclam" => Some("!".into()),
            "quotedbl" => Some("\"".into()),
            "numbersign" => Some("#".into()),
            "dollar" => Some("$".into()),
            "percent" => Some("%".into()),
            "ampersand" => Some("&".into()),
            "quoteright" => Some("'".into()),
            "parenleft" => Some("(".into()),
            "parenright" => Some(")".into()),
            "asterisk" => Some("*".into()),
            "plus" => Some("+".into()),
            "comma" => Some(",".into()),
            "hyphen" | "minus" => Some("-".into()),
            "period" => Some(".".into()),
            "slash" => Some("/".into()),
            "zero" => Some("0".into()),
            "one" => Some("1".into()),
            "two" => Some("2".into()),
            "three" => Some("3".into()),
            "four" => Some("4".into()),
            "five" => Some("5".into()),
            "six" => Some("6".into()),
            "seven" => Some("7".into()),
            "eight" => Some("8".into()),
            "nine" => Some("9".into()),
            "colon" => Some(":".into()),
            "semicolon" => Some(";".into()),
            "less" => Some("<".into()),
            "equal" => Some("=".into()),
            "greater" => Some(">".into()),
            "question" => Some("?".into()),
            "at" => Some("@".into()),
            "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N"
            | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" => {
                Some(name.into())
            }
            "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n"
            | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" => {
                Some(name.into())
            }
            "bracketleft" => Some("[".into()),
            "backslash" => Some("\\".into()),
            "bracketright" => Some("]".into()),
            "asciicircum" => Some("^".into()),
            "underscore" => Some("_".into()),
            "quoteleft" => Some("`".into()),
            "braceleft" => Some("{".into()),
            "bar" => Some("|".into()),
            "braceright" => Some("}".into()),
            "asciitilde" => Some("~".into()),
            "Euro" => Some("€".into()),
            _ if name.len() == 1 => Some(name.into()),
            _ => None,
        }
    }
}

fn get_win_ansi_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 {
            m.insert(i, i as char);
        }
        // WinAnsi specifics (simplified)
        m.insert(128, '€');
        m
    })
}

fn get_mac_roman_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 {
            m.insert(i, i as char);
        }
        m
    })
}

fn get_standard_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 {
            m.insert(i, i as char);
        }
        m
    })
}
