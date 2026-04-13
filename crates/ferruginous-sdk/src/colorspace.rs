//! Color Space management (ISO 32000-2:2020 Clause 8.6).
//!
//! This module implements support for calibrated color spaces, ICC profiles, 
//! and specialized color spaces (Separation, DeviceN).

use crate::core::{Object, Resolver, PdfResult, PdfError, ContentErrorVariant};
use std::collections::BTreeMap;
use std::sync::Arc;
use lcms2::{Profile, Transform, Intent, PixelFormat};

/// Represents the family of a color space.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ColorSpace {
    /// Device-dependent Grayscale color space (Clause 8.6.4.2).
    DeviceGray,
    /// Device-dependent RGB color space (Clause 8.6.4.3).
    DeviceRGB,
    /// Device-dependent CMYK color space (Clause 8.6.4.4).
    DeviceCMYK,
    /// Calibrated Grayscale color space (Clause 8.6.5.2).
    CalGray { 
        /// CIE 1931 XYZ white point.
        white_point: [f32; 3], 
        /// CIE 1931 XYZ black point.
        black_point: [f32; 3], 
        /// Gamma for the gray space.
        gamma: f32 
    },
    /// Calibrated RGB color space (Clause 8.6.5.3).
    CalRGB { 
        /// CIE 1931 XYZ white point.
        white_point: [f32; 3], 
        /// CIE 1931 XYZ black point.
        black_point: [f32; 3], 
        /// Gamma for RGB components.
        gamma: [f32; 3], 
        /// Transformation matrix from CalRGB to XYZ.
        matrix: [f32; 9] 
    },
    /// CIE-based L*a*b* color space (Clause 8.6.5.4).
    Lab { 
        /// CIE 1931 XYZ white point.
        white_point: [f32; 3], 
        /// CIE 1931 XYZ black point.
        black_point: [f32; 3], 
        /// Range of a* and b* components.
        range: [f32; 4] 
    },
    /// ICC-based color space (Clause 8.6.5.5).
    ICCBased(Arc<ICCProfile>),
    /// Indexed color space (Clause 8.6.6.3).
    Indexed { 
        /// The base color space.
        base: Box<ColorSpace>, 
        /// Max index (0-255).
        hival: u8, 
        /// Lookup table data.
        lookup: Arc<Vec<u8>> 
    },
    /// Pattern color space (Clause 8.6.6.2).
    Pattern,
    /// Separation color space (Clause 8.6.6.4).
    Separation { 
        /// Name of the colorant.
        name: Arc<Vec<u8>>, 
        /// Alternate color space for overprint or non-separation devices.
        alternate: Box<ColorSpace>, 
        /// Tint transformation function.
        tint_transform: Object 
    },
    /// DeviceN color space (Clause 8.6.6.5).
    DeviceN { 
        /// Names of the colorants.
        names: Vec<Arc<Vec<u8>>>, 
        /// Alternate color space.
        alternate: Box<ColorSpace>, 
        /// Tint transformation function.
        tint_transform: Object 
    },
}

/// Represents an ICC Profile (ISO 32000-2:2020 Clause 8.6.5.5).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ICCProfile {
    /// Number of color components.
    pub components: u8,
    /// The raw profile data (uncompressed).
    pub data: Vec<u8>,
    /// Optional alternate color space if the profile is not supported.
    pub alternate: Option<Box<ColorSpace>>,
    /// Range of color components (default [0 1] for all).
    pub range: Vec<f32>,
    /// Metadata from the profile (e.g. desc).
    pub metadata: BTreeMap<String, String>,
}

