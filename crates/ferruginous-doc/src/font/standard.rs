use crate::font::encoding::Encoding;

/// Metrics for a standard 14 font.
pub struct StandardFontMetrics {
    pub name: &'static str,
    pub alias: &'static [&'static str],
    pub widths: [u16; 256], // Widths in 1/1000 em
    pub ascent: i16,
    pub descent: i16,
    pub default_encoding: Encoding,
}

/// Returns metrics for a standard font if the name matches a known standard font or alias.
pub fn get_standard_metrics(name: &str) -> Option<&'static StandardFontMetrics> {
    STANDARD_FONTS.iter().find(|&metrics| metrics.name == name || metrics.alias.contains(&name))
}

// Representative widths for Helvetica (simplified for this implementation)
const HELVETICA_WIDTHS: [u16; 256] = {
    let mut w = [0u16; 256];
    // ASCII range
    let mut i = 32; while i < 127 { w[i] = 600; i += 1; } // Generic width for Sans
    // Specific characters (examples)
    w[b'i' as usize] = 278;
    w[b'l' as usize] = 278;
    w[b'm' as usize] = 833;
    w[b'w' as usize] = 833;
    w[b'I' as usize] = 333;
    w[b' ' as usize] = 278;
    w
};

const TIMES_WIDTHS: [u16; 256] = {
    let mut w = [0u16; 256];
    let mut i = 32; while i < 127 { w[i] = 500; i += 1; } // Generic width for Serif
    w[b'i' as usize] = 273;
    w[b'm' as usize] = 783;
    w
};

const COURIER_WIDTHS: [u16; 256] = [600; 256]; // Fixed width

static STANDARD_FONTS: &[StandardFontMetrics] = &[
    StandardFontMetrics {
        name: "Helvetica",
        alias: &["Arial", "ArialMT", "Helvetica-Normal"],
        widths: HELVETICA_WIDTHS,
        ascent: 718,
        descent: -207,
        default_encoding: Encoding::Standard,
    },
    StandardFontMetrics {
        name: "Helvetica-Bold",
        alias: &["Arial-Bold", "Arial-BoldMT"],
        widths: HELVETICA_WIDTHS, // Simplified: use normal widths for now
        ascent: 718,
        descent: -207,
        default_encoding: Encoding::Standard,
    },
    StandardFontMetrics {
        name: "Times-Roman",
        alias: &["TimesNewRoman", "TimesNewRomanPSMT"],
        widths: TIMES_WIDTHS,
        ascent: 683,
        descent: -217,
        default_encoding: Encoding::Standard,
    },
    StandardFontMetrics {
        name: "Courier",
        alias: &["CourierNew", "CourierNewPSMT"],
        widths: COURIER_WIDTHS,
        ascent: 629,
        descent: -157,
        default_encoding: Encoding::Standard,
    },
    // ... Others would be added here
];
