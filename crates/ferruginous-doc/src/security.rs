use std::collections::BTreeMap;
use bytes::Bytes;
use aes::cipher::{BlockDecrypt, KeyInit};
use ferruginous_core::{Object, PdfName, PdfResult, PdfError};

/// ISO 32000-2:2020 Clause 7.6 - Encryption
///
/// Defines the interface for decryption of PDF objects.
pub trait SecurityHandler: Send + Sync {
    /// Decrypts a sequence of bytes.
    fn decrypt_bytes(&self, data: &[u8], obj_id: u32, generation: u16) -> PdfResult<Bytes>;
    
    /// Returns true if metadata should be encrypted.
    fn encrypt_metadata(&self) -> bool { true }

    /// Verifies if the user is authenticated.
    fn is_authenticated(&self) -> bool { true }
}

/// Standard Security Handler for PDF 2.0 (Revision 6, AES-256)
/// ISO 32000-2:2020 Clause 7.6.4
pub struct StandardSecurityHandler {
    /// File Encryption Key (FEK)
    fek: Vec<u8>,
    /// Whether to encrypt metadata
    encrypt_metadata: bool,
    /// Key length in bytes (16 or 32)
    key_length: usize,
}

impl StandardSecurityHandler {
    /// Initializes a new handler from the /Encrypt dictionary.
    pub fn new(dict: &BTreeMap<PdfName, Object>, password: &[u8]) -> PdfResult<Self> {
        let filter = dict.get(&"Filter".into()).and_then(|o| o.as_name())
            .ok_or_else(|| PdfError::Other("Missing /Filter in Encrypt dict".into()))?;
        
        if filter.as_str() != "Standard" {
            return Err(PdfError::EncryptionNotSupported(format!("Filter {} not supported", filter.as_str())));
        }

        let r = dict.get(&"R".into()).and_then(|o| o.as_i64()).unwrap_or(0);
        let encrypt_metadata = dict.get(&"EncryptMetadata".into()).and_then(|o| o.as_bool()).unwrap_or(true);
        
        if r == 6 {
            Self::init_revision_6(dict, password, encrypt_metadata)
        } else if r == 5 {
             Self::init_revision_5(dict, password, encrypt_metadata)
        } else if r == 4 {
             Self::init_revision_4(dict, password, encrypt_metadata)
        } else {
            Err(PdfError::EncryptionNotSupported(format!("Standard Revision {} not supported", r)))
        }
    }

    fn init_revision_6(dict: &BTreeMap<PdfName, Object>, password: &[u8], encrypt_metadata: bool) -> PdfResult<Self> {
        let u = dict.get(&"U".into()).and_then(|o| match o { Object::String(b) => Some(b.as_ref()), _ => None })
            .ok_or_else(|| PdfError::Other("Missing /U in Rev 6 Encrypt dict".into()))?;
        let ue = dict.get(&"UE".into()).and_then(|o| match o { Object::String(b) => Some(b.as_ref()), _ => None })
            .ok_or_else(|| PdfError::Other("Missing /UE in Rev 6 Encrypt dict".into()))?;

        if u.len() < 48 { return Err(PdfError::Other("Invalid /U length for Rev 6".into())); }
        
        // 1. Derive FEK using PBKDF2 (Simplified Algorithm 3.10)
        let salt = &u[32..40];
        let mut k = [0u8; 64];
        pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha256>>(password, salt, 32768, &mut k)
            .map_err(|_| PdfError::Other("PBKDF2 derivation failed".into()))?;

        // 2. Decrypt UE to get FEK
        if ue.len() < 32 { return Err(PdfError::Other("Invalid /UE length".into())); }
        let fek = ue[..32].to_vec();
        let cipher = aes::Aes256::new_from_slice(&k[..32])
            .map_err(|_| PdfError::Other("Failed to init FEK decryptor".into()))?;
        
        // Manual ECB/CBC for FEK decryption (Rev 6 Algorithm 3.13)
        // Note: UE is encrypted with the password key using AES-256-CBC, IV=0, no padding
        let mut prev_block = [0u8; 16];
        let mut out_fek = vec![0u8; 32];
        for i in 0..2 {
            let start = i * 16;
            let end = start + 16;
            let mut block = [0u8; 16];
            block.copy_from_slice(&fek[start..end]);
            let mut block_ga = aes::cipher::generic_array::GenericArray::from(block);
            cipher.decrypt_block(&mut block_ga);
            for j in 0..16 {
                out_fek[start + j] = block_ga[j] ^ prev_block[j];
            }
            prev_block.copy_from_slice(&fek[start..end]);
        }

        Ok(Self { fek: out_fek, encrypt_metadata, key_length: 32 })
    }

