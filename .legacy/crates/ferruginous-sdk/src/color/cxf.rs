//! CxF (Color Exchange Format) parsing for spectral color simulation.
//! (ISO 17972-1:2015 / ISO 32000-2:2020)

use roxmltree::Document;

/// Represents a color sample extracted from CxF data.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CxfSample {
    /// The name of the color (e.g., PANTONE 185 C).
    pub name: String,
    /// CIELab values if available.
    pub lab: Option<[f32; 3]>,
}

/// Simple parser for CxF/X-4 (ISO 17972-4) data.
/// Used for high-precision spot color simulation in PDF/X-6.
pub struct CxfParser;

impl CxfParser {
    /// Parses CxF XML and returns a list of samples.
    ///
    /// # Errors
    /// Returns an empty vector if the XML is malformed or no samples are found.
    pub fn parse(xml: &str) -> Vec<CxfSample> {
        let doc = match Document::parse(xml) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut samples = Vec::new();
        
        // Find ColorSamples or similar entries depending on CxF version/profile
        // PDF/X-6 uses CxF/X-4 (ISO 17972-4)
        for node in doc.descendants() {
            if node.has_tag_name("ColorSample") {
                let name = node.attribute("Name").unwrap_or("Unknown").to_string();
                let mut lab = None;
                
                // Look for CIELab values (D50/2 degree is the standard for CxF/X-4)
                for child in node.descendants() {
                    if child.has_tag_name("ColorCIELab") {
                        let l = child.children().find(|n| n.has_tag_name("L")).and_then(|n| n.text()?.trim().parse::<f32>().ok());
                        let a = child.children().find(|n| n.has_tag_name("A")).and_then(|n| n.text()?.trim().parse::<f32>().ok());
                        let b = child.children().find(|n| n.has_tag_name("B")).and_then(|n| n.text()?.trim().parse::<f32>().ok());
                        
                        if let (Some(l), Some(a), Some(b)) = (l, a, b) {
                            lab = Some([l, a, b]);
                            break;
                        }
                    }
                }
                
                samples.push(CxfSample { name, lab });
            }
        }
        
        samples
    }
}
