use crate::Reference;
use std::collections::{HashMap, HashSet};
use std::convert::AsRef;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct PageData<Key: Hash, Value: Hash> {
    pub key: Key,
    pub value: Value,
    pub next: Option<Key>,
}

#[derive(Clone, Debug)]
pub struct Page<Key: Hash, Value: Hash> {
    pub level: u32,
    pub low: Option<Key>,
    pub list: Vec<PageData<Key, Value>>,
}

pub struct Store<Key: AsRef<[u8]>, Value> {
    pages: HashMap<Key, Value>,
}

impl<Key: AsRef<[u8]> + Eq + Hash + Copy, Value: Reference<Key = Key>> Store<Key, Value> {
    pub fn new() -> Self {
        Store {
            pages: HashMap::new(),
        }
    }

    pub fn put(&mut self, key: Key, value: Value) -> Key {
        self.pages.insert(key, value);
        key
    }

    pub fn get(&self, key: Key) -> Option<&Value> {
        self.pages.get(&key)
    }

    pub fn has(&self, key: Key) -> bool {
        self.pages.contains_key(&key)
    }

    pub fn remove(&mut self, key: Key) {
        self.pages.remove(&key);
    }

    pub fn missing_set(&self, root: Key) -> HashSet<Key> {
        let mut result = HashSet::new();
        let mut to_visit = Vec::new(); // Stack for DFS
        let mut visited = HashSet::new(); // Track visited nodes

        // Start with the root
        to_visit.push(root);

        // Process nodes in DFS order
        while let Some(hash) = to_visit.pop() {
            // Skip if already visited
            if !visited.insert(hash) {
                continue;
            }

            match self.pages.get(&hash) {
                None => {
                    // This hash is missing - add to result
                    result.insert(hash);
                }
                Some(page) => {
                    // Add all unvisited references to the stack
                    for ref_hash in page.refs() {
                        if !visited.contains(&ref_hash) {
                            to_visit.push(ref_hash);
                        }
                    }
                }
            }
        }

        result
    }

    /// Provides an iterator over the key-value pairs in the store
    pub fn iter(&self) -> std::collections::hash_map::Iter<Key, Value> {
        self.pages.iter()
    }
}

impl<Key: AsRef<[u8]> + Eq + Hash + Copy, Value: Hash + Reference<Key = Key>> Reference
    for Page<Key, Value>
{
    type Key = <Value as Reference>::Key;
    fn refs(&self) -> Vec<Self::Key> {
        let mut refs = Vec::new();
        if let Some(low) = self.low {
            refs.push(low);
        }
        for page_data in self.list.iter() {
            if let Some(reference) = page_data.next {
                refs.push(reference);
            }
        }
        refs
    }
}

impl<Key: AsRef<[u8]> + Eq + Hash + Copy, Value: Clone> Clone for Store<Key, Value> {
    fn clone(&self) -> Self {
        let mut new_pages = HashMap::new();
        for (key, value) in &self.pages {
            new_pages.insert(*key, value.clone());
        }
        Store { pages: new_pages }
    }
}
