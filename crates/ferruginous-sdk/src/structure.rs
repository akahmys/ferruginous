//! PDF Logical Structure Engine (ISO 32000-2:2020 Clause 14.7)
//! 
//! (ISO 14289-2 / PDF/UA-2 Compliance Bridge)

use ferruginous_core::{Handle, Object, PdfArena, PdfName, PdfResult, PdfError};
use std::collections::{BTreeMap, VecDeque};
use serde::{Serialize, Deserialize};

/// A visitor for traversing the Logical Structure Tree iteratively (RR-15 compliant).
pub struct StructureVisitor<'a> {
    /// Reference to the PDF arena.
    pub arena: &'a PdfArena,
    /// Stack for iterative DFS traversal.
    pub stack: VecDeque<Handle<BTreeMap<Handle<PdfName>, Object>>>,
}

impl<'a> StructureVisitor<'a> {
    /// Creates a new visitor starting from the given structure root.
    pub fn new(arena: &'a PdfArena, root: Handle<BTreeMap<Handle<PdfName>, Object>>) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(root);
        Self { arena, stack }
    }

    /// Iteratively walks the tree and yields structure elements.
    pub fn next_element(&mut self) -> Option<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        let current = self.stack.pop_back()?;
        
        let dict = self.arena.get_dict(current)?;
        let kids_key = self.arena.get_name_by_str("K")?;

        if let Some(kids) = dict.get(&kids_key) {
            match kids.resolve(self.arena) {
                Object::Array(h) => {
                    if let Some(array) = self.arena.get_array(h) {
                        for kid in array.iter().rev() {
                            if let Some(kid_handle) = kid.resolve(self.arena).as_dict_handle() {
                                self.stack.push_back(kid_handle);
                            }
                        }
                    }
                }
                Object::Dictionary(h) => {
                    self.stack.push_back(h);
                }
                _ => {} // MCR or OBJR are leaves, handled by the Auditor
            }
        }

        Some(current)
    }
}

/// Matterhorn-compliant Structural Auditor.
pub struct MatterhornAuditor<'a> {
    /// Reference to the PDF arena for looking up objects.
    arena: &'a PdfArena,
}

/// Represents a single finding from a structural audit.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditFinding {
    /// The Matterhorn Protocol checkpoint ID (e.g., "01-001").
    pub checkpoint: String,
    /// The severity of the finding (e.g., "Error", "Warning").
    pub severity: String,
    /// A human-readable message describing the issue.
    pub message: String,
}

impl<'a> MatterhornAuditor<'a> {
    /// Creates a new Matterhorn Auditor.
    pub fn new(arena: &'a PdfArena) -> Self {
        Self { arena }
    }

    /// Performs a full UA-2 structural audit.
    pub fn audit(&self, root: Handle<BTreeMap<Handle<PdfName>, Object>>) -> PdfResult<Vec<AuditFinding>> {
        let mut findings = Vec::new();
        let mut visitor = StructureVisitor::new(self.arena, root);
        
        let mut last_heading_level = 0;

        while let Some(element_handle) = visitor.next_element() {
            let dict = self.arena.get_dict(element_handle).ok_or_else(|| PdfError::Other("Structure element dict not found".into()))?;
            let s_key = self.arena.get_name_by_str("S").ok_or_else(|| PdfError::Other("S key not interned".into()))?; // Subtype/Tag name

            if let Some(tag_name_handle) = dict.get(&s_key).and_then(|o: &Object| o.resolve(self.arena).as_name()) {
                let tag_name = self.arena.get_name(tag_name_handle).ok_or_else(|| PdfError::Other("Tag name not found".into()))?;
                let tag_str = tag_name.as_str();
                
                // 1. Heading Hierarchy Check (Matterhorn Checkpoint 14)
                if tag_str.starts_with('H') && tag_str.len() == 2
                    && let Ok(level) = tag_str[1..].parse::<i32>() {
                        if level > last_heading_level + 1 {
                            findings.push(AuditFinding {
                                checkpoint: "14-001".into(),
                                severity: "Error".into(),
                                message: format!("Heading level skipped: {tag_str} follows {last_heading_level}"),
                            });
                        }
                        last_heading_level = level;
                }

                // 2. Alt-text Check (Matterhorn Checkpoint 13)
                if tag_str == "Figure" {
                    let alt_key = self.arena.get_name_by_str("Alt").ok_or_else(|| PdfError::Other("Alt key not interned".into()))?;
                    if !dict.contains_key(&alt_key) {
                        findings.push(AuditFinding {
                            checkpoint: "13-001".into(),
                            severity: "Error".into(),
                            message: "Figure element missing /Alt text".into(),
                        });
                    }
                }
            }
        }

        Ok(findings)
    }
}
