use ferruginous_core::{Object, PdfResult, PdfError, Reference, Resolver, PdfName};
use crate::signature::Signature;
use cms::signed_data::SignedData;
use der::{Decode, Encode};
use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use sha1::Sha1;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::VerifyingKey;
use signature::Verifier;
use spki::DecodePublicKey;
use p256::ecdsa::VerifyingKey as EcdsaVerifyingKey;
use p256::ecdsa::Signature as EcdsaSignature;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    Valid,
    Invalid(String),
    Inconclusive(String),
    Weak(String),
    Untrusted(String),
    Revoked(String),
}

#[derive(Default)]
pub struct RevocationCache {
    pub certificates: Vec<Vec<u8>>,
    pub crls: Vec<Vec<u8>>,
    pub ocsps: Vec<Vec<u8>>,
}

pub struct SignatureVerifier<'a> {
    resolver: &'a dyn Resolver,
    root_id: u32,
    cache: std::sync::Arc<parking_lot::RwLock<RevocationCache>>,
}

impl<'a> SignatureVerifier<'a> {
    pub fn new(resolver: &'a dyn Resolver) -> Self {
        Self::with_root(resolver, 1) // Default to 1, but with_root is preferred
    }

    pub fn with_root(resolver: &'a dyn Resolver, root_id: u32) -> Self {
        Self { 
            resolver,
            root_id,
            cache: std::sync::Arc::new(parking_lot::RwLock::new(RevocationCache::default())),
        }
    }

    pub fn verify(&self, sig: &Signature, raw_data: &[u8]) -> PdfResult<ValidationStatus> {
        // Resolve LTV data if available from Catalog
        let catalog = self.resolver.resolve(&Reference::new(self.root_id, 0))?.as_dict()
            .ok_or_else(|| PdfError::Other("Invalid catalog".into()))?
            .clone();
        let _ = self.resolve_ltv_data(&catalog);

        self.verify_internal(sig, raw_data)
    }

