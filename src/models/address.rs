use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct Address(pub String);

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl Eq for Address {}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

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
    let mut map = std::collections::HashMap::new();

    map.insert(Address("0xAlice".to_string()), "first_value");

    assert!(map.contains_key(&Address("0xAlice".to_string())));
    assert!(map.contains_key(&Address("0xaLICE".to_string())));
    assert!(!map.contains_key(&Address("0xBob".to_string())));
}
