//! Refinery 2.1 Typesafe Handle System.
//!
//! Handles provide O(1) access to objects within a `PdfArena` without the overhead
//! of Reference Counting (Arc) or the risks of raw pointers.

use std::marker::PhantomData;
use std::fmt;
use serde::{Serialize, Deserialize};

/// A typesafe handle to an object in the `PdfArena`.
///
/// The handle contains an index into the arena and uses PhantomData to preserve
/// the logical type of the object (e.g., Handle<Dictionary>).
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Handle<T> {
    index: u32,
    _phantom: PhantomData<T>,
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Handle<T> {}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> Handle<T> {
    /// Creates a new handle from a raw index.
    pub const fn new(index: u32) -> Self {
        Self {
            index,
            _phantom: PhantomData,
        }
    }

    /// Returns the raw index of the handle.
    pub const fn index(&self) -> u32 {
        self.index
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({})", std::any::type_name::<T>().split("::").last().unwrap_or("Unknown"), self.index)
    }
}
