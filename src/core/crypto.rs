//! AES decryption for KGC API responses

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes128;
use base64::{engine::general_purpose, Engine as _};
use thiserror::Error;

use super::kgc::AES_KEY;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key length: expected 16 bytes, got {0}")]
    InvalidKeyLength(usize),
    #[error("Hex decode error: {0}")]
    HexDecodeError(String),
    #[error("Not base64 encoded (possibly plain text)")]
    NotBase64,
    #[error("Ciphertext length must be multiple of 16 bytes")]
    InvalidCiphertextLength,
}

/// Decrypt AES-ECB encrypted data from base64 using KGC key
pub fn decrypt_base64(ciphertext_b64: &str) -> Result<String, CryptoError> {
    decrypt_base64_with_key(ciphertext_b64, AES_KEY.as_bytes())
}

/// Decrypt raw AES-ECB encrypted bytes using KGC key
pub fn decrypt_raw(ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    decrypt_raw_with_key(ciphertext, AES_KEY.as_bytes())
}

/// Decrypt AES-ECB encrypted data from hex using KGC key
pub fn decrypt_hex(ciphertext_hex: &str) -> Result<String, CryptoError> {
    decrypt_hex_with_key(ciphertext_hex, AES_KEY.as_bytes())
}

/// Decrypt AES-ECB encrypted data from base64 with custom key
pub fn decrypt_base64_with_key(ciphertext_b64: &str, key_bytes: &[u8]) -> Result<String, CryptoError> {
    if key_bytes.len() != 16 {
        return Err(CryptoError::InvalidKeyLength(key_bytes.len()));
    }

    let trimmed = ciphertext_b64.trim();

    let ciphertext = general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|_| CryptoError::NotBase64)?;

    decrypt_ecb(&ciphertext, key_bytes)
}

/// Decrypt AES-ECB encrypted data from hex with custom key
pub fn decrypt_hex_with_key(ciphertext_hex: &str, key_bytes: &[u8]) -> Result<String, CryptoError> {
    if key_bytes.len() != 16 {
        return Err(CryptoError::InvalidKeyLength(key_bytes.len()));
    }

    let trimmed = ciphertext_hex.trim();
    let ciphertext = hex::decode(trimmed)
        .map_err(|e| CryptoError::HexDecodeError(e.to_string()))?;

    decrypt_ecb(&ciphertext, key_bytes)
}

/// Decrypt raw AES-ECB encrypted bytes with custom key
pub fn decrypt_raw_with_key(ciphertext: &[u8], key_bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key_bytes.len() != 16 {
        return Err(CryptoError::InvalidKeyLength(key_bytes.len()));
    }

    if ciphertext.len() % 16 != 0 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    let key = GenericArray::from_slice(key_bytes);
    let cipher = Aes128::new(key);

    let mut decrypted = ciphertext.to_vec();
    for chunk in decrypted.chunks_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }

    let unpadded = match unpad_pkcs7(&decrypted) {
        Some(data) => data.to_vec(),
        None => decrypted,
    };

    Ok(unpadded)
}

/// Core AES-ECB decryption function
fn decrypt_ecb(ciphertext: &[u8], key: &[u8]) -> Result<String, CryptoError> {
    if ciphertext.len() % 16 != 0 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    let key = GenericArray::from_slice(key);
    let cipher = Aes128::new(key);

    let mut decrypted = ciphertext.to_vec();

    // Decrypt each block
    for chunk in decrypted.chunks_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }

    // Try to unpad PKCS7
    let unpadded = match unpad_pkcs7(&decrypted) {
        Some(data) => data,
        None => &decrypted,
    };

    // Convert to UTF-8, removing null bytes
    let result = String::from_utf8_lossy(unpadded)
        .trim_end_matches('\0')
        .to_string();

    Ok(result)
}

fn unpad_pkcs7(data: &[u8]) -> Option<&[u8]> {
    if data.is_empty() {
        return None;
    }

    let padding_len = data[data.len() - 1] as usize;

    if padding_len == 0 || padding_len > 16 || padding_len > data.len() {
        return None;
    }

    // Verify all padding bytes are the same
    for i in 0..padding_len {
        if data[data.len() - 1 - i] != padding_len as u8 {
            return None;
        }
    }

    Some(&data[..data.len() - padding_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpad_pkcs7() {
        let data = b"Hello World\x05\x05\x05\x05\x05";
        assert_eq!(unpad_pkcs7(data), Some(b"Hello World".as_ref()));
    }

    #[test]
    fn test_key_length_validation() {
        let result = decrypt_base64_with_key("test", b"short");
        assert!(matches!(result, Err(CryptoError::InvalidKeyLength(_))));
    }
}
