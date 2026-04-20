use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum XrefEntry {
    /// In use entry: (offset, generation)
    Normal { offset: u64, generation: u16 },
    /// Compressed entry: (container_obj_id, index)
    Compressed { container: u32, index: u16 },
    /// Free entry: (next_free_obj_id, next_generation)
    Free { next: u32, generation: u16 },
}

#[derive(Debug, Clone, Default)]
pub struct Xref {
    pub entries: BTreeMap<u32, XrefEntry>,
}

impl Xref {
    pub fn new() -> Self {
        Xref { entries: BTreeMap::new() }
    }

    pub fn insert(&mut self, id: u32, entry: XrefEntry) {
        self.entries.insert(id, entry);
    }

    pub fn get(&self, id: u32) -> Option<&XrefEntry> {
        self.entries.get(&id)
    }

    /// Merges another xref into this one (e.g. from an incremental update).
    /// Later entries overwrite earlier ones.
    pub fn merge(&mut self, other: Xref) {
        for (id, entry) in other.entries {
            self.entries.insert(id, entry);
        }
    }
}
