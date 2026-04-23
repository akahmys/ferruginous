use crate::interpreter::Interpreter;
use ferruginous_core::{PdfResult, PdfName, Object, Matrix};

impl Interpreter<'_> {
    #[allow(clippy::many_single_char_names)]
    pub(crate) fn handle_state_operator(&mut self, op: &str) -> PdfResult<()> {
        match op {
            "q" => { self.state_stack.push(self.state.clone()); self.backend.push_state(); }
            "Q" => { if let Some(old) = self.state_stack.pop() { self.state = old; self.backend.pop_state(); }}
            "cm" => {
                let f = self.pop_f64()?; let e = self.pop_f64()?;
                let d = self.pop_f64()?; let c = self.pop_f64()?;
                let b = self.pop_f64()?; let a = self.pop_f64()?;
                let m = Matrix::new(a, b, c, d, e, f);
                self.state.ctm = self.state.ctm.concat(&m);
                self.backend.transform(m.as_affine());
            }
            "gs" => { let name = self.pop_name()?; self.handle_gs_operator(&name)?; }
            _ => {}
        }
        Ok(())
    }

    fn handle_gs_operator(&mut self, name: &PdfName) -> PdfResult<()> {
        let entry = self.find_resource(&self.doc.arena().intern_name(PdfName::new("ExtGState")), name)?;
        let gs_obj = entry.resolve(self.doc.arena());
        if let Object::Dictionary(h) = gs_obj
            && let Some(gs_dict) = self.doc.arena().get_dict(h) {
                let ca_key = self.doc.arena().intern_name(PdfName::new("ca"));
                let ca_up_key = self.doc.arena().intern_name(PdfName::new("CA"));
                
                if let Some(ca) = gs_dict.get(&ca_key).and_then(|o| o.as_f64()) {
                    self.state.fill_alpha = ca;
                    self.backend.set_fill_alpha(ca);
                }
                if let Some(ca_up) = gs_dict.get(&ca_up_key).and_then(|o| o.as_f64()) {
                    self.state.stroke_alpha = ca_up;
                    self.backend.set_stroke_alpha(ca_up);
                }
        }
        Ok(())
    }
}
