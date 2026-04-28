//! PDF Content Stream Intermediate Representation (IR).
//!
//! This module defines a structured, normalized representation of PDF drawing commands,
//! allowing downstream components to render or analyze documents without parsing
//! raw PostScript-style tokens.

use kurbo::{Affine, BezPath, Point};
use serde::{Deserialize, Serialize};
pub mod parser;
use crate::graphics::{Color, WindingRule, StrokeStyle, TextRenderingMode};
use crate::handle::Handle;
use crate::object::{Object, PdfName};
use crate::font::FontResource;

/// A high-level, normalized drawing command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // --- Graphics State ---
    /// Push the current graphics state onto the stack (q).
    PushState,
    /// Pop the graphics state from the stack (Q).
    PopState,
    /// Concatenate a transformation matrix (cm).
    Transform(Affine),
    
    // --- Path Construction ---
    /// Begin a new subpath (m).
    MoveTo(Point),
    /// Append a straight line segment (l).
    LineTo(Point),
    /// Append a cubic Bézier curve (c, v, y).
    CurveTo(Point, Point, Point),
    /// Close the current subpath (h).
    ClosePath,
    /// Append a rectangle (re).
    Rect(kurbo::Rect),

    // --- Painting ---
    /// Fill the current path (f, f*, F).
    Fill(WindingRule),
    /// Stroke the current path (S).
    Stroke(StrokeStyle),
    /// Fill and then stroke the current path (B, B*, b, b*).
    FillStroke(WindingRule, StrokeStyle),
    /// Use the current path as a clipping path (W, W*).
    Clip(WindingRule),

    // --- Text ---
    /// Begin a text object (BT).
    BeginText,
    /// End a text object (ET).
    EndText,
    /// Set the font and font size (Tf).
    SetFont {
        /// The resource name of the font (e.g., "F1").
        font: String,
        /// Optical size in points.
        size: f64,
    },
    /// Show a string of text (Tj, TJ).
    /// The string is GUARANTEED to be UTF-8 normalized.
    ShowText(String),
    /// Move the text position (Td, TD, T*).
    MoveText(Point),
    /// Set the text matrix (Tm).
    SetTextMatrix(Affine),
    /// Set the character spacing (Tc).
    SetCharSpacing(f64),
    /// Set the word spacing (Tw).
    SetWordSpacing(f64),
    /// Set the horizontal scaling (Tz).
    SetHorizontalScaling(f64),
    /// Set the text rendering mode (Tr).
    SetTextRenderMode(TextRenderingMode),
    /// Set the text rise (Ts).
    SetTextRise(f64),
    /// Set the writing mode (0 for horizontal, 1 for vertical).
    SetWritingMode(u8),

    // --- Color ---
    /// Set the fill color (sc, scn, g, rg, k).
    SetFillColor(Color),
    /// Set the stroke color (SC, SCN, G, RG, K).
    SetStrokeColor(Color),

    // --- XObjects & Images ---
    /// Draw an external object (Do).
    DrawXObject(String),
    /// Define an inline image (BI...ID...EI).
    DrawInlineImage {
        width: u32,
        height: u32,
        format: crate::graphics::PixelFormat,
        data: Vec<u8>,
    },

    // --- Compatibility & Extensions ---
    /// Begin a marked-content sequence (BMC, BDC).
    BeginMarkedContent {
        tag: PdfName,
        properties: Option<String>, // Tag name or resource name
    },
    /// End a marked-content sequence (EMC).
    EndMarkedContent,
    
    /// A raw PDF operator that could not be sublimated.
    RawOperator {
        name: String,
        operands: Vec<Object>,
    },
}
