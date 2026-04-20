//! Encryption module for modified lopdf.

pub mod algorithms;
pub mod rc4;

pub use algorithms::{derive_key_v2, Algorithm, Decryptor};
pub use rc4::Rc4;
