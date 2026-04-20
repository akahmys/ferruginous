//! High-level encryption algorithms for PDF 1.7.
//!
//! Original Copyright (c) 2016-2022 J-F-Liu
//! Licensed under MIT License.

use crate::lopdf::encryption::rc4::Rc4;
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use md5::{Digest, Md5};

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub enum Algorithm {
    Rc4,
    Aes,
}

pub struct Decryptor {
    algorithm: Algorithm,
    key: Vec<u8>,
}

impl Decryptor {
    pub fn new(algorithm: Algorithm, key: &[u8]) -> Self {
        Decryptor { algorithm, key: key.to_vec() }
    }

    pub fn decrypt(&self, obj_id: u32, gen: u16, data: &[u8]) -> Vec<u8> {
        let mut key = self.key.clone();
        key.extend_from_slice(&(obj_id & 0xFFFFFF).to_le_bytes()[0..3]);
        key.extend_from_slice(&gen.to_le_bytes()[0..2]);

        if matches!(self.algorithm, Algorithm::Aes) {
            key.extend_from_slice(b"sAlT");
        }

        let mut hasher = Md5::new();
        hasher.update(&key);
        let derived_key = &hasher.finalize()[..std::cmp::min(key.len() + 5, 16)];

        let mut output = data.to_vec();
        match self.algorithm {
            Algorithm::Rc4 => {
                let mut rc4 = Rc4::new(derived_key);
                rc4.apply_keystream(&mut output);
            }
            Algorithm::Aes => {
                if output.len() < 16 {
                    return output;
                }
                let iv = &output[..16];
                let ciphertext = &output[16..];
                let cipher = Aes128CbcDec::new(derived_key.into(), iv.into());
                // In a real implementation, we'd handle padding correctly.
                // For simplicity in this bridge, we assume standard Pkcs7.
                let mut buffer = ciphertext.to_vec();
                if let Ok(decrypted) = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer) {
                    return decrypted.to_vec();
                }
            }
        }
        output
    }
}

/// Derives the file encryption key from the document's security dictionary.
pub fn derive_key_v2(
    password: &[u8],
    o: &[u8],
    p: i32,
    id: &[u8],
    encrypt_metadata: bool,
) -> Vec<u8> {
    let mut hasher = Md5::new();
    // Padded password
    let mut padded_pw = [0u8; 32];
    let len = std::cmp::min(password.len(), 32);
    padded_pw[..len].copy_from_slice(&password[..len]);
    if len < 32 {
        let padding = [
            0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA,
            0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE,
            0x64, 0x53, 0x69, 0x7A,
        ];
        padded_pw[len..].copy_from_slice(&padding[..32 - len]);
    }
    hasher.update(padded_pw);
    hasher.update(o);
    hasher.update((p as u32).to_le_bytes());
    hasher.update(id);
    if !encrypt_metadata {
        hasher.update([0xFF, 0xFF, 0xFF, 0xFF]);
    }
    hasher.finalize().to_vec()
}
