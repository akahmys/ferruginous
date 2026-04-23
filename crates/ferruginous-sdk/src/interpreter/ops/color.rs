use crate::interpreter::Interpreter;
use ferruginous_core::PdfResult;
use ferruginous_core::graphics::Color;

impl Interpreter<'_> {
    pub(crate) fn handle_color_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "g" => {
                let gray = self.pop_f64()?;
                let c = Color::Gray(gray);
                self.state.fill_color = c;
                self.backend.set_fill_color(c);
            }
            "G" => {
                let gray = self.pop_f64()?;
                let c = Color::Gray(gray);
                self.state.stroke_color = c;
                self.backend.set_stroke_color(c);
            }
            "rg" => {
                let b = self.pop_f64()?;
                let g = self.pop_f64()?;
                let r = self.pop_f64()?;
                let c = Color::Rgb(r, g, b);
                self.state.fill_color = c;
                self.backend.set_fill_color(c);
            }
            "RG" => {
                let b = self.pop_f64()?;
                let g = self.pop_f64()?;
                let r = self.pop_f64()?;
                let c = Color::Rgb(r, g, b);
                self.state.stroke_color = c;
                self.backend.set_stroke_color(c);
            }
            "k" => {
                let k = self.pop_f64()?;
                let y = self.pop_f64()?;
                let m = self.pop_f64()?;
                let c = self.pop_f64()?;
                let col = Color::Cmyk(c, m, y, k);
                self.state.fill_color = col;
                self.backend.set_fill_color(col);
            }
            "K" => {
                let k = self.pop_f64()?;
                let y = self.pop_f64()?;
                let m = self.pop_f64()?;
                let c = self.pop_f64()?;
                let col = Color::Cmyk(c, m, y, k);
                self.state.stroke_color = col;
                self.backend.set_stroke_color(col);
            }
            _ => {}
        }
        Ok(())
    }
}
