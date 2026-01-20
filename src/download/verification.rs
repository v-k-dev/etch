use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Verify a file's SHA256 checksum
pub fn verify_sha256(path: &Path, expected_hash: &str) -> Result<bool> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let hash_hex = format!("{:x}", result);

    if hash_hex.eq_ignore_ascii_case(expected_hash) {
        Ok(true)
    } else {
        bail!(
            "SHA256 mismatch! Expected: {}, Got: {}",
            expected_hash,
            hash_hex
        )
    }
}

/// Calculate SHA256 hash of a file (for debugging/verification)
pub fn calculate_sha256(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Quick verify if a file exists and matches expected hash
pub fn quick_verify(path: &Path, expected_hash: &str) -> bool {
    // Skip verification if hash is placeholder
    if expected_hash.is_empty() || expected_hash.starts_with("PLACEHOLDER") {
        return path.exists(); // Just check existence
    }
    
    // Verify with hash
    verify_sha256(path, expected_hash).unwrap_or(false)
}
