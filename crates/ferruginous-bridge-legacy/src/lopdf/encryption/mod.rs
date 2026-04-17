//! Encryption module for modified lopdf.

pub mod rc4;
pub mod algorithms;

pub use algorithms::{Decryptor, Algorithm, derive_key_v2};
pub use rc4::Rc4;
