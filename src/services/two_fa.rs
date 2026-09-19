use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a 6-digit one-time code.
pub fn generate_code() -> String {
    let code: u32 = rand::thread_rng().gen_range(100_000..=999_999);
    code.to_string()
}

/// Hash a code with SHA-256 before storing.
/// We do NOT store plain-text codes in the database.
pub fn hash_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compare a submitted code against the stored hash.
pub fn verify_code(submitted: &str, stored_hash: &str) -> bool {
    hash_code(submitted) == stored_hash
}
