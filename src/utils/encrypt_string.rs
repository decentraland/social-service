use hex::encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;

// Create alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

pub fn hash_with_key(str: &str, key: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(str.as_bytes());

    // `result` has type `CtOutput` which is a thin wrapper around array of
    // bytes for providing constant time equality check
    let result = mac.finalize();

    // To get underlying array use `into_bytes`, but be careful, since
    // incorrect use of the code value may permit timing attacks which defeats
    // the security provided by the `CtOutput`
    let code_bytes = result.into_bytes();

    encode(code_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_with_same_string() {
        // Should be deterministic for the same string twice
        assert_eq!(
            hash_with_key("test string", "test_key"),
            hash_with_key("test string", "test_key")
        );
    }

    #[test]
    fn test_hash_with_different_string() {
        // Should be deterministic for the same string twice
        assert_ne!(
            hash_with_key("test string", "test_key"),
            hash_with_key("test2 string", "test_key")
        );
    }
}
