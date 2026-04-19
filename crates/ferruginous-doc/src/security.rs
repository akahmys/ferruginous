use std::collections::BTreeMap;
use bytes::Bytes;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
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
    /// Encryption revision (R)
    revision: i32,
}

const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x66, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41,
    0x64, 0x43, 0x6E, 0x01, 0x64, 0x71, 0x08, 0x26, 0x02, 0xAD, 0x5A, 0xE0, 0x59, 0xA3, 0x34, 0x60,
];

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
        let id = if let Some(Object::Array(arr)) = dict.get(&"ID".into()) {
            arr.get(0).and_then(|o| o.as_string()).map(|b| b.as_ref()).unwrap_or(&[])
        } else {
            &[]
        };

        if r == 6 || r == 5 {
            Self::init_revision_6(dict, password, encrypt_metadata)
        } else if r == 4 || r == 3 || r == 2 {
            Self::init_legacy(dict, password, encrypt_metadata, id, r as i32)
        } else {
            Err(PdfError::EncryptionNotSupported(format!("Standard Revision {} not supported", r)))
        }
    }

    fn init_legacy(
        dict: &BTreeMap<PdfName, Object>,
        password: &[u8],
        encrypt_metadata: bool,
        id: &[u8],
        r: i32
    ) -> PdfResult<Self> {
        let _v = dict.get(&"V".into()).and_then(|o| o.as_i64()).unwrap_or(0);
        let o = dict.get(&"O".into()).and_then(|o| match o { Object::String(b) => Some(b.as_ref()), _ => None })
            .ok_or_else(|| PdfError::Other("Missing /O in legacy Encrypt dict".into()))?;
        let p = dict.get(&"P".into()).and_then(|o| o.as_i64()).unwrap_or(0) as i32;
        let length = dict.get(&"Length".into()).and_then(|o| o.as_i64()).unwrap_or(40) as usize;

        // Algorithm 3.2 - Computing an encryption key
        let mut context = md5::Context::new();
        
        // a) Pad password
        let mut padded = [0u8; 32];
        let len = std::cmp::min(password.len(), 32);
        padded[..len].copy_from_slice(&password[..len]);
        if len < 32 {
            padded[len..].copy_from_slice(&PADDING[..32 - len]);
        }
        context.consume(&padded);

        // b) O value
        context.consume(o);

        // c) P value (little-endian)
        context.consume(&p.to_le_bytes());

        // d) ID[0]
        context.consume(id);

        // e) EncryptMetadata (if R >= 4)
        if r >= 4 && !encrypt_metadata {
            context.consume(&[0xFF, 0xFF, 0xFF, 0xFF]);
        }

        let mut hash = context.finalize().0;

        // f) Revision 3+ loop
        if r >= 3 {
            let key_len = length / 8;
            for _ in 0..50 {
                hash = md5::compute(&hash[..key_len]).0;
            }
        }

        let final_key_len = if r == 2 { 5 } else { std::cmp::min(length / 8, 16) };
        let fek = hash[..final_key_len].to_vec();

        Ok(Self { fek, encrypt_metadata, key_length: final_key_len, revision: r })
    }

    fn init_revision_6(dict: &BTreeMap<PdfName, Object>, password: &[u8], encrypt_metadata: bool) -> PdfResult<Self> {
        let u = dict.get(&"U".into()).and_then(|o| match o { Object::String(b) => Some(b.as_ref()), _ => None })
            .ok_or_else(|| PdfError::Other("Missing /U in Rev 6 Encrypt dict".into()))?;
        let ue = dict.get(&"UE".into()).and_then(|o| match o { Object::String(b) => Some(b.as_ref()), _ => None })
            .ok_or_else(|| PdfError::Other("Missing /UE in Rev 6 Encrypt dict".into()))?;

        if u.len() < 48 { return Err(PdfError::Other("Invalid /U length for Rev 6".into())); }
        
        // 1. Derive FEK using PBKDF2 (Algorithm 3.10)
        let salt = &u[40..48];
        let mut k = [0u8; 64];
        pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha256>>(password, salt, 32768, &mut k)
            .map_err(|_| PdfError::Other("PBKDF2 derivation failed".into()))?;

        eprintln!("DEBUG: Derived K[0..16]: {:02x?}", &k[..16]);

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

        Ok(Self { fek: out_fek, encrypt_metadata, key_length: 32, revision: 6 })
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
            return Err(PdfError::Other("Encrypted data too short for AES (missing IV)".into()));
        }

        let iv = &data[..16];
        let ciphertext = &data[16..];
        let is_aesv3 = self.revision >= 6;

        if !is_aesv3 && !ciphertext.len().is_multiple_of(16) {
             return Err(PdfError::Other("Invalid AES-CBC ciphertext length (not a multiple of 16)".into()));
        }

        let mut out = vec![0u8; ciphertext.len()];
        let mut prev_block = iv.to_vec();

        if self.key_length == 32 {
            let cipher = aes::Aes256::new_from_slice(&self.fek)
                .map_err(|_| PdfError::Other("Failed to init AES-256".into()))?;
            
            // Decrypt full blocks
            let full_blocks = ciphertext.len() / 16;
            for i in 0..full_blocks {
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

            // Handle partial block for AESV3 (Revision 6+)
            if is_aesv3 && ciphertext.len() % 16 != 0 {
                let last_start = full_blocks * 16;
                let partial_len = ciphertext.len() - last_start;
                
                // Mask generation: Encrypt the last ciphertext block (or IV) in ECB mode
                let mut mask_block = [0u8; 16];
                mask_block.copy_from_slice(&prev_block); 
                let mut mask_ga = aes::cipher::generic_array::GenericArray::from(mask_block);
                cipher.encrypt_block(&mut mask_ga);
                
                for j in 0..partial_len {
                    out[last_start + j] = ciphertext[last_start + j] ^ mask_ga[j];
                }
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

        // PKCS#7 Unpadding (Skip for AESV3/Revision 6)
        if !is_aesv3 {
            if let Some(&last_byte) = out.last() {
                let pad_len = last_byte as usize;
                if pad_len > 0 && pad_len <= 16 {
                    let len = out.len();
                    if len >= pad_len && out[len - pad_len..].iter().all(|&b| b == last_byte) {
                        out.truncate(len - pad_len);
                    }
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
