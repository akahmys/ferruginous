//! Optional Content Groups (OCG) and Membership Dictionaries (OCMD).
//!
//! (ISO 32000-2:2020 Clause 14.11)

use crate::core::{Object, Reference, Resolver};
use std::collections::BTreeMap;

/// Represents the visibility policy for an `OCMD` (Clause 14.11.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityPolicy {
    /// Visible if all corresponding OCGs are ON.
    AllOn,
    /// Visible if any corresponding OCG is ON (Default).
    AnyOn,
    /// Visible if any corresponding OCG is OFF.
    AnyOff,
    /// Visible if all corresponding OCGs are OFF.
    AllOff,
}

impl VisibilityPolicy {
    /// Returns the policy from a name object.
    #[must_use] pub fn from_name(name: &[u8]) -> Self {
        match name {
            b"AllOn" => Self::AllOn,
            b"AnyOn" => Self::AnyOn,
            b"AnyOff" => Self::AnyOff,
            b"AllOff" => Self::AllOff,
            _ => Self::AnyOn,
        }
    }
}

/// Represents an Optional Content Group (Clause 14.11.2.2).
#[derive(Debug, Clone)]
pub struct OptionalContentGroup {
    /// The object reference for this OCG.
    pub reference: Reference,
    /// The name of the group (for UI).
    pub name: Vec<u8>,
    /// The intent of the group (e.g., /View, /Design).
    pub intent: Vec<Vec<u8>>,
}

/// Represents an Optional Content Membership Dictionary (Clause 14.11.2.3).
#[derive(Debug, Clone)]
pub struct OptionalContentMembership {
    /// The OCGs associated with this membership dictionary.
    pub ocgs: Vec<Reference>,
    /// The visibility policy.
    pub policy: VisibilityPolicy,
}

/// Manages the state and configuration of Optional Content (Clause 14.11.4).
#[derive(Debug, Clone)]
pub struct OCContext {
    /// Map of OCG reference to its current ON/OFF state.
    pub states: BTreeMap<Reference, bool>,
}

impl OCContext {
    /// Creates a new context with all OCGs in their default state (OFF if not specified).
    pub fn new(states: BTreeMap<Reference, bool>) -> Self {
        Self { states }
    }

    /// Evaluates the visibility of an object given its properties (OCG or OCMD).
    pub fn is_visible(&self, properties: &Object, resolver: &dyn Resolver) -> bool {
        match properties {
            Object::Reference(r) => {
                // Could be an OCG or an OCMD
                let obj = match resolver.resolve(r) {
                    Ok(o) => o,
                    Err(_) => return true, // Visibility failure defaults to visible to avoid data loss
                };
                
                match obj {
                    Object::Dictionary(ref dict) => {
                        let type_name = dict.get(b"Type".as_ref()).and_then(|o| if let Object::Name(n) = o { Some(n.as_ref()) } else { None });
                        match type_name {
                            Some(b"OCG") => *self.states.get(r).unwrap_or(&true),
                            Some(b"OCMD") => self.evaluate_ocmd(dict),
                            _ => true,
                        }
                    }
                    _ => true,
                }
            }
            Object::Dictionary(dict) => {
                 // Inline OCMD (rare but possible according to spec)
                 self.evaluate_ocmd(dict)
            }
            _ => true,
        }
    }

    fn evaluate_ocmd(&self, dict: &BTreeMap<Vec<u8>, Object>) -> bool {
        let ocgs_obj = dict.get(b"OCGs".as_ref());
        let policy_name = dict.get(b"P".as_ref()).and_then(|o| if let Object::Name(n) = o { Some(n.as_ref()) } else { None }).unwrap_or(b"AnyOn");
        let policy = VisibilityPolicy::from_name(policy_name);

        let ocgs = match ocgs_obj {
            Some(Object::Reference(r)) => vec![*r],
            Some(Object::Array(arr)) => arr.iter().filter_map(|o| if let Object::Reference(r) = o { Some(*r) } else { None }).collect(),
            _ => return true,
        };

        if ocgs.is_empty() { return true; }

        match policy {
            VisibilityPolicy::AllOn => ocgs.iter().all(|r| *self.states.get(r).unwrap_or(&true)),
            VisibilityPolicy::AnyOn => ocgs.iter().any(|r| *self.states.get(r).unwrap_or(&true)),
            VisibilityPolicy::AnyOff => ocgs.iter().any(|r| !*self.states.get(r).unwrap_or(&true)),
            VisibilityPolicy::AllOff => ocgs.iter().all(|r| !*self.states.get(r).unwrap_or(&true)),
        }
    }
}

/// Represents the global `/OCProperties` configuration.
pub struct OCProperties {
    /// All OCGs in the document.
    pub ocgs: Vec<Reference>,
    /// The default configuration dictionary.
    pub default_config: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
}

impl OCProperties {
    /// Extracts the initial state (ON/OFF) from the default configuration.
    pub fn default_state(&self) -> BTreeMap<Reference, bool> {
        let mut states = BTreeMap::new();
        
        // Initially all OCGs are ON unless specified in /OFF
        for &r in &self.ocgs {
            states.insert(r, true);
        }

        if let Some(Object::Array(off_arr)) = self.default_config.get(b"OFF".as_ref()) {
            for item in off_arr.iter() {
                if let Object::Reference(r) = item {
                    states.insert(*r, false);
                }
            }
        }
        
        // Also respect /ON if it's explicitly provided (redundant but for correctness)
        if let Some(Object::Array(on_arr)) = self.default_config.get(b"ON".as_ref()) {
            for item in on_arr.iter() {
                if let Object::Reference(r) = item {
                    states.insert(*r, true);
                }
            }
        }

        states
    }
}