impl ICCProfile {
    /// Parses the raw ICC profile data (ISO 15076-1).
    pub fn parse(data: &[u8], components: u8) -> PdfResult<Self> {
        if data.len() < 128 { 
            return Err(PdfError::ContentError(ContentErrorVariant::General("ICC profile too small".into()))); 
        }

        // Header check
        let magic = &data[36..40];
        if magic != b"acsp" {
            return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid ICC magic".into())));
        }

        let tag_count = u32::from_be_bytes(data[128..132].try_into().unwrap_or([0; 4]));
        let mut metadata = BTreeMap::new();

        // Basic tag parsing
        for i in 0..tag_count {
            let offset = 132 + (i as usize * 12);
            if offset + 12 > data.len() { break; }
            let sig = &data[offset..offset+4];
            let tag_offset = u32::from_be_bytes(data[offset+4..offset+8].try_into().unwrap_or([0; 4])) as usize;
            let tag_size = u32::from_be_bytes(data[offset+8..offset+12].try_into().unwrap_or([0; 4])) as usize;

            if tag_offset + tag_size <= data.len() {
                if sig == b"desc" {
                    // ASCII description is usually at offset + 12
                    let desc_data = &data[tag_offset..tag_offset + tag_size];
                    if desc_data.len() > 12 {
                         metadata.insert("description".into(), String::from_utf8_lossy(&desc_data[12..]).to_string());
                    }
                }
            }
        }

        Ok(Self {
            components,
            data: data.to_vec(),
            alternate: None,
            range: [0.0, 1.0].repeat(components as usize),
            metadata,
        })
    }
}

impl ColorSpace {
    /// Parses a color space object (Name or Array) with resilience.
    pub fn from_object(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Self> {
        let result = Self::from_object_internal(obj, resolver);
        match result {
            Ok(cs) => Ok(cs),
            Err(_e) => {
                Ok(ColorSpace::DeviceRGB)
            }
        }
    }

    fn from_object_internal(obj: &Object, resolver: &dyn Resolver) -> PdfResult<Self> {
        match obj {
            Object::Name(name) => match name.as_slice() {
                b"DeviceGray" | b"G" => Ok(ColorSpace::DeviceGray),
                b"DeviceRGB" | b"RGB" => Ok(ColorSpace::DeviceRGB),
                b"DeviceCMYK" | b"CMYK" => Ok(ColorSpace::DeviceCMYK),
                b"Pattern" => Ok(ColorSpace::Pattern),
                _ => {
                    Ok(ColorSpace::DeviceRGB)
                }
            },
            Object::Array(arr) => {
                if arr.is_empty() { return Err(PdfError::ContentError(ContentErrorVariant::General("Empty color space array".into()))); }
                let family = match &arr[0] {
                    Object::Name(n) => n.as_slice(),
                    _ => return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid color space family".into()))),
                };

                match family {
                    b"DeviceGray" | b"G" => Ok(ColorSpace::DeviceGray),
                    b"DeviceRGB" | b"RGB" => Ok(ColorSpace::DeviceRGB),
                    b"DeviceCMYK" | b"CMYK" => Ok(ColorSpace::DeviceCMYK),
                    b"CalGray" => parse_cal_gray(&arr[1], resolver),
                    b"CalRGB" => parse_cal_rgb(&arr[1], resolver),
                    b"Lab" => parse_lab(&arr[1], resolver),
                    b"ICCBased" => parse_icc_based(&arr[1], resolver),
                    b"Indexed" | b"I" => parse_indexed(arr, resolver),
                    b"Separation" => parse_separation(arr, resolver),
                    b"DeviceN" => parse_devicen(arr, resolver),
                    b"Pattern" => Ok(ColorSpace::Pattern),
                    _ => {
                        Ok(ColorSpace::DeviceRGB)
                    }
                }
            }
            Object::Reference(r) => Self::from_object(&resolver.resolve(r)?, resolver),
            _ => Err(PdfError::ContentError(ContentErrorVariant::General("Invalid color space object".into()))),
        }
    }

    /// Returns the number of color components for this color space.
    #[must_use] pub fn components(&self) -> u8 {
        match self {
            Self::DeviceGray | Self::CalGray { .. } => 1,
            Self::DeviceRGB | Self::CalRGB { .. } | Self::Lab { .. } => 3,
            Self::DeviceCMYK => 4,
            Self::ICCBased(profile) => profile.components,
            Self::Indexed { .. } => 1,
            Self::Pattern => 0, // Varies
            Self::Separation { .. } => 1,
            Self::DeviceN { names, .. } => names.len() as u8,
        }
    }