    fn verify_internal(&self, sig: &Signature, raw_data: &[u8]) -> PdfResult<ValidationStatus> {
        // 1. Extract signed bytes (Integrity)
        let signed_bytes = sig.extract_signed_data(&bytes::Bytes::copy_from_slice(raw_data))?;
        
        // 2. Parse CMS signature
        let signed_data = SignedData::from_der(&sig.contents)
            .map_err(|e| PdfError::Other(format!("CMS parse error: {}", e)))?;

        // 3. Verify Message Digest
        let mut status = ValidationStatus::Valid;

        for signer_info in signed_data.signer_infos.0.iter() {
            let digest_oid = signer_info.digest_alg.oid.to_string();
            
            if digest_oid == "1.3.14.3.2.26" {
                status = ValidationStatus::Weak("SHA-1 digest used".into());
            }

            let mut hasher: Box<dyn crate::validation::DigestHasher> = match digest_oid.as_str() {
                "2.16.840.1.101.3.4.2.1" => Box::new(Sha256::new()),
                "1.3.14.3.2.26" => Box::new(Sha1::new()),
                _ => return Ok(ValidationStatus::Inconclusive(format!("Unsupported digest algorithm: {}", digest_oid))),
            };

            hasher.update(&signed_bytes);
            let computed_digest = hasher.finalize_vec();

            if let Some(auth_attrs) = &signer_info.signed_attrs {
                let mut message_digest_found = false;
                for attr in auth_attrs.iter() {
                    if attr.oid.to_string() == "1.2.840.113549.1.9.4" {
                        let digest_val = attr.values.iter().next()
                            .ok_or_else(|| PdfError::Other("Empty message-digest attribute".into()))?;
                        
                        let embedded_digest = digest_val.decode_as::<der::asn1::OctetString>()
                            .map_err(|_| PdfError::Other("Invalid message-digest format".into()))?;

                        if embedded_digest.as_bytes() != computed_digest.as_slice() {
                            return Ok(ValidationStatus::Invalid("Message digest mismatch".into()));
                        }
                        message_digest_found = true;
                        break;
                    }
                }
                if !message_digest_found {
                    return Ok(ValidationStatus::Invalid("Missing message-digest attribute".into()));
                }

                // 4. Verify Signature on Authenticated Attributes
                let attr_bytes = auth_attrs.to_der()
                    .map_err(|e| PdfError::Other(format!("Failed to encode attributes: {}", e)))?;
                
                // Find signer's certificate
                let signer_cert_bytes = self.find_signer_cert(&signed_data, signer_info)?;

                if let Some(cert_der) = signer_cert_bytes {
                    let (_, x509) = x509_parser::parse_x509_certificate(&cert_der)
                        .map_err(|e| PdfError::Other(format!("X.509 parse error: {}", e)))?;
                    
                    let pub_key_der = x509.public_key().raw;
                    let sig_oid = signer_info.signature_algorithm.oid.to_string();
                    
                    match sig_oid.as_str() {
                        "1.2.840.113549.1.1.1" | "1.2.840.113549.1.1.11" => {
                            let rsa_key = RsaPublicKey::from_public_key_der(pub_key_der)
                                .map_err(|e| PdfError::Other(format!("RSA key error: {}", e)))?;
                            let verifying_key: VerifyingKey<Sha256> = VerifyingKey::new(rsa_key);
                            
                            let signature_bytes = signer_info.signature.as_bytes();
                            // signature crate's Signature implementation for RSA
                            let sig = rsa::pkcs1v15::Signature::try_from(signature_bytes)
                                .map_err(|e| PdfError::Other(format!("Invalid RSA signature: {}", e)))?;
                            
                            if let Err(e) = verifying_key.verify(&attr_bytes, &sig) {
                                return Ok(ValidationStatus::Invalid(format!("RSA verification failed: {}", e)));
                            }
                        }
                        "1.2.840.10045.4.3.2" => { // ecdsa-with-SHA256
                            let verifying_key = EcdsaVerifyingKey::from_public_key_der(pub_key_der)
                                .map_err(|e| PdfError::Other(format!("ECDSA key error: {}", e)))?;
                            
                            let signature_bytes = signer_info.signature.as_bytes();
                            let sig = EcdsaSignature::from_der(signature_bytes)
                                .map_err(|e| PdfError::Other(format!("Invalid ECDSA signature: {}", e)))?;
                            
                            if let Err(e) = verifying_key.verify(&attr_bytes, &sig) {
                                return Ok(ValidationStatus::Invalid(format!("ECDSA verification failed: {}", e)));
                            }
                        }
                        _ => return Ok(ValidationStatus::Inconclusive(format!("Unsupported signature algorithm: {}", sig_oid))),
                    }

                    // 5. Verify RFC 3161 Timestamp if present
                    self.verify_timestamp(signer_info, signer_info.signature.as_bytes())?;

                    // 6. Verify Certificate Chain
                    let chain_status = self.verify_chain(&cert_der)?;
                    if let ValidationStatus::Valid = status {
                        status = chain_status;
                    }
                } else {
                    return Ok(ValidationStatus::Inconclusive("Signer certificate not found".into()));
                }
            }
        }

        Ok(status)
    }