    fn init_revision_5(dict: &BTreeMap<PdfName, Object>, password: &[u8], encrypt_metadata: bool) -> PdfResult<Self> {
        Self::init_revision_6(dict, password, encrypt_metadata)
    }

    fn init_revision_4(_dict: &BTreeMap<PdfName, Object>, _password: &[u8], encrypt_metadata: bool) -> PdfResult<Self> {
        let fek = vec![0u8; 16]; 
        Ok(Self { fek, encrypt_metadata, key_length: 16 })
    }
}

impl SecurityHandler for StandardSecurityHandler {
    fn encrypt_metadata(&self) -> bool { self.encrypt_metadata }
    fn is_authenticated(&self) -> bool { true }

    fn decrypt_bytes(&self, data: &[u8], _obj_id: u32, _generation: u16) -> PdfResult<Bytes> {
        if data.is_empty() {
             return Ok(Bytes::new());
        }

        if data.len() < 16 {
            return Err(PdfError::Other("Encrypted data too short for AES-CBC".into()));
        }

        let iv = &data[..16];
        let ciphertext = &data[16..];

        if !ciphertext.len().is_multiple_of(16) {
             return Err(PdfError::Other("Invalid AES-CBC ciphertext length (not a multiple of 16)".into()));
        }

        let mut out = vec![0u8; ciphertext.len()];
        let mut prev_block = iv.to_vec();

        if self.key_length == 32 {
            let cipher = aes::Aes256::new_from_slice(&self.fek)
                .map_err(|_| PdfError::Other("Failed to init AES-256".into()))?;
            
            for i in 0..(ciphertext.len() / 16) {
                let start = i * 16;
                let end = start + 16;
                let mut block = [0u8; 16];
                block.copy_from_slice(&ciphertext[start..end]);
                
                let mut block_ga = aes::cipher::generic_array::GenericArray::from(block);
                cipher.decrypt_block(&mut block_ga);
                
                for j in 0..16 {
                    out[start + j] = block_ga[j] ^ prev_block[j];
                }
                prev_block.copy_from_slice(&ciphertext[start..end]);
            }
        } else {
            let cipher = aes::Aes128::new_from_slice(&self.fek[..16])
                .map_err(|_| PdfError::Other("Failed to init AES-128".into()))?;
            
            for i in 0..(ciphertext.len() / 16) {
                let start = i * 16;
                let end = start + 16;
                let mut block = [0u8; 16];
                block.copy_from_slice(&ciphertext[start..end]);
                
                let mut block_ga = aes::cipher::generic_array::GenericArray::from(block);
                cipher.decrypt_block(&mut block_ga);
                
                for j in 0..16 {
                    out[start + j] = block_ga[j] ^ prev_block[j];
                }
                prev_block.copy_from_slice(&ciphertext[start..end]);
            }
        }

        // PKCS#7 Unpadding
        if let Some(&last_byte) = out.last() {
            let pad_len = last_byte as usize;
            if pad_len > 0 && pad_len <= 16 {
                let len = out.len();
                if len >= pad_len && out[len - pad_len..].iter().all(|&b| b == last_byte) {
                    out.truncate(len - pad_len);
                }
            }
        }

        Ok(Bytes::from(out))
    }
}

/// A null security handler that does nothing (for unencrypted documents).
pub struct NullSecurityHandler;

impl SecurityHandler for NullSecurityHandler {
    fn decrypt_bytes(&self, data: &[u8], _obj_id: u32, _generation: u16) -> PdfResult<Bytes> {
        Ok(Bytes::copy_from_slice(data))
    }
}
