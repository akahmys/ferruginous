use crate::interpreter::Interpreter;
use ferruginous_core::graphics::{BlendMode, Color, PixelFormat, StrokeStyle, WindingRule};
use ferruginous_core::{Document, Handle, Object, PdfArena, PdfName, PdfResult};
use ferruginous_render::RenderBackend;
use kurbo::{Affine, BezPath};
use std::collections::BTreeMap;

/// A single span of text with its associated styling and positioning.
#[derive(Debug, Clone)]
pub struct TextSpan {
    /// The decoded text content.
    pub text: String,
    /// The optical font size in points.
    pub font_size: f64,
    /// Whether the text is inferred as bold.
    pub is_bold: bool,
    /// X coordinate in PDF space.
    pub x: f64,
    /// Y coordinate in PDF space.
    pub y: f64,
    /// The width of the text span.
    pub width: f64,
}

/// A backend that extracts text content from a page.
pub struct TextExtractionBackend {
    output: String,
    last_y: f64,
}

impl TextExtractionBackend {
    /// Creates a new text extraction backend.
    pub fn new() -> Self {
        Self { output: String::new(), last_y: 0.0 }
    }

    /// Returns the accumulated text and consumes the backend.
    pub fn finish(self) -> String {
        self.output
    }
}

impl Default for TextExtractionBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for TextExtractionBackend {
    fn push_state(&mut self) {}
    fn pop_state(&mut self) {}
    fn transform(&mut self, _affine: Affine) {}
    fn fill_path(&mut self, _path: &BezPath, _color: &Color, _rule: WindingRule) {}
    fn stroke_path(&mut self, _path: &BezPath, _color: &Color, _style: &StrokeStyle) {}
    fn push_clip(&mut self, _path: &BezPath, _rule: WindingRule) {}
    fn pop_clip(&mut self) {}
    fn draw_image(&mut self, _data: &[u8], _width: u32, _height: u32, _format: PixelFormat) {}
    fn set_fill_alpha(&mut self, _alpha: f64) {}
    fn set_stroke_alpha(&mut self, _alpha: f64) {}
    fn set_blend_mode(&mut self, _mode: ferruginous_core::graphics::BlendMode) {}
    fn set_fill_color(&mut self, _color: Color) {}
    fn set_stroke_color(&mut self, _color: Color) {}
    fn define_font(
        &mut self,
        _name: &str,
        _base: Option<&str>,
        _data: Option<std::sync::Arc<Vec<u8>>>,
        _index: Option<usize>,
        _cid_to_gid_map: Option<Vec<u16>>,
    ) {
    }
    fn set_font(&mut self, _name: &str) {}
    fn show_text(
        &mut self,
        _glyphs: &[(u32, f32, f32, f32, u32)],
        text: &str,
        _size: f64,
        transform: Affine,
        _tc: f64,
        _tw: f64,
        _vertical: bool,
    ) {
        let coeffs = transform.as_coeffs();
        let y = coeffs[5];
        if (y - self.last_y).abs() > 5.0 && !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output.push_str(text);
        self.last_y = y;
    }
    fn set_text_render_mode(&mut self, _mode: ferruginous_core::graphics::TextRenderingMode) {}
    fn set_char_spacing(&mut self, _spacing: f64) {}
    fn set_word_spacing(&mut self, _spacing: f64) {}
}

/// A backend that collects text spans for analysis instead of rendering them.
struct CollectorBackend {
    spans: Vec<TextSpan>,
    fonts: BTreeMap<String, String>, // Name -> BaseFont
    current_font: Option<String>,
}

impl CollectorBackend {
    fn new() -> Self {
        Self {
            spans: Vec::new(),
            fonts: BTreeMap::new(),
            current_font: None,
        }
    }
}

impl RenderBackend for CollectorBackend {
    fn set_text_render_mode(&mut self, _mode: ferruginous_core::graphics::TextRenderingMode) {}
    fn set_char_spacing(&mut self, _spacing: f64) {}
    fn set_word_spacing(&mut self, _spacing: f64) {}
    fn set_fill_color(&mut self, _color: Color) {}
    fn set_stroke_color(&mut self, _color: Color) {}
    fn set_fill_alpha(&mut self, _alpha: f64) {}
    fn set_stroke_alpha(&mut self, _alpha: f64) {}
    fn set_blend_mode(&mut self, _mode: BlendMode) {}
    fn transform(&mut self, _matrix: Affine) {}
    fn push_state(&mut self) {}
    fn pop_state(&mut self) {}
    fn push_clip(&mut self, _path: &BezPath, _rule: WindingRule) {}
    fn pop_clip(&mut self) {}
    fn draw_image(&mut self, _data: &[u8], _w: u32, _h: u32, _fmt: PixelFormat) {}
    fn fill_path(&mut self, _path: &BezPath, _color: &Color, _rule: WindingRule) {}
    fn stroke_path(&mut self, _path: &BezPath, _color: &Color, _style: &StrokeStyle) {}

