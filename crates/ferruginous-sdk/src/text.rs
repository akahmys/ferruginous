use kurbo::Affine;
use crate::core::Object;

/// ISO 32000-2:2020 Clause 9.3 - Text State Parameters
/// Manages font, font size, matrices, and spacing for text operations.
#[derive(Debug, Clone, PartialEq)]
pub struct TextState {
    /// Tc - Character spacing (Clause 9.3.2).
    pub char_spacing: f64,
    /// Tw - Word spacing (Clause 9.3.3).
    pub word_spacing: f64,
    /// Th - Horizontal scaling (Clause 9.3.4).
    pub horizontal_scaling: f64,
    /// Tl - Leading (Clause 9.3.5).
    pub leading: f64,
    /// Tf - Current font reference (Clause 9.3.6).
    pub font: Option<Object>,
    /// Tfs - Current font size (Clause 9.3.6).
    pub font_size: f64,
    /// Tr - Rendering mode (Clause 9.3.7).
    pub rendering_mode: i32,
    /// Ts - Text rise (Clause 9.3.8).
    pub text_rise: f64,
    /// WMode - Writing mode (0 for horizontal, 1 for vertical).
    pub wmode: u8,
    
    /// Tm - Current text matrix (Clause 9.4.2).
    pub matrix: Affine,
    /// Tlm - Current text line matrix (Clause 9.4.2).
    pub line_matrix: Affine,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            font: None,
            font_size: 0.0,
            rendering_mode: 0,
            text_rise: 0.0,
            wmode: 0,
            matrix: Affine::IDENTITY,
            line_matrix: Affine::IDENTITY,
        }
    }
}

impl TextState {
    /// BT - Clause 9.4.2 (Begin Text)
    pub fn begin_text(&mut self) {
        self.matrix = Affine::IDENTITY;
        self.line_matrix = Affine::IDENTITY;
    }

    /// Td - Clause 9.4.2 (Move Text)
    pub fn move_text(&mut self, tx: f64, ty: f64) {
        let translation = Affine::translate((tx, ty));
        self.line_matrix = self.line_matrix * translation;
        self.matrix = self.line_matrix;
    }

    /// Tm - Clause 9.4.2 (Set Text Matrix)
    #[allow(clippy::many_single_char_names)]
    pub fn set_matrix(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.matrix = Affine::new([a, b, c, d, e, f]);
        self.line_matrix = self.matrix;
    }

    /// Clause 9.4.4 - Text Rendering Matrix and Cursor Advancement
    /// Returns the shift value (tx' or ty') and the matrix *before* advancement.
    pub fn advance_glyph(&mut self, is_space: bool, glyph_width: f64, tj_adj: f64) -> (f64, Affine) {
        let fs = self.font_size;
        let tc = self.char_spacing;
        let tw = if is_space { self.word_spacing } else { 0.0 };
        let th = self.horizontal_scaling / 100.0;
        
        let matrix_before = self.matrix;
        let shift = ((glyph_width - tj_adj) / 1000.0).mul_add(fs, tc) + tw;

        if self.wmode == 0 {
            // Horizontal: tx = shift * th
            let tx = shift * th;
            let coeffs = self.matrix.as_coeffs();
            self.matrix = Affine::new([
                coeffs[0], coeffs[1], coeffs[2], coeffs[3], 
                coeffs[4] + tx * coeffs[0], 
                coeffs[5] + tx * coeffs[1]
            ]);
            (tx, matrix_before)
        } else {
            // Vertical: ty = -shift (writing downwards)
            let ty = -shift;
            let coeffs = self.matrix.as_coeffs();
            self.matrix = Affine::new([
                coeffs[0], coeffs[1], coeffs[2], coeffs[3], 
                coeffs[4] + ty * coeffs[2], 
                coeffs[5] + ty * coeffs[3]
            ]);
            (ty, matrix_before)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_matrix_ops() {
        let mut ts = TextState::default();
        ts.begin_text();
        ts.move_text(10.0, 20.0);
        let c = ts.matrix.as_coeffs();
        assert!((c[4] - 10.0).abs() < f64::EPSILON);
        assert!((c[5] - 20.0).abs() < f64::EPSILON);
        
        ts.move_text(5.0, 5.0);
        let c = ts.matrix.as_coeffs();
        assert!((c[4] - 15.0).abs() < f64::EPSILON);
        assert!((c[5] - 25.0).abs() < f64::EPSILON);
    }
}