    /// Converts a color value in this color space to RGB (0.0 to 1.0).
    pub fn to_rgb(&self, components: &[f32]) -> [f32; 3] {
        match self {
            Self::DeviceGray | Self::CalGray { .. } => {
                let g = components.first().copied().unwrap_or(0.0);
                [g, g, g]
            }
            Self::DeviceRGB | Self::CalRGB { .. } => {
                if components.len() >= 3 {
                    [components[0], components[1], components[2]]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }
            Self::DeviceCMYK => {
                if components.len() >= 4 {
                    let (cyan, magenta, yellow, black) = (components[0], components[1], components[2], components[3]);
                    let red = (1.0 - cyan) * (1.0 - black);
                    let green = (1.0 - magenta) * (1.0 - black);
                    let blue = (1.0 - yellow) * (1.0 - black);
                    [red, green, blue]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }
            Self::ICCBased(profile) => {
                // Use lcms2 for conversion
                if let Ok(src_profile) = Profile::new_icc(&profile.data) {
                    let dst_profile = Profile::new_srgb();
                    let format = match profile.components {
                        1 => PixelFormat::GRAY_FLT,
                        3 => PixelFormat::RGB_FLT,
                        4 => PixelFormat::CMYK_FLT,
                        _ => PixelFormat::RGB_FLT,
                    };
                    if let Ok(t) = Transform::new(&src_profile, format, &dst_profile, PixelFormat::RGB_FLT, Intent::RelativeColorimetric) {
                        let mut output = [0.0f32; 3];
                        t.transform_pixels(components, &mut output);
                        return output;
                    }
                }
                // Fallback if ICC failed
                if components.len() >= 3 { [components[0], components[1], components[2]] } else { [0.0, 0.0, 0.0] }
            }
            Self::Lab { .. } => {
                 // Simplified Lab -> RGB for now
                 [0.5, 0.5, 0.5]
            }
            _ => [0.0, 0.0, 0.0],
        }
    }

    /// Returns true if this is a Pattern color space.
    #[must_use] pub fn is_pattern(&self) -> bool {
        matches!(self, Self::Pattern)
    }
}

// --- Internal Parsers ---

fn parse_cal_gray(obj: &Object, resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    let dict = resolve_dict(obj, resolver)?;
    let white_point = parse_f32_array(dict.get(b"WhitePoint".as_slice()), &[1.0, 1.0, 1.0])?;
    let black_point = parse_f32_array(dict.get(b"BlackPoint".as_slice()), &[0.0, 0.0, 0.0])?;
    let gamma = match dict.get(b"Gamma".as_slice()) {
        Some(Object::Real(r)) => *r as f32,
        Some(Object::Integer(i)) => *i as f32,
        _ => 1.0,
    };
    Ok(ColorSpace::CalGray { white_point, black_point, gamma })
}

fn parse_cal_rgb(obj: &Object, resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    let dict = resolve_dict(obj, resolver)?;
    let white_point = parse_f32_array(dict.get(b"WhitePoint".as_slice()), &[1.0, 1.0, 1.0])?;
    let black_point = parse_f32_array(dict.get(b"BlackPoint".as_slice()), &[0.0, 0.0, 0.0])?;
    let gamma = parse_f32_array(dict.get(b"Gamma".as_slice()), &[1.0, 1.0, 1.0])?;
    let matrix = parse_f32_array(dict.get(b"Matrix".as_slice()), &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])?;
    Ok(ColorSpace::CalRGB { white_point, black_point, gamma, matrix })
}

fn parse_lab(obj: &Object, resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    let dict = resolve_dict(obj, resolver)?;
    let white_point = parse_f32_array(dict.get(b"WhitePoint".as_slice()), &[1.0, 1.0, 1.0])?;
    let black_point = parse_f32_array(dict.get(b"BlackPoint".as_slice()), &[0.0, 0.0, 0.0])?;
    let range = parse_f32_array(dict.get(b"Range".as_slice()), &[-100.0, 100.0, -100.0, 100.0])?;
    Ok(ColorSpace::Lab { white_point, black_point, range })
}