    fn set_font(&mut self, name: &str) {
        self.current_font = Some(name.to_string());
    }

    fn show_text(
        &mut self,
        glyphs: &[(u32, f32, f32, f32, u32)],
        text: &str,
        size: f64,
        transform: Affine,
        _tc: f64,
        _tw: f64,
        _vertical: bool,
    ) {
        let coeffs = transform.as_coeffs();
        let mut is_bold = false;
        let mut width = 0.0;

        if let Some(font_name) = &self.current_font
            && let Some(base_font) = self.fonts.get(font_name) {
                let name_lower = base_font.to_lowercase();
                is_bold = name_lower.contains("bold")
                    || name_lower.contains("heavy")
                    || name_lower.contains("black");
            }

        // Calculate width from glyph advances
        for (_, advance, _, _, _) in glyphs {
            #[allow(clippy::cast_possible_truncation)]
            let adv_scaled = f64::from(advance * (size as f32) / 1000.0);
            width += adv_scaled;
        }

        self.spans.push(TextSpan {
            text: text.to_string(),
            font_size: size * coeffs[3],
            is_bold,
            x: coeffs[4],
            y: coeffs[5],
            width,
        });
    }

    fn define_font(
        &mut self,
        name: &str,
        base: Option<&str>,
        _data: Option<std::sync::Arc<Vec<u8>>>,
        _index: Option<usize>,
        _cid_to_gid_map: Option<Vec<u16>>,
    ) {
        if let Some(base_name) = base {
            self.fonts.insert(name.to_string(), base_name.to_string());
        }
    }
}

/// Represents a proposed structural change to the PDF.
#[derive(Debug, Clone)]
pub struct RemediationCandidate {
    /// Description of the proposed change.
    pub description: String,
    /// The page index where this remediation applies.
    pub page_index: usize,
    /// The type of remediation action.
    pub action_type: RemediationActionType,
}

/// Types of structural remediations inferred by the engine.
#[derive(Debug, Clone)]
pub enum RemediationActionType {
    /// Promote a text span to a heading.
    SetHeading {
        /// Inferred text content.
        text: String,
        /// Inferred heading level (1-6).
        level: u8,
    },
    /// Organize a set of spans into a table.
    CreateTable {
        /// Number of inferred rows.
        rows: usize,
    },
    /// Cluster a set of spans into a paragraph.
    ClusterParagraphs {
        /// Number of spans in the cluster.
        count: usize,
    },
}

/// Inference engine for structural remediation.
pub struct HeuristicEngine<'a> {
    arena: &'a PdfArena,
}

impl<'a> HeuristicEngine<'a> {
    /// Creates a new heuristic engine for the given arena.
    pub fn new(arena: &'a PdfArena) -> Self {
        Self { arena }
    }

    /// Primary entry point for re-tagging a document.
    pub fn infer_structure(&self, doc: &Document) -> PdfResult<Vec<RemediationCandidate>> {
        let mut candidates = Vec::new();
        let page_count = doc.page_count()?;

        for i in 0..page_count {
            let page = doc.get_page(i)?;
            let mut collector = CollectorBackend::new();

            let res_handle: Handle<BTreeMap<Handle<PdfName>, Object>> = page
                .resolve_attribute("Resources")
                .and_then(|o| o.as_dict_handle())
                .unwrap_or_else(|| {
                    self.arena.alloc_dict(BTreeMap::<Handle<PdfName>, Object>::new())
                });

            let mut interpreter =
                Interpreter::new(&mut collector, doc, res_handle, kurbo::Affine::IDENTITY);

            if let Some(contents) = page.resolve_attribute("Contents") {
                let data = doc.decode_stream(&contents)?;
                let _ = interpreter.execute(&data);
            }

            candidates.extend(self.detect_headings(i, &collector.spans)?);
            candidates.extend(self.detect_tables(i, &collector.spans)?);
            candidates.extend(self.cluster_paragraphs(i, &collector.spans)?);
        }

        Ok(candidates)
    }

