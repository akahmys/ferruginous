use crate::arena::PdfArena;
use crate::handle::Handle;
use crate::object::Object;
use crate::font::FontResource;
use crate::Document;
use crate::PdfName;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn discover_fonts(arena: &PdfArena, doc: &Document) -> BTreeMap<u32, Arc<FontResource>> {
    let mut cache = BTreeMap::new();
    let type_key = arena.name("Type");
    let font_val = arena.name("Font");
    let base_font_key = arena.name("BaseFont");
    let subtype_key = arena.name("Subtype");

    for handle in arena.all_dict_handles() {
        if let Some(dict) = arena.get_dict(handle) {
            let is_font = dict.get(&type_key).and_then(|o| o.resolve(arena).as_name()) == Some(font_val)
                || (dict.contains_key(&base_font_key) && dict.contains_key(&subtype_key));
            
            if is_font
                && let Ok(font_res) = FontResource::load(&dict, doc) {
                cache.insert(handle.index(), Arc::new(font_res));
            }
        }
    }
    cache
}

pub fn map_stream_contexts(arena: &PdfArena, fonts: &BTreeMap<u32, Arc<FontResource>>) -> BTreeMap<u32, BTreeMap<String, Arc<FontResource>>> {
    let mut contexts = BTreeMap::new();
    let type_key = arena.name("Type");
    let page_val = arena.name("Page");
    let subtype_key = arena.name("Subtype");
    let form_val = arena.name("Form");
    let resources_key = arena.name("Resources");
    let font_key = arena.name("Font");
    let contents_key = arena.name("Contents");

    for handle in arena.all_dict_handles() {
        if let Some(dict) = arena.get_dict(handle) {
            let is_page = dict.get(&type_key).and_then(|o| o.resolve(arena).as_name()) == Some(page_val);
            let is_form = dict.get(&subtype_key).and_then(|o| o.resolve(arena).as_name()) == Some(form_val);
            
            if (is_page || is_form)
                && let Some(res_obj) = dict.get(&resources_key)
                && let Some(res_dict_h) = res_obj.resolve(arena).as_dict_handle()
                && let Some(res_dict) = arena.get_dict(res_dict_h)
                && let Some(f_obj) = res_dict.get(&font_key)
                && let Some(f_dict_h) = f_obj.resolve(arena).as_dict_handle()
                && let Some(f_dict) = arena.get_dict(f_dict_h)
            {
                let mut context_fonts = BTreeMap::new();
                for (res_name_h, font_obj) in f_dict {
                    if let Some(res_name) = arena.get_name(res_name_h)
                        && let Some(font_dict_h) = font_obj.resolve(arena).as_dict_handle()
                        && let Some(font_res) = fonts.get(&font_dict_h.index()) {
                        context_fonts.insert(res_name.as_str().to_string(), font_res.clone());
                    }
                }
                
                if is_page {
                    associate_page_streams(arena, &dict, &contents_key, context_fonts, &mut contexts);
                } else {
                    associate_form_stream(arena, handle, context_fonts, &mut contexts);
                }
            }
        }
    }
    contexts
}

fn associate_page_streams(
    arena: &PdfArena,
    dict: &BTreeMap<Handle<PdfName>, Object>,
    contents_key: &Handle<PdfName>,
    context_fonts: BTreeMap<String, Arc<FontResource>>,
    contexts: &mut BTreeMap<u32, BTreeMap<String, Arc<FontResource>>>,
) {
    if let Some(contents) = dict.get(contents_key) {
        let resolved = contents.resolve(arena);
        match resolved {
            Object::Reference(h) => { contexts.insert(h.index(), context_fonts); }
            Object::Stream(h, _) => { contexts.insert(h.index(), context_fonts); }
            Object::Array(ah) => {
                if let Some(arr) = arena.get_array(ah) {
                    for item in arr {
                        let res_item = item.resolve(arena);
                        match res_item {
                            Object::Reference(h) => { contexts.insert(h.index(), context_fonts.clone()); }
                            Object::Stream(h, _) => { contexts.insert(h.index(), context_fonts.clone()); }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn associate_form_stream(
    arena: &PdfArena,
    dict_handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
    context_fonts: BTreeMap<String, Arc<FontResource>>,
    contexts: &mut BTreeMap<u32, BTreeMap<String, Arc<FontResource>>>,
) {
    for i in 0..arena.object_count() {
        let h = Handle::new(i);
        if let Some(Object::Stream(dh, _)) = arena.get_object(h)
            && dh.index() == dict_handle.index() {
            contexts.insert(h.index(), context_fonts.clone());
        }
    }
}
