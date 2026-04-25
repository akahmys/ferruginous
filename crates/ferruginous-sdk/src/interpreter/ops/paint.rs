use crate::interpreter::Interpreter;
use ferruginous_core::PdfResult;
use ferruginous_core::graphics::WindingRule;
use ferruginous_render::path::PathBuilder;

impl Interpreter<'_> {
    pub(crate) fn handle_painting_operator(&mut self, op: &str) -> PdfResult<()> {
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
            "f" | "F" => {
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::NonZero);
            }
            "f*" => {
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::EvenOdd);
            }
            "b" => {
                self.path.close_path();
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::NonZero);
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "b*" => {
                self.path.close_path();
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::EvenOdd);
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "B" => {
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::NonZero);
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "B*" => {
                let p = self.path.clone().finish();
                self.backend.fill_path(&p, &self.state.fill_color, WindingRule::EvenOdd);
                self.backend.stroke_path(&p, &self.state.stroke_color, &self.state.stroke_style);
            }
            "n" => {}
            _ => {}
        }

        // Painting operators discard the path
        self.path = PathBuilder::new();

        if let Some(p) = p_for_clip
            && let Some(rule) = self.pending_clip.take()
        {
            self.backend.push_clip(&p, rule);
        }

        Ok(())
    }
}
