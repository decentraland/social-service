struct EthAddress<'a> {
    address: &'a str,
}

impl EthAddress<'_> {
    pub fn new(address: &str) -> Self {
        EthAddress { address }
    }
}

impl PartialEq for EthAddress<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.address.to_lowercase() == other.address.to_lowercase()
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
