use crate::interpreter::Interpreter;
use ferruginous_core::font::FontResource;
use ferruginous_core::{Handle, Object, PdfError, PdfName, PdfResult};
use std::collections::BTreeMap;
use std::sync::Arc;

impl Interpreter<'_> {
    pub(crate) fn resolve_font_resource(&mut self, name: &PdfName) -> PdfResult<Arc<FontResource>> {
        let entry =
            self.find_resource(&self.doc.arena().intern_name(PdfName::new("Font")), name)?;
        let h =
            entry.as_reference().unwrap_or_else(|| self.doc.arena().alloc_object(entry.clone()));
        self.get_font(h)
    }

    pub(crate) fn get_font(&mut self, h: Handle<Object>) -> PdfResult<Arc<FontResource>> {
        if let Some(cached) = self.font_cache.get(&h) {
            return Ok(Arc::clone(cached));
        }

        let arena = self.doc.arena();
        let font_obj = self.doc.resolve(&h)?;

        let mut res = if let Object::Dictionary(dfh) = font_obj {
            let dict = arena
                .get_dict(dfh)
                .ok_or_else(|| PdfError::Other("Invalid font dictionary".into()))?;
            let mut initial_res = FontResource::load(&dict, self.doc)?;

            // Handle Type0 DescendantFonts
            if initial_res.subtype.as_str() == "Type0"
                && let Some(desc_fonts_obj) = dict.get(&arena.name("DescendantFonts"))
                && let Object::Array(ah) = desc_fonts_obj.resolve(arena)
                && let Some(arr) = arena.get_array(ah)
                && let Some(desc_font) = arr.first()
                && let Object::Dictionary(dfh) = desc_font.resolve(arena)
                && let Some(df_dict) = arena.get_dict(dfh)
            {
                let mut desc_res = FontResource::load(&df_dict, self.doc)?;
                // Propagate Encoding, ToUnicode, and WMode from Type0 parent
                desc_res.encoding.clone_from(&initial_res.encoding);
                desc_res.wmode = initial_res.wmode;
                if desc_res.to_unicode.is_none() {
                    desc_res.to_unicode.clone_from(&initial_res.to_unicode);
                }
                initial_res = desc_res;
            }
            Arc::new(initial_res)
        } else {
            return Err(PdfError::Other("Font dictionary not found".into()));
        };

        // Update global rescue CMap if this font has a superior one (either ToUnicode or Encoding)
        let mut candidate_map = res.to_unicode.as_ref();
        if let Some(ref enc) = res.encoding
            && candidate_map.is_none_or(|m| enc.mappings.len() > m.mappings.len())
        {
            candidate_map = Some(enc);
        }

        if let Some(m) = candidate_map {
            let current_max = self.global_rescue_cmap.as_ref().map_or(0, |m| m.mappings.len());
            if m.mappings.len() > current_max {
                self.global_rescue_cmap = Some(m.clone());
            }
        }

        // CMAP RESCUE: Only apply if the font LACKS any useful mapping.
        let is_named = !res.base_font.as_str().is_empty() && res.base_font.as_str() != "Untitled";
        let is_cjk = res.subtype.as_str() == "Type0"
            || res.encoding.as_ref().is_some_and(|e| e.is_multibyte());
        let is_western =
            ["Century", "Arial", "Times", "Helvetica", "Courier", "Symbol", "ZapfDingbats"]
                .iter()
                .any(|&name| res.base_font.as_str().contains(name));

        if !res.has_any_mapping()
            && (is_named || is_cjk)
            && !is_western
            && let Some(parent_cmap) =
                self.find_rescue_cmap(&res).or_else(|| self.global_rescue_cmap.clone())
        {
            let mut res_mut = (*res).clone();
            res_mut.to_unicode = Some(parent_cmap);
            res = Arc::new(res_mut);
        }

        // Save to cache before defining in backend
        self.font_cache.insert(h, Arc::clone(&res));

        // Register font with backend if data is available
        let name = self.font_name_map.get(&h).cloned().unwrap_or_else(|| "UnknownFont".to_string());
        if !self.defined_fonts.contains(name.as_str()) {
            self.backend.define_font(
                name.as_str(),
                Some(res.base_font.as_str()),
                res.data.clone(),
                None,
                res.cid_to_gid_map.clone(),
            );
            self.defined_fonts.insert(name.as_str().to_string());
        }

        Ok(res)
    }

    pub(crate) fn scan_for_global_rescue_cmap(
        &mut self,
        res_h: Handle<BTreeMap<Handle<PdfName>, Object>>,
    ) {
        let arena = self.doc.arena();
        let font_type_key = arena.intern_name(PdfName::new("Font"));
        let to_unicode_key = arena.intern_name(PdfName::new("ToUnicode"));

        let mut font_names = Vec::new();

        if let Some(dict) = arena.get_dict(res_h)
            && let Some(font_dict_obj) = dict.get(&font_type_key)
            && let Some(fh) = font_dict_obj.resolve(arena).as_dict_handle()
            && let Some(font_dict) = arena.get_dict(fh)
        {
            // Pass 1: Discover the best available CMap by direct inspection of dictionaries
            let mut best_count = 0;
            for (key, obj) in &font_dict {
                if let Some(name) = arena.get_name(*key) {
                    font_names.push(name);
                }

                let f_dict_obj = obj.resolve(arena);
                if let Some(f_dict_h) = f_dict_obj.as_dict_handle()
                    && let Some(f_dict) = arena.get_dict(f_dict_h)
                    && let Some(tu_obj) = f_dict.get(&to_unicode_key)
                    && let Ok(data) = self.doc.decode_stream(&tu_obj.resolve(arena))
                    && let Ok(m) = ferruginous_core::font::cmap::CMap::parse(&data)
                    && m.mappings.len() > best_count
                {
                    best_count = m.mappings.len();
                    self.global_rescue_cmap = Some(m);
                }
            }
        }

        // Pass 2: Resolve all fonts using the discovered best rescue CMap
        for name in font_names {
            let _ = self.resolve_font_resource(&name);
        }
    }

    pub(crate) fn find_rescue_cmap(
        &self,
        current_res: &FontResource,
    ) -> Option<ferruginous_core::font::cmap::CMap> {
        let current_fd = current_res.font_descriptor?;
        let arena = self.doc.arena();
        let font_type_key = arena.intern_name(PdfName::new("Font"));
        let font_descriptor_key = arena.intern_name(PdfName::new("FontDescriptor"));
        let descendant_fonts_key = arena.intern_name(PdfName::new("DescendantFonts"));
        let to_unicode_key = arena.intern_name(PdfName::new("ToUnicode"));

        for res_handle in self.resource_stack.iter().rev() {
            if let Some(dict) = arena.get_dict(*res_handle)
                && let Some(font_dict_obj) = dict.get(&font_type_key)
                && let Object::Dictionary(fh) = font_dict_obj.resolve(arena)
                && let Some(font_dict) = arena.get_dict(fh)
            {
                for obj in font_dict.values() {
                    if let Object::Dictionary(h) = obj.resolve(arena)
                        && let Some(d) = arena.get_dict(h)
                    {
                        // 1. Identify candidate FontDescriptor
                        let mut candidate_fd = if let Some(fd_obj) = d.get(&font_descriptor_key) {
                            fd_obj.as_reference()
                        } else {
                            None
                        };
                        if candidate_fd.is_none()
                            && let Some(df_obj) = d.get(&descendant_fonts_key)
                            && let Object::Array(ah) = df_obj.resolve(arena)
                            && let Some(arr) = arena.get_array(ah)
                            && let Some(df_dict_obj) = arr.first()
                            && let Object::Dictionary(dfh) = df_dict_obj.resolve(arena)
                            && let Some(df_dict) = arena.get_dict(dfh)
                            && let Some(fd_obj) = df_dict.get(&font_descriptor_key)
                        {
                            candidate_fd = fd_obj.as_reference();
                        }

                        // 2. Compare descriptors and check for better ToUnicode
                        if let Some(cfd) = candidate_fd
                            && cfd.index() == current_fd.index()
                            && let Some(tu_obj) = d.get(&to_unicode_key)
                            && let Ok(data) = self.doc.decode_stream(&tu_obj.resolve(arena))
                            && let Ok(m) = ferruginous_core::font::cmap::CMap::parse(&data)
                        {
                            let current_len =
                                current_res.to_unicode.as_ref().map_or(0, |m| m.mappings.len());
                            if m.mappings.len() > current_len && m.mappings.len() >= 20 {
                                return Some(m);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
