use hex::encode;
use sha2::{Digest, Sha256};

pub fn hash(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let result = hasher.finalize();
    encode(result)
}