fn parse_icc_based(obj: &Object, resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    let res = match obj {
        Object::Reference(r) => resolver.resolve(r)?,
        _ => obj.clone(),
    };
    let (dict, data) = if let Object::Stream(dict, data) = res {
        (dict, data)
    } else {
        return Err(PdfError::InvalidType { expected: "Stream".into(), found: "Other".into() });
    };
    
    let n = match dict.get(b"N".as_slice()) {
        Some(Object::Integer(i)) => *i as u8,
        _ => return Err(PdfError::ContentError(ContentErrorVariant::MissingRequiredKey("N"))),
    };

    let alternate = if let Some(alt_obj) = dict.get(b"Alternate".as_slice()) {
        Some(Box::new(ColorSpace::from_object(alt_obj, resolver)?))
    } else {
        None
    };

    let range = if let Some(Object::Array(arr)) = dict.get(b"Range".as_slice()) {
        let mut r = Vec::with_capacity(arr.len());
        for item in arr.iter() {
            r.push(match item { Object::Real(re) => *re as f32, Object::Integer(i) => *i as f32, _ => 0.0 });
        }
        r
    } else {
        let mut r = Vec::with_capacity((n * 2) as usize);
        for _ in 0..n { r.push(0.0); r.push(1.0); }
        r
    };

    let mut profile = ICCProfile::parse(&data, n)?;
    profile.alternate = alternate;
    profile.range = range;

    Ok(ColorSpace::ICCBased(Arc::new(profile)))
}

fn parse_indexed(arr: &[Object], resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    if arr.len() != 4 { return Err(PdfError::ContentError(ContentErrorVariant::General("Indexed color space must have 4 elements".into()))); }
    let base = ColorSpace::from_object(&arr[1], resolver)?;
    let hival = match &arr[2] {
        Object::Integer(i) => *i as u8,
        _ => return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid hival in Indexed".into()))),
    };
    let lookup = match &arr[3] {
        Object::String(s) => s.clone(),
        Object::Stream(dict, data) => {
            Arc::new(crate::filter::decode_stream(dict, data)?)
        }
        Object::Reference(r) => {
            let res = resolver.resolve(r)?;
            if let Object::String(s) = res { s } else { return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid lookup in Indexed".into()))); }
        }
        _ => return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid lookup type in Indexed".into()))),
    };

    Ok(ColorSpace::Indexed { base: Box::new(base), hival, lookup })
}

fn parse_separation(arr: &[Object], resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    if arr.len() < 4 { return Err(PdfError::ContentError(ContentErrorVariant::General("Separation color space must have 4 elements".into()))); }
    let name = match &arr[1] {
        Object::Name(n) => n.clone(),
        _ => return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid name in Separation".into()))),
    };
    let alternate = ColorSpace::from_object(&arr[2], resolver)?;
    let tint_transform = arr[3].clone();
    Ok(ColorSpace::Separation { name, alternate: Box::new(alternate), tint_transform })
}

fn parse_devicen(arr: &[Object], resolver: &dyn Resolver) -> PdfResult<ColorSpace> {
    if arr.len() < 4 { return Err(PdfError::ContentError(ContentErrorVariant::General("DeviceN color space must have at least 4 elements".into()))); }
    let names = match &arr[1] {
        Object::Array(a) => {
            let mut res = Vec::new();
            for item in a.iter() {
                if let Object::Name(n) = item { res.push(n.clone()); }
            }
            res
        }
        _ => return Err(PdfError::ContentError(ContentErrorVariant::General("Invalid names in DeviceN".into()))),
    };
    let alternate = ColorSpace::from_object(&arr[2], resolver)?;
    let tint_transform = arr[3].clone();
    Ok(ColorSpace::DeviceN { names, alternate: Box::new(alternate), tint_transform })
}

// --- Utilities ---

fn resolve_dict<'a>(obj: &'a Object, resolver: &'a dyn Resolver) -> PdfResult<&'a BTreeMap<Vec<u8>, Object>> {
    match obj {
        Object::Dictionary(d) => Ok(d),
        Object::Reference(r) => {
            let target_obj = resolver.resolve(r)?;
            if let Object::Dictionary(d) = Box::leak(Box::new(target_obj)) {
                Ok(d)
            } else {
                Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })
            }
        }
        _ => Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() }),
    }
}

fn parse_f32_array<const N: usize>(obj: Option<&Object>, default: &[f32; N]) -> PdfResult<[f32; N]> {
    if let Some(Object::Array(a)) = obj {
        if a.len() != N { return Ok(*default); }
        let mut res = [0.0; N];
        for (i, item) in a.iter().enumerate() {
            res[i] = match item {
                Object::Real(r) => *r as f32,
                Object::Integer(v) => *v as f32,
                _ => 0.0,
            };
        }
        Ok(res)
    } else {
        Ok(*default)
    }
}
