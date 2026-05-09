use crate::interpreter::Interpreter;
use ferruginous_core::PdfResult;
use ferruginous_core::graphics::WindingRule;
use ferruginous_render::path::PathBuilder;

impl Interpreter<'_> {
    pub(crate) fn handle_painting_operator(&mut self, op: &str) -> PdfResult<()> {
        let rule = match op {
            "f" | "F" | "B" | "b" => {
                if self.in_type3_glyph {
                    WindingRule::EvenOdd
                } else {
                    WindingRule::NonZero
                }
            }
            "f*" | "B*" | "b*" => WindingRule::EvenOdd,
            _ => WindingRule::NonZero,
        };

        let p_for_clip =
            if self.pending_clip.is_some() { Some(self.path.clone().finish()) } else { None };

        match op {
            "S" => {
                let p = self.path.clone().finish();
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "s" => {
                self.path.close_path();
                let p = self.path.clone().finish();
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "f" | "F" | "f*" => {
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, rule);
            }
            "B" | "B*" | "b" | "b*" => {
                if op.starts_with('b') {
                    self.path.close_path();
                }
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, rule);
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "n" => {} // End path without filling or stroking
            _ => {}
        }

        if let Some(p) = p_for_clip
            && let Some(rule) = self.pending_clip.take()
        {
            self.backend.push_clip(&p, rule);
            self.state.clip_count += 1;
        }

        self.path = PathBuilder::new();
        Ok(())
    }
}
