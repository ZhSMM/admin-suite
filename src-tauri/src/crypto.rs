//! AES-GCM encryption for LLM API keys at rest.
//!
//! Master key lives in `<config_dir>/.llm_master_key` with mode 0600 (best
//! effort on Windows). The key is generated on first use and never written
//! to the DB or logs. Cipher format: `nonce(12B) || ciphertext || tag(16B)`,
//! which is what `aes-gcm` produces by default.
//!
//! Per-encryption nonce comes from `OsRng`. Each encrypt call uses a fresh
//! nonce; the same plaintext + same key will produce different ciphertexts,
//! which is what we want.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use zeroize::Zeroize;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const MASTER_KEY_FILENAME: &str = ".llm_master_key";

/// 32-byte master key + path on disk.
#[derive(Clone)]
pub struct MasterKey {
    bytes: [u8; KEY_LEN],
    path: PathBuf,
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl MasterKey {
    /// Load the master key from disk, generating + persisting one if missing.
    pub fn load_or_create(config_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(config_dir)?;
        let path = config_dir.join(MASTER_KEY_FILENAME);
        if path.exists() {
            let bytes = fs::read(&path)?;
            if bytes.len() != KEY_LEN {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("master key has wrong length: {} (expected {})", bytes.len(), KEY_LEN),
                ));
            }
            let mut k = [0u8; KEY_LEN];
            k.copy_from_slice(&bytes);
            Ok(Self { bytes: k, path })
        } else {
            let mut k = [0u8; KEY_LEN];
            OsRng.fill_bytes(&mut k);
            // Best-effort permission tightening. On Windows this is a no-op
            // for the admin user; ACLs are inherited from the parent dir.
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let mut opts = fs::OpenOptions::new();
                opts.write(true).create(true).truncate(true).mode(0o600);
                let mut f = opts.open(&path)?;
                use std::io::Write;
                f.write_all(&k)?;
            }
            #[cfg(not(unix))]
            {
                fs::write(&path, &k)?;
            }
            Ok(Self { bytes: k, path })
        }
    }

    /// Encrypt a plaintext API key. Returns nonce || ciphertext || tag.
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CryptoError> {
        let key = Key::<Aes256Gcm>::from_slice(&self.bytes);
        let cipher = Aes256Gcm::new(key);
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::Encrypt)?;
        let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypt nonce || ciphertext || tag back to plaintext.
    pub fn decrypt(&self, blob: &[u8]) -> Result<String, CryptoError> {
        if blob.len() < NONCE_LEN + 16 {
            return Err(CryptoError::CipherTooShort);
        }
        let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
        let key = Key::<Aes256Gcm>::from_slice(&self.bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);
        let pt = cipher
            .decrypt(nonce, ct)
            .map_err(|_| CryptoError::Decrypt)?;
        String::from_utf8(pt).map_err(|_| CryptoError::Utf8)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
    #[error("ciphertext too short")]
    CipherTooShort,
    #[error("decrypted bytes are not valid UTF-8")]
    Utf8,
}

#[allow(dead_code)] // surface for callers that want richer errors; currently wrapped by LlmError::Internal
impl CryptoError {
    pub fn category(&self) -> &'static str {
        match self {
            CryptoError::Encrypt => "encrypt",
            CryptoError::Decrypt => "decrypt",
            CryptoError::CipherTooShort => "cipher_too_short",
            CryptoError::Utf8 => "utf8",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_key() -> MasterKey {
        let dir = env::temp_dir().join(format!(
            "admin-suite-crypto-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        MasterKey::load_or_create(&dir).expect("create master key")
    }

    #[test]
    fn round_trip() {
        let k = tmp_key();
        let plaintext = "sk-abc1234567890XYZ";
        let ct = k.encrypt(plaintext).expect("encrypt");
        let pt = k.decrypt(&ct).expect("decrypt");
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn encrypt_is_nondeterministic() {
        let k = tmp_key();
        let a = k.encrypt("same input").unwrap();
        let b = k.encrypt("same input").unwrap();
        assert_ne!(a, b, "nonce must vary");
        assert_eq!(k.decrypt(&a).unwrap(), k.decrypt(&b).unwrap());
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let k1 = tmp_key();
        let k2 = tmp_key();
        let ct = k1.encrypt("secret").unwrap();
        assert!(k2.decrypt(&ct).is_err());
    }

    #[test]
    fn truncated_ciphertext_rejected() {
        let k = tmp_key();
        let ct = k.encrypt("secret").unwrap();
        assert!(matches!(k.decrypt(&ct[..5]), Err(CryptoError::CipherTooShort)));
    }

    #[test]
    fn persisted_key_reused() {
        let dir = env::temp_dir().join(format!(
            "admin-suite-crypto-persist-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let k1 = MasterKey::load_or_create(&dir).expect("first load");
        let ct = k1.encrypt("persist me").unwrap();
        // Reload from same path; should decrypt correctly.
        let k2 = MasterKey::load_or_create(&dir).expect("second load");
        assert_eq!(k2.decrypt(&ct).unwrap(), "persist me");
    }
}