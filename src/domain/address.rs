use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct Address(pub String);

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Address {}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hashing each char to avoid copying the String
        for c in self.0.as_bytes() {
            c.to_ascii_lowercase().hash(state)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::address::Address;
    use std::collections::HashMap;

    #[test]
    fn test_different_addresses() {
        let first_address = Address("0xAlice".to_string());
        let second_address = Address("0xBob".to_string());

        assert_ne!(first_address, second_address);
    }

    #[test]
    fn test_same_address_lower() {
        let first_address = Address("0xAlice".to_string());
        let second_address = Address("0xaLICE".to_string());

        assert_eq!(first_address, second_address);
    }

    #[test]
    fn test_hash_map() {
        let mut map = HashMap::new();

        map.insert(Address("0xAlice".to_string()), "first_value");

        assert!(map.contains_key(&Address("0xAlice".to_string())));
        assert!(map.contains_key(&Address("0xaLICE".to_string())));
        assert!(!map.contains_key(&Address("0xBob".to_string())));
    }
}
