use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct Address {
    address: String,
}

impl Address {
    pub fn new(address: String) -> Self {
        Address { address }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.address.to_lowercase() == other.address.to_lowercase()
    }
}

impl Eq for Address {}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.to_lowercase().hash(state);
    }
}

#[test]
fn test_different_addresses() {
    let first_address = Address::new("0xAlice".to_string());
    let second_address = Address::new("0xBob".to_string());

    assert_ne!(first_address, second_address);
}

#[test]
fn test_same_address_lower() {
    let first_address = Address::new("0xAlice".to_string());
    let second_address = Address::new("0xaLICE".to_string());

    assert_eq!(first_address, second_address);
}

#[test]
fn test_hash_map() {
    let mut map = std::collections::HashMap::new();

    map.insert(Address::new("0xAlice".to_string()), "first_value");

    assert!(map.contains_key(&Address::new("0xAlice".to_string())));
    assert!(map.contains_key(&Address::new("0xaLICE".to_string())));
    assert!(!map.contains_key(&Address::new("0xBob".to_string())));
}
