use crate::interpreter::Interpreter;
use ferruginous_core::{Handle, Object, PdfError, PdfName, PdfResult};
use std::collections::BTreeMap;

impl Interpreter<'_> {
    pub(crate) fn handle_xobject_operator(&mut self) -> PdfResult<()> {
        let name = self.pop_name()?;
        let entry =
            self.find_resource(&self.doc.arena().intern_name(PdfName::new("XObject")), &name)?;
        let xobj = entry.resolve(self.doc.arena());
        if let Object::Stream(dh, data) = xobj
            && let Some(dict) = self.doc.arena().get_dict(dh)
        {
            let subtype_key = self.doc.arena().intern_name(PdfName::new("Subtype"));
            if let Some(sub) =
                dict.get(&subtype_key).and_then(|o| o.resolve(self.doc.arena()).as_name())
            {
                let sub_name = self
                    .doc
                    .arena()
                    .get_name(sub)
                    .ok_or_else(|| PdfError::Other("Subtype name not found".into()))?;
                let sub_str = sub_name.as_str();
                match sub_str {
                    "Image" => self.render_image_xobject(&dict, &data)?,
                    "Form" => self.render_form_xobject(&dict, &data)?,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::many_single_char_names)]
    pub(crate) fn render_form_xobject(
        &mut self,
        dict: &BTreeMap<Handle<PdfName>, Object>,
        data: &[u8],
    ) -> PdfResult<()> {
        // 1. Save state
        self.state_stack.push(self.state.clone());
        self.backend.push_state();

        // 2. Apply Matrix
        let matrix_key = self.doc.arena().intern_name(PdfName::new("Matrix"));
        if let Some(Object::Array(h)) = dict.get(&matrix_key).map(|o| o.resolve(self.doc.arena()))
            && let Some(arr) = self.doc.arena().get_array(h)
            && arr.len() == 6
        {
            let a = arr[0].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let b = arr[1].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let c = arr[2].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let d = arr[3].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let e = arr[4].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let f = arr[5].resolve(self.doc.arena()).as_f64().unwrap_or(0.0);
            let m = ferruginous_core::graphics::Matrix::new(a, b, c, d, e, f);
            self.state.ctm = self.state.ctm.concat(&m);
            self.backend.transform(m.as_affine());
        }

        // 3. Setup Resources
        let mut pushed = false;
        let res_key = self.doc.arena().intern_name(PdfName::new("Resources"));
        if let Some(Object::Dictionary(h)) = dict.get(&res_key).map(|o| o.resolve(self.doc.arena()))
        {
            self.resource_stack.push(h);
            pushed = true;
        }

        // 4. Recursive Execute
        self.execute(data)?;

        // 5. Cleanup
        if pushed {
            self.resource_stack.pop();
        }
        if let Some(old) = self.state_stack.pop() {
            self.state = old;
            self.backend.pop_state();
        }

        Ok(())
    }

    pub(crate) fn render_image_xobject(
        &mut self,
        dict: &BTreeMap<Handle<PdfName>, Object>,
        data: &[u8],
    ) -> PdfResult<()> {
        let width_key = self.doc.arena().intern_name(PdfName::new("Width"));
        let height_key = self.doc.arena().intern_name(PdfName::new("Height"));
        let w = u32::try_from(
            dict.get(&width_key)
                .and_then(|o| o.resolve(self.doc.arena()).as_integer())
                .unwrap_or(0),
        )
        .unwrap_or(0);
        let h = u32::try_from(
            dict.get(&height_key)
                .and_then(|o| o.resolve(self.doc.arena()).as_integer())
                .unwrap_or(0),
        )
        .unwrap_or(0);
        let decoded = self.doc.arena().process_filters(data, dict)?;
        self.backend.draw_image(&decoded, w, h, ferruginous_core::graphics::PixelFormat::Rgb8);
        Ok(())
    }
}
