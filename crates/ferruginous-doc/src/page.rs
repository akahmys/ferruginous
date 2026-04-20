use ferruginous_core::{Object, PdfError, PdfName, PdfResult, Reference, Resolver};
use std::sync::Arc;

pub struct PageTree<'a> {
    root: Reference,
    resolver: &'a dyn Resolver,
}

impl<'a> PageTree<'a> {
    pub fn new(root: Reference, resolver: &'a dyn Resolver) -> Self {
        Self { root, resolver }
    }

    pub fn count(&self) -> PdfResult<usize> {
        let obj = self.resolver.resolve(&self.root)?;
        if let Some(dict) = obj.as_dict()
            && let Some(Object::Integer(count)) = dict.get(&"Count".into())
        {
            return Ok(*count as usize);
        }
        Err(PdfError::Other("Invalid /Pages node".into()))
    }

    pub fn page(&self, index: usize) -> PdfResult<Page<'a>> {
        let mut stack = vec![(self.root, 0)];
        let mut pages_seen = 0;

        while let Some((node_ref, _)) = stack.last() {
            let node_ref = *node_ref; // Copy to avoid borrow issues

            let obj = self.resolver.resolve(&node_ref)?;
            let dict = obj
                .as_dict()
                .ok_or_else(|| PdfError::Other(format!("Expected dictionary at {:?}", node_ref)))?;

            let type_val = dict
                .get(&"Type".into())
                .and_then(|o| if let Object::Name(n) = o { Some(n.0.as_ref()) } else { None });

            if type_val == Some(b"Page") {
                if pages_seen == index {
                    let parent_refs = stack.iter().take(stack.len() - 1).map(|(r, _)| *r).collect();

                    return Ok(Page {
                        reference: node_ref,
                        dictionary: Arc::new(dict.clone()),
                        parents: parent_refs,
                        resolver: self.resolver,
                    });
                }
                pages_seen += 1;
                stack.pop();
            } else {
                // Pages node (Branch)
                let kids = if let Some(Object::Array(k)) = dict.get(&"Kids".into()) {
                    k
                } else {
                    return Err(PdfError::Other(format!(
                        "Missing Kids in Pages node {:?}",
                        node_ref
                    )));
                };

                // Get current child index and increment it
                let child_idx = {
                    let top =
                        stack.last_mut().ok_or_else(|| PdfError::Other("Empty stack".into()))?;
                    let idx = top.1;
                    top.1 += 1;
                    idx
                };

                if child_idx < kids.len() {
                    let next_ref = if let Object::Reference(r) = &kids[child_idx] {
                        *r
                    } else {
                        return Err(PdfError::Other("Invalid Kid Reference".into()));
                    };
                    stack.push((next_ref, 0));
                } else {
                    stack.pop();
                }
            }

            if stack.len() > 32 {
                return Err(PdfError::Other(
                    "Page tree recursion depth exceeded limit (32)".into(),
                ));
            }
        }
        Err(PdfError::Other("Page not found".into()))
    }
}

pub struct Page<'a> {
    pub reference: Reference,
    pub dictionary: Arc<std::collections::BTreeMap<PdfName, Object>>,
    pub parents: Vec<Reference>, // The chain of /Pages nodes
    pub resolver: &'a dyn Resolver,
}

impl<'a> Page<'a> {
    /// Resolves an attribute, following the inheritance chain if necessary.
    /// (ISO 32000-2:2020 Clause 7.7.3.3)
    pub fn attribute(&self, key: &PdfName) -> Option<Object> {
        // 1. Check current page dictionary
        if let Some(val) = self.dictionary.get(key) {
            return Some(val.clone());
        }

        // 2. Check parents in order (leaf-to-root)
        for parent_ref in self.parents.iter().rev() {
            if let Ok(obj) = self.resolver.resolve(parent_ref)
                && let Some(dict) = obj.as_dict()
                && let Some(val) = dict.get(key)
            {
                return Some(val.clone());
            }
        }

        None
    }

    pub fn resources(&self) -> Option<Object> {
        self.attribute(&"Resources".into()).and_then(|o| self.resolver.resolve_if_ref(&o).ok())
    }

    pub fn media_box(&self) -> Option<Object> {
        self.attribute(&"MediaBox".into())
    }
}
