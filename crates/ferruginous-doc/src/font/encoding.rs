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
                    // Simplified: map some common names to unicode
                    match name.as_str() {
                        "space" => Some(" ".to_string()),
                        "Euro" => Some("€".to_string()),
                        _ if name.len() == 1 => Some(name.clone()),
                        _ => None,
                    }
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
}

fn get_win_ansi_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 { m.insert(i, i as char); }
        // WinAnsi specifics (simplified)
        m.insert(128, '€');
        m
    })
}

fn get_mac_roman_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 { m.insert(i, i as char); }
        m
    })
}

fn get_standard_map() -> &'static BTreeMap<u8, char> {
    static MAP: OnceLock<BTreeMap<u8, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for i in 32..127 { m.insert(i, i as char); }
        m
    })
}