    /// Detects potential H1-H6 headings based on font metrics and positioning.
    #[allow(clippy::cast_possible_truncation)]
    fn detect_headings(
        &self,
        page_index: usize,
        spans: &[TextSpan],
    ) -> PdfResult<Vec<RemediationCandidate>> {
        let mut candidates = Vec::new();
        if spans.is_empty() {
            return Ok(candidates);
        }

        // 1. Calculate frequency of font sizes to find "Body" text
        let mut size_counts: BTreeMap<i32, usize> = BTreeMap::new();
        for span in spans {
            let s = (span.font_size * 10.0) as i32;
            *size_counts.entry(s).or_default() += 1;
        }

        let body_size = size_counts.iter().max_by_key(|&(_, count)| count).map_or(120, |(&s, _)| s);

        // 2. Collect all heading sizes (larger than body)
        let mut heading_sizes: Vec<i32> = size_counts.keys().filter(|&&s| s > body_size + 15).copied().collect();
        heading_sizes.sort_by(|a, b| b.cmp(a)); // Descending order: largest first

        for span in spans {
            let s = (span.font_size * 10.0) as i32;
            if let Some(rank) = heading_sizes.iter().position(|&hs| hs == s) {
                let level = (rank + 1).min(6) as u8;
                candidates.push(RemediationCandidate {
                    description: format!(
                        "Set '{}' as Heading Level {} ({}pt{})",
                        span.text.trim(),
                        level,
                        span.font_size,
                        if span.is_bold { ", Bold" } else { "" }
                    ),
                    page_index,
                    action_type: RemediationActionType::SetHeading {
                        text: span.text.clone(),
                        level,
                    },
                });
            } else if span.is_bold && s >= body_size - 5 {
                // Bold text near body size -> H6 or Strong
                candidates.push(RemediationCandidate {
                    description: format!(
                        "Set '{}' as Heading Level 6 (Bold Body)",
                        span.text.trim()
                    ),
                    page_index,
                    action_type: RemediationActionType::SetHeading {
                        text: span.text.clone(),
                        level: 6,
                    },
                });
            }
        }
        Ok(candidates)
    }

    /// Detects potential table structures based on grid alignment.
    #[allow(clippy::cast_possible_truncation)]
    fn detect_tables(
        &self,
        page_index: usize,
        spans: &[TextSpan],
    ) -> PdfResult<Vec<RemediationCandidate>> {
        let mut candidates = Vec::new();
        if spans.len() < 4 {
            return Ok(candidates);
        }

        // 1. Group spans into "Lines" (near-identical Y)
        let mut lines: BTreeMap<i32, Vec<&TextSpan>> = BTreeMap::new();
        for span in spans {
            let y = (span.y * 10.0) as i32;
            lines.entry(y).or_default().push(span);
        }

        // 2. Look for "Columnar Consistency"
        // If multiple lines have similar X-offsets, it's likely a table.
        let mut potential_table_rows = 0;
        let mut prev_cols = Vec::new();

        for (_y, line_spans) in lines {
            let mut current_cols: Vec<i32> =
                line_spans.iter().map(|s| (s.x * 10.0) as i32).collect();
            current_cols.sort_unstable();

            if current_cols.len() > 1 && !prev_cols.is_empty() {
                // Check if current cols match prev cols
                let mut matches = 0;
                for &c in &current_cols {
                    for &pc in &prev_cols {
                        if i32::abs(pc - c) < 50 {
                            // 5pt tolerance
                            matches += 1;
                            break;
                        }
                    }
                }
                if matches > 1 {
                    potential_table_rows += 1;
                }
            }
            prev_cols = current_cols;
        }

        if potential_table_rows > 2 {
            candidates.push(RemediationCandidate {
                description: format!("Form table from {potential_table_rows} aligned rows"),
                page_index,
                action_type: RemediationActionType::CreateTable {
                    rows: usize::try_from(potential_table_rows).unwrap_or(0),
                },
            });
        }

        Ok(candidates)
    }

    /// Groups adjacent text blocks into Paragraph (P) elements.
    fn cluster_paragraphs(
        &self,
        page_index: usize,
        spans: &[TextSpan],
    ) -> PdfResult<Vec<RemediationCandidate>> {
        let mut candidates = Vec::new();
        // Group spans with small Y-diff and similar X-start
        let mut paragraphs = 0;
        let mut last_y = 0.0;

        for span in spans {
            if (span.y - last_y).abs() > 20.0 {
                // New paragraph gap
                paragraphs += 1;
            }
            last_y = span.y;
        }

        if paragraphs > 0 {
            candidates.push(RemediationCandidate {
                description: format!("Cluster {paragraphs} text blocks into paragraphs"),
                page_index,
                action_type: RemediationActionType::ClusterParagraphs {
                    count: usize::try_from(paragraphs).unwrap_or(0),
                },
            });
        }
        Ok(candidates)
    }
}

/// Helper function to perform structural re-tagging on a document.
pub fn retag(doc: &mut Document) -> PdfResult<()> {
    let engine = HeuristicEngine::new(doc.arena());
    let candidates = engine.infer_structure(doc)?;

    for candidate in candidates {
        println!("Automatically applying: {}", candidate.description);
        // STUB: Real application logic would happen here
    }

    Ok(())
}
