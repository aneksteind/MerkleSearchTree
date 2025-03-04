use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::hash::Hash;

use crate::Reference;

pub type MSTKey = GenericArray<u8, U32>;

pub fn compare<Key: Ord>(key: Key, key2: Key) -> std::cmp::Ordering {
    std::cmp::Ord::cmp(&key, &key2)
}

// Define the Merge trait
pub trait Merge {
    fn merge(self, other: Self) -> Self;
}

pub fn hash<Key: AsRef<[u8]>>(key: Key) -> impl Hash + IntoIterator<Item = u8> {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.finalize()
}

pub fn calc_level<Key: AsRef<[u8]>>(key: Key) -> u32 {
    let hash = hash(key);
    let mut count = 0;
    for byte in hash.into_iter() {
        let string = &format!("0{:b} ", byte);
        for c in string.chars() {
            if c == '0' {
                count += 1;
            } else {
                break;
            }
        }
    }
    count
}

// Add this newtype wrapper
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct Event(bool);

impl Event {
    pub fn new() -> Self {
        Event(true)
    }
}

impl AsRef<[u8]> for Event {
    fn as_ref(&self) -> &[u8] {
        static TRUE_BYTES: [u8; 1] = [1];
        &TRUE_BYTES
    }
}

impl Reference for Event {
    type Key = MSTKey;
    fn refs(&self) -> Vec<Self::Key> {
        vec![]
    }
}

impl Merge for Event {
    fn merge(self, _other: Self) -> Self {
        Event(true) // Events just exist, no need to merge values
    }
}

// Add this new trait to utils.rs
pub trait KeyComparable {
    type Key;

    fn compare_keys(key1: &Self::Key, key2: &Self::Key) -> Ordering;
}
