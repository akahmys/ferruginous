use crate::Document;
use crate::PdfName;
use crate::arena::PdfArena;
use crate::font::FontResource;
use crate::handle::Handle;
use crate::object::Object;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn discover_fonts(arena: &PdfArena, doc: &Document) -> BTreeMap<u32, Arc<FontResource>> {
    let mut cache = BTreeMap::new();
    let type_key = arena.name("Type");
    let font_val = arena.name("Font");
    let base_font_key = arena.name("BaseFont");
    let subtype_key = arena.name("Subtype");

    for i in 0..arena.object_count() {
        let obj_handle = Handle::new(i);
        if let Some(Object::Dictionary(dict_handle)) = arena.get_object(obj_handle)
            && let Some(dict) = arena.get_dict(dict_handle) {
                let type_val = dict.get(&type_key).and_then(|o| o.resolve(arena).as_name());
                let is_font = if let Some(tv) = type_val {
                    tv == font_val
                } else {
                    dict.contains_key(&base_font_key) && dict.contains_key(&subtype_key)
                };

                if is_font {
                    match FontResource::load(&dict, doc) {
                        Ok(font_res) => {
                            cache.insert(obj_handle.index(), Arc::new(font_res));
                        }
                        Err(_e) => {
                            // Identified as font but failed to load
                        }
                    }
                }
            }
    }
    cache
}

pub fn map_stream_contexts(
    arena: &PdfArena,
    fonts: &BTreeMap<u32, Arc<FontResource>>,
) -> BTreeMap<u32, BTreeMap<String, Arc<FontResource>>> {
    let mut contexts = BTreeMap::new();
    let type_key = arena.name("Type");
    let page_val = arena.name("Page");
    let subtype_key = arena.name("Subtype");
    let form_val = arena.name("Form");
    let resources_key = arena.name("Resources");
    let font_key = arena.name("Font");
    let contents_key = arena.name("Contents");

    for i in 0..arena.object_count() {
        let obj_h = Handle::new(i);
        if let Some(Object::Dictionary(handle)) | Some(Object::Stream(handle, _)) = arena.get_object(obj_h)
            && let Some(dict) = arena.get_dict(handle) {
                let is_page =
                    dict.get(&type_key).and_then(|o| o.resolve(arena).as_name()) == Some(page_val);
                let is_form =
                    dict.get(&subtype_key).and_then(|o| o.resolve(arena).as_name()) == Some(form_val);

                if is_page || is_form {
                    let mut context_fonts = BTreeMap::new();
                    
                    // 1. Accumulate resources from the entire hierarchy (Real-world flattening)
                    let mut current_node = Some(dict.clone());
                    let mut resource_nodes = Vec::new();

                    while let Some(node) = current_node {
                        if let Some(res_obj) = node.get(&resources_key)
                            && let Some(res_dict_h) = res_obj.resolve(arena).as_dict_handle()
                            && let Some(res_dict) = arena.get_dict(res_dict_h)
                        {
                            resource_nodes.push(res_dict);
                        }
                        
                        if is_form { break; }

                        let parent_key = arena.name("Parent");
                        if let Some(parent_ref) = node.get(&parent_key) {
                            let resolved_parent = parent_ref.resolve(arena);
                            if let Object::Dictionary(parent_dict_h) = resolved_parent {
                                current_node = arena.get_dict(parent_dict_h);
                            } else {
                                current_node = None;
                            }
                        } else {
                            current_node = None;
                        }
                    }

                    // 2. Merge all discovered resources (Child overrides Parent)
                    resource_nodes.reverse(); // Parents first
                    for res_dict in resource_nodes {
                        if let Some(f_obj) = res_dict.get(&font_key)
                            && let Some(f_dict_h) = f_obj.resolve(arena).as_dict_handle()
                            && let Some(f_dict) = arena.get_dict(f_dict_h)
                        {
                            for (res_name_h, font_obj) in f_dict {
                                if let Some(res_name) = arena.get_name(res_name_h)
                                    && let Some(font_obj_h) = font_obj.as_reference()
                                    && let Some(font_res) = fonts.get(&font_obj_h.index())
                                {
                                    context_fonts.insert(res_name.as_str().to_string(), font_res.clone());
                                }
                            }
                        }
                    }

                    if is_page {
                        associate_page_streams(
                            arena,
                            &dict,
                            &contents_key,
                            context_fonts,
                            &mut contexts,
                        );
                    } else {
                        // For Forms, the stream object itself is obj_h
                        contexts.insert(obj_h.index(), context_fonts);
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
        match contents {
            Object::Reference(h) => {
                contexts.insert(h.index(), context_fonts);
            }
            Object::Array(ah) => {
                if let Some(arr) = arena.get_array(*ah) {
                    for item in arr {
                        if let Object::Reference(h) = item {
                            contexts.insert(h.index(), context_fonts.clone());
                        }
                    }
                }
            }
            Object::Stream(_, _) => {
                // This shouldn't happen for Page Contents (usually references),
                // but if it's a direct stream, we'd need its object handle.
            }
            _ => {}
        }
    }
}