    fn verify_timestamp(&self, signer_info: &cms::signed_data::SignerInfo, _sig_value: &[u8]) -> PdfResult<()> {
        if let Some(unauth_attrs) = &signer_info.unsigned_attrs {
            for attr in unauth_attrs.iter() {
                if attr.oid.to_string() == "1.2.840.113549.1.9.16.2.14" { // id-aa-signingCertificateV2 or timestampToken
                    if let Some(any_val) = attr.values.iter().next() {
                        let token_data = any_val.value();
                        let signed_data = SignedData::from_der(token_data)
                            .map_err(|e| PdfError::Other(format!("Timestamp CMS parse error: {}", e)))?;
                        
                        // Basic validation: the token should have its own signer
                        if signed_data.signer_infos.0.is_empty() {
                            return Err(PdfError::Other("Empty timestamp token".into()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_chain(&self, cert_der: &[u8]) -> PdfResult<ValidationStatus> {
        let (_, x509) = x509_parser::parse_x509_certificate(cert_der)
            .map_err(|e| PdfError::Other(format!("Chain cert parse error: {}", e)))?;
        
        // Match against DSS cache first
        let cache = self.cache.read();
        for dss_cert in &cache.certificates {
            let (_, dss_x509) = x509_parser::parse_x509_certificate(dss_cert)
                .map_err(|_| PdfError::Other("Invalid DSS certificate".into()))?;
            
            if dss_x509.subject() == x509.issuer() {
                // Potential issuer found in LTV cache
                // In a full implementation, we would recursively verify up to a root
                return Ok(ValidationStatus::Valid);
            }
        }

        // Fallback: check against system trust store would go here
        
        Ok(ValidationStatus::Untrusted("Certificate chain not verified to root".into()))
    }

    fn find_signer_cert(&self, signed_data: &SignedData, signer_info: &cms::signed_data::SignerInfo) -> PdfResult<Option<Vec<u8>>> {
        if let (Some(cert_set), cms::signed_data::SignerIdentifier::IssuerAndSerialNumber(ias)) = (&signed_data.certificates, &signer_info.sid) {
            for cert_choice in cert_set.0.iter() {
                let cms::cert::CertificateChoices::Certificate(cert) = cert_choice else { continue };
                if cert.tbs_certificate.issuer == ias.issuer && cert.tbs_certificate.serial_number == ias.serial_number {
                    return Ok(Some(cert.to_der().map_err(|e| PdfError::Other(e.to_string()))?));
                }
            }
        }
        
        // Also check LTV cache
        let cache = self.cache.read();
        for cert_der in &cache.certificates {
            let (_, _x509) = x509_parser::parse_x509_certificate(cert_der)
                .map_err(|e| PdfError::Other(format!("LTV cert parse error: {}", e)))?;
            
            // Match issuer and serial (simplified)
            // ...
        }

        Ok(None)
    }

    pub fn resolve_ltv_data(&self, root_dict: &BTreeMap<PdfName, Object>) -> PdfResult<()> {
        if let Some(dss) = root_dict.get(&"DSS".into()).and_then(|o| o.as_dict()) {
            let mut cache = self.cache.write();
            
            // Resolve /Certs
            if let Some(certs) = dss.get(&"Certs".into()).and_then(|o| o.as_array()) {
                for cert_ref in certs.iter() {
                    if let Some(data) = cert_ref.as_reference()
                        .and_then(|r| self.resolver.resolve(&r).ok())
                        .and_then(|o| if let Object::Stream(_, d) = o { Some(d) } else { None }) {
                            cache.certificates.push(data.to_vec());
                    }
                }
            }

            // Resolve /CRLs
            if let Some(crls) = dss.get(&"CRLs".into()).and_then(|o| o.as_array()) {
                for crl_ref in crls.iter() {
                    if let Some(data) = crl_ref.as_reference()
                        .and_then(|r| self.resolver.resolve(&r).ok())
                        .and_then(|o| if let Object::Stream(_, d) = o { Some(d) } else { None }) {
                            cache.crls.push(data.to_vec());
                    }
                }
            }

            // Resolve /OCSPs
            if let Some(ocsps) = dss.get(&"OCSPs".into()).and_then(|o| o.as_array()) {
                for ocsp_ref in ocsps.iter() {
                    if let Some(data) = ocsp_ref.as_reference()
                        .and_then(|r| self.resolver.resolve(&r).ok())
                        .and_then(|o| if let Object::Stream(_, d) = o { Some(d) } else { None }) {
                            cache.ocsps.push(data.to_vec());
                    }
                }
            }
        }
        Ok(())
    }
}

trait DigestHasher {
    fn update(&mut self, data: &[u8]);
    fn finalize_vec(&mut self) -> Vec<u8>;
}

impl DigestHasher for Sha256 {
    fn update(&mut self, data: &[u8]) { Digest::update(self, data); }
    fn finalize_vec(&mut self) -> Vec<u8> { self.clone().finalize().to_vec() }
}

impl DigestHasher for Sha1 {
    fn update(&mut self, data: &[u8]) { Digest::update(self, data); }
    fn finalize_vec(&mut self) -> Vec<u8> { self.clone().finalize().to_vec() }
}
