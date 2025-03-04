use crate::utils::{KeyComparable, Merge};
use crate::{MSTKey, Reference};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

pub fn create_key(input: &[u8]) -> MSTKey {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize()
}

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct TestValue {
    pub key: MSTKey,
    pub data: [u8; 4],
}

impl AsRef<[u8]> for TestValue {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl Reference for TestValue {
    type Key = MSTKey;
    fn refs(&self) -> Vec<Self::Key> {
        vec![] // Values don't have references
    }
}

// Implement Merge for TestValue
impl Merge for TestValue {
    fn merge(self, other: Self) -> Self {
        // Always take the second value in case of merge
        other
    }
}

impl KeyComparable for TestValue {
    type Key = MSTKey;

    fn compare_keys(key1: &Self::Key, key2: &Self::Key) -> Ordering {
        key1.cmp(key2)
    }
}
