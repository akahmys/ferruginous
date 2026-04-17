use bytes::Bytes;
use thiserror::Error;

pub mod lopdf;

use encoding_rs::SHIFT_JIS;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Normalization error: {0}")]
    Normalization(String),
}

/// Helper to normalize Shift-JIS strings to UTF-8.
pub fn normalize_sjis(data: &[u8]) -> String {
    let (res, _enc, _errors) = SHIFT_JIS.decode(data);
    res.into_owned()
}

/// Trait for the legacy PDF bridge.
pub trait LegacyBridge {
    /// Loads a legacy PDF and returns normalized content.
    fn load_and_normalize(&self, data: Bytes) -> Result<Bytes, BridgeError>;
    
    /// Decrypts content using legacy methods (RC4/AES-128).
    fn decrypt_legacy(&self, data: Bytes, key: &[u8], algorithm: lopdf::encryption::Algorithm) -> Result<Bytes, BridgeError>;
}

pub struct LopdfBridge;

impl LopdfBridge {
    pub fn new() -> Self {
        LopdfBridge
    }
}

impl Default for LopdfBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyBridge for LopdfBridge {
    fn load_and_normalize(&self, data: Bytes) -> Result<Bytes, BridgeError> {
        // Use the newly implemented Reader for structural repair
        let mut doc = lopdf::Reader::load_document(&data)?;
        
        // Apply Shift-JIS to UTF-8 normalization if it's a legacy version
        if !data.starts_with(b"%PDF-2.0") {
            doc.apply_normalization();
        }

        // For now, we return the original data if it was repairable.
        // In a full implementation, we could re-serialize the document here.
        // For Step 2/3, the SDK will use the Document object directly via migration.
        Ok(data) 
    }

    fn decrypt_legacy(&self, data: Bytes, key: &[u8], algorithm: lopdf::encryption::Algorithm) -> Result<Bytes, BridgeError> {
        let dc = lopdf::encryption::Decryptor::new(algorithm, key);
        // Obj ID and Generation are used to derive the key in legacy encryption.
        let output = dc.decrypt(0, 0, &data);
        Ok(Bytes::from(output))
    }
}
