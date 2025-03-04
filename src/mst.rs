use crate::calc_level;
use crate::store::{Page, PageData};
use crate::utils::KeyComparable;
use crate::utils::Merge;
use crate::{MSTKey, Reference, Store};
use sha2::Digest;
use sha2::Sha256;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

/// A Merkle Search Tree combines properties of search trees with content-addressable storage,
/// providing efficient lookups while cryptographically verifying content.
///
/// # Key Features
/// - Content-addressed via SHA-256 hashes
/// - Self-balancing structure
/// - Efficient search and insertion
/// - Tree merging support
///
/// # Type Parameters
/// * `Value`: Must implement `Hash`, `Debug`, `AsRef<[u8]>`, `Reference`, `Copy`, and `Merge`
pub struct MST<Value: Hash + std::fmt::Debug + KeyComparable<Key = MSTKey>> {
    /// The hash key of the root node
    pub root: MSTKey,
    /// Content-addressable storage mapping hash keys to pages
    pub store: Store<MSTKey, Page<MSTKey, Value>>,
}

impl<
    Value: AsRef<[u8]>
        + Hash
        + Reference<Key = MSTKey>
        + Copy
        + std::fmt::Debug
        + Merge
        + KeyComparable<Key = MSTKey>,
> MST<Value>
{
    /// Creates a new empty MST with the default root key
    ///
    /// # Example
    /// ```
    /// use mst::{MST};
    /// use mst::test_utils::TestValue;
    ///
    /// let mst: MST<TestValue> = MST::new();
    /// ```
    pub fn new() -> Self {
        Self {
            root: MSTKey::default(),
            store: Store::new(),
        }
    }

    /// Creates a new empty MST with the specified root key
    ///
    /// # Example
    /// ```
    /// use mst::{MST, MSTKey};
    /// use mst::test_utils::TestValue;
    ///
    /// let root_key = MSTKey::default();
    /// let mst: MST<TestValue> = MST::with_root(root_key);
    /// ```
    pub fn with_root(root_key: MSTKey) -> Self {
        Self {
            root: root_key,
            store: Store::new(),
        }
    }

    /// Creates a new MST with the provided store
    ///
    /// # Arguments
    ///
    /// * `root_key`: The hash key of the root node
    /// * `store`: Pre-existing store of pages
    ///
    /// # Returns
    ///
    /// A new MST instance with the provided store
    pub fn with_store(root_key: MSTKey, store: Store<MSTKey, Page<MSTKey, Value>>) -> Self {
        Self {
            root: root_key,
            store,
        }
    }

    /// Retrieves a page from the store by its key.
    ///
    /// This is a low-level operation that provides direct access to the tree's pages.
    /// Most users should use `get_value()` instead.
    pub fn get(&self, page_key: MSTKey) -> Option<&Page<MSTKey, Value>> {
        self.store.get(page_key)
    }

    /// Converts the tree to a sorted list of values using MST-specific traversal.
    ///
    /// # Example
    /// ```
    /// use mst::{MST, MSTKey};
    /// use mst::test_utils::TestValue;
    ///
    /// let mut mst: MST<TestValue> = MST::new();
    /// let values = mst.to_list();
    /// ```
    pub fn to_list(&self) -> Vec<Value> {
        // Return empty vector if the tree is empty
        if self.root == MSTKey::default() {
            return Vec::new();
        }

        let mut result_values = Vec::new();

        let visitor = |event: TraversalEvent<MSTKey, Value>| {
            if let TraversalEvent::VisitEntry(_, entry) = event {
                result_values.push(entry.value);
            }
            TraversalControl::Continue
        };

        // Use MST-specific traversal order
        self.traverse_tree(TraversalStrategy::MSTOrder, visitor);
        result_values
    }

    /// Inserts a new key-value pair into the tree.
    ///
    /// The insertion process maintains the tree's ordered structure and balance.
    /// If the key already exists, the values will be merged using the `Merge` trait.
    ///
    /// # Example
    /// ```
    /// use mst::{MST, MSTKey};
    /// use mst::test_utils::TestValue;
    ///
    /// let mut mst: MST<TestValue> = MST::new();
    /// let key = MSTKey::default();
    /// let value = TestValue { key, data: [0; 4] };
    /// mst.insert(key, value);
    /// ```
    pub fn insert(&mut self, item_key: MSTKey, item_value: Value) -> MSTKey {
        let level = calc_level(&item_key);
        let (new_root, _) = self.insert_at(self.root, item_key, item_value, level);
        self.root = new_root;
        self.root
    }

    /// Helper function that recursively inserts an item at the specified level.
    /// Returns the new root key of the affected subtree and any modified store.
    fn insert_at(
        &mut self,
        current_root: MSTKey,
        item_key: MSTKey,
        item_value: Value,
        level: u32,
    ) -> (MSTKey, bool) {
        // Handle empty tree or non-existent root
        if current_root == MSTKey::default() || !self.store.has(current_root) {
            let new_page = Page {
                level,
                low: None,
                list: vec![PageData {
                    key: item_key,
                    value: item_value,
                    next: None,
                }],
            };
            let new_key = hash_page(&new_page);
            self.store.put(new_key, new_page);
            return (new_key, true);
        }

        // Get the current page
        let current_page = self.store.get(current_root).cloned().unwrap();

        // Case 1: Current level is less than item level
        if current_page.level < level {
            // We need to create a new node at a higher level
            // First split the tree at our insertion point
            let (low_key, high_key) = self.split(Some(current_root), item_key);

            // Create a new page with our item between the split parts
            let new_page = Page {
                level,
                low: low_key,
                list: vec![PageData {
                    key: item_key,
                    value: item_value,
                    next: high_key,
                }],
            };
            let new_key = hash_page(&new_page);
            self.store.put(new_key, new_page);
            return (new_key, true);
        }
        // Case 2: Current level is equal to item level
        else if current_page.level == level {
            let mut new_page = Page {
                level: current_page.level,
                low: current_page.low,
                list: Vec::with_capacity(current_page.list.len() + 1),
            };

            // Handle empty list case
            if current_page.list.is_empty() {
                new_page.list.push(PageData {
                    key: item_key,
                    value: item_value,
                    next: None,
                });
            } else {
                let first_key = current_page.list[0].key;

                if Value::compare_keys(&item_key, &first_key) == Ordering::Less {
                    // Item belongs before the first element
                    let (low2a, low2b) = self.split(current_page.low, item_key);

                    // Create new list starting with our item
                    new_page.list.push(PageData {
                        key: item_key,
                        value: item_value,
                        next: low2b,
                    });

                    // Add the rest of the original list
                    new_page.list.extend(current_page.list);
                    new_page.low = low2a;
                } else {
                    // Item belongs after the first element
                    new_page.list =
                        self.insert_after_first(&current_page.list, item_key, item_value);
                }
            }

            let new_key = hash_page(&new_page);
            self.store.put(new_key, new_page);
            return (new_key, true);
        }
        // Case 3: Current level is greater than item level
        else {
            if current_page.list.is_empty() {
                // No items in this node, insert into low child
                let low_key = current_page.low;
                let (new_low_key, low_modified) = match low_key {
                    Some(key) => self.insert_at(key, item_key, item_value, level),
                    None => {
                        // Create a new page for the item
                        let new_page = Page {
                            level,
                            low: None,
                            list: vec![PageData {
                                key: item_key,
                                value: item_value,
                                next: None,
                            }],
                        };
                        let new_key = hash_page(&new_page);
                        self.store.put(new_key, new_page);
                        (new_key, true)
                    }
                };

                // Only create a new page if the child was modified
                if low_modified {
                    let mut new_page = current_page.clone();
                    new_page.low = Some(new_low_key);
                    let new_key = hash_page(&new_page);
                    self.store.put(new_key, new_page);
                    return (new_key, true);
                } else {
                    return (current_root, false);
                }
            }

            let first_key = current_page.list[0].key;

            if Value::compare_keys(&item_key, &first_key) == Ordering::Less {
                // Key is less than first entry - go to low child
                let low_key = current_page.low;
                let (new_low_key, low_modified) = match low_key {
                    Some(key) => self.insert_at(key, item_key, item_value, level),
                    None => {
                        // Create a new page for the item
                        let new_page = Page {
                            level,
                            low: None,
                            list: vec![PageData {
                                key: item_key,
                                value: item_value,
                                next: None,
                            }],
                        };
                        let new_key = hash_page(&new_page);
                        self.store.put(new_key, new_page);
                        (new_key, true)
                    }
                };

                // Only create a new page if the child was modified
                if low_modified {
                    let mut new_page = current_page.clone();
                    new_page.low = Some(new_low_key);
                    let new_key = hash_page(&new_page);
                    self.store.put(new_key, new_page);
                    return (new_key, true);
                } else {
                    return (current_root, false);
                }
            } else {
                // Find where the item belongs in the list
                let mut new_page = current_page.clone();
                let modified =
                    self.insert_into_list(&mut new_page.list, item_key, item_value, level);

                // Only create a new page if a child was modified
                if modified {
                    let new_key = hash_page(&new_page);
                    self.store.put(new_key, new_page);
                    return (new_key, true);
                } else {
                    return (current_root, false);
                }
            }
        }
    }

    /// Helper function to insert an item into the right position in a list
    /// Returns true if any modifications were made
    fn insert_into_list(
        &mut self,
        list: &mut Vec<PageData<MSTKey, Value>>,
        item_key: MSTKey,
        item_value: Value,
        level: u32,
    ) -> bool {
        for i in 0..list.len() {
            // Check if the item belongs before this entry
            if i < list.len() - 1
                && Value::compare_keys(&item_key, &list[i + 1].key) == Ordering::Less
            {
                // Item belongs between entries i and i+1
                let next_key = list[i].next;
                let (new_next_key, next_modified) = match next_key {
                    Some(key) => self.insert_at(key, item_key, item_value, level),
                    None => {
                        // Create a new page for the item
                        let new_page = Page {
                            level,
                            low: None,
                            list: vec![PageData {
                                key: item_key,
                                value: item_value,
                                next: None,
                            }],
                        };
                        let new_key = hash_page(&new_page);
                        self.store.put(new_key, new_page);
                        (new_key, true)
                    }
                };

                // Only update the list if the child was modified
                if next_modified {
                    list[i].next = Some(new_next_key);
                    return true;
                } else {
                    return false;
                }
            }
        }

        // If we get here, the item belongs after the last entry
        let last_idx = list.len() - 1;
        let next_key = list[last_idx].next;
        let (new_next_key, next_modified) = match next_key {
            Some(key) => self.insert_at(key, item_key, item_value, level),
            None => {
                // Create a new page for the item
                let new_page = Page {
                    level,
                    low: None,
                    list: vec![PageData {
                        key: item_key,
                        value: item_value,
                        next: None,
                    }],
                };
                let new_key = hash_page(&new_page);
                self.store.put(new_key, new_page);
                (new_key, true)
            }
        };

        // Only update the list if the child was modified
        if next_modified {
            list[last_idx].next = Some(new_next_key);
            return true;
        }

        false
    }

    /// Helper function to insert a key-value pair after the first entry in a list
    fn insert_after_first(
        &mut self,
        entries: &[PageData<MSTKey, Value>],
        item_key: MSTKey,
        item_value: Value,
    ) -> Vec<PageData<MSTKey, Value>> {
        if entries.is_empty() {
            return Vec::new();
        }

        let mut result_entries = Vec::with_capacity(entries.len() + 1);
        let mut current_idx = 0;

        while current_idx < entries.len() {
            let entry = &entries[current_idx];

            match Value::compare_keys(&entry.key, &item_key) {
                Ordering::Equal => {
                    // Key already exists - merge values
                    let merged_value = entry.value.merge(item_value);
                    result_entries.push(PageData {
                        key: entry.key,
                        value: merged_value,
                        next: entry.next,
                    });
                    // Append the rest of the entries
                    for i in (current_idx + 1)..entries.len() {
                        result_entries.push(entries[i].clone());
                    }
                    break;
                }
                Ordering::Less => {
                    if current_idx == entries.len() - 1
                        || Value::compare_keys(&item_key, &entries[current_idx + 1].key)
                            == Ordering::Less
                    {
                        // Insert between current entry and next entry
                        let (left_subtree, right_subtree) = self.split(entry.next, item_key);
                        result_entries.push(PageData {
                            key: entry.key,
                            value: entry.value.clone(),
                            next: left_subtree,
                        });
                        result_entries.push(PageData {
                            key: item_key,
                            value: item_value,
                            next: right_subtree,
                        });
                        // Append the rest
                        for i in (current_idx + 1)..entries.len() {
                            result_entries.push(entries[i].clone());
                        }
                        break;
                    } else {
                        // Not the right spot yet, keep current entry and continue
                        result_entries.push(entries[current_idx].clone());
                        current_idx += 1;
                    }
                }
                Ordering::Greater => {
                    // This should never happen in insert_after_first
                    panic!("Unexpected order in insert_after_first");
                }
            }
        }

        result_entries
    }

    /// Splits the tree into two parts at the given key.
    ///
    /// This operation divides the tree into two separate subtrees:
    /// - Left subtree: Contains all keys strictly less than split_key
    /// - Right subtree: Contains all keys greater than or equal to split_key
    ///
    /// # Arguments
    /// * `node_key_opt`: Optional key to the node where splitting starts
    /// * `split_key`: The key value at which to split the tree
    ///
    /// # Returns
    /// A tuple of (left_subtree_key, right_subtree_key), both optional
    fn split(
        &mut self,
        node_key_opt: Option<MSTKey>,
        split_key: MSTKey,
    ) -> (Option<MSTKey>, Option<MSTKey>) {
        // Early return for empty or default trees - nothing to split
        if node_key_opt.is_none() || node_key_opt == Some(MSTKey::default()) {
            return (None, None);
        }

        let node_key = node_key_opt.unwrap();
        let current_page = match self.store.get(node_key).cloned() {
            Some(page) => page,
            None => return (None, None),
        };

        // Remove the current page as we'll be creating new pages with its content
        self.store.remove(node_key);

        let level = current_page.level;
        let low_child = current_page.low;
        let entries = current_page.list;

        // If page has no entries, the left result is just the low branch
        if entries.is_empty() {
            return (low_child, None);
        }

        let first_entry = &entries[0];

        // Compare split key with first entry to determine how to split
        match Value::compare_keys(&split_key, &first_entry.key) {
            Ordering::Less => {
                // Split key is less than first entry - need to split the low branch
                // and move all entries to the right subtree
                let (lowlow, lowhi) = self.split(low_child, split_key);

                // Create right page with all the original entries
                let right_page_key = self.create_and_store_page(level, lowhi, entries.clone());

                (lowlow, Some(right_page_key))
            }
            _ => {
                // Split key is greater than or equal to first entry
                // We'll process entries one by one to determine where the split occurs
                let mut left_entries = Vec::new();
                let mut right_result = None;

                // Process entries
                let mut i = 0;
                while i < entries.len() {
                    let entry = &entries[i];

                    if i < entries.len() - 1
                        && Value::compare_keys(&split_key, &entries[i + 1].key) == Ordering::Less
                    {
                        // We found the split point: between current entry and next entry
                        // Current entry goes to left, entries after it go to right
                        let (next_left, next_right) = self.split(entry.next, split_key);

                        // Add current entry to the left part with the proper next pointer
                        left_entries.push(PageData {
                            key: entry.key,
                            value: entry.value,
                            next: next_left,
                        });

                        // Create right page with all remaining entries
                        let mut right_entries = Vec::new();
                        for j in i + 1..entries.len() {
                            right_entries.push(entries[j].clone());
                        }

                        let right_page_key =
                            self.create_and_store_page(level, next_right, right_entries);

                        right_result = Some(right_page_key);
                        break;
                    }

                    if i == entries.len() - 1 {
                        // We've reached the last entry - need to split its next branch
                        let (next_left, next_right) = self.split(entry.next, split_key);

                        // Add the last entry to the left subtree
                        left_entries.push(PageData {
                            key: entry.key,
                            value: entry.value,
                            next: next_left,
                        });

                        right_result = next_right;
                        break;
                    }

                    // Current entry belongs fully in the left part, continue to next
                    left_entries.push(entry.clone());
                    i += 1;
                }

                // Create left page
                let left_page = Page {
                    level,
                    low: low_child,
                    list: left_entries,
                };

                let left_page_key = hash_page(&left_page);
                self.store.put(left_page_key, left_page);

                (Some(left_page_key), right_result)
            }
        }
    }

    /// Merges this MST with another MST, combining their contents.
    ///
    /// This operation creates a new tree that contains all items from both trees,
    /// properly handling duplicate keys by using the Merge trait to combine values.
    /// The merge operation preserves the cryptographic properties of both trees.
    ///
    /// # Example
    /// ```
    /// use mst::{MST, MSTKey};
    /// use mst::test_utils::TestValue;
    ///
    /// let mut mst1: MST<TestValue> = MST::new();
    /// let mst2: MST<TestValue> = MST::new();
    /// let (merged_root, merged_store) = mst1.merge(&mst2);
    /// ```
    pub fn merge(&mut self, other: &Self) -> (MSTKey, Store<MSTKey, Page<MSTKey, Value>>) {
        // Create a new empty MST
        let mut new_mst = MST::new();

        // Add all items from both trees directly, with proper merging
        if self.root != MSTKey::default() {
            self.add_items_to_mst(&mut new_mst);
        }

        if other.root != MSTKey::default() {
            other.add_items_to_mst(&mut new_mst);
        }

        (new_mst.root, new_mst.store)
    }

    /// Helper function to add all items from this MST to another MST
    fn add_items_to_mst(&self, target: &mut MST<Value>) {
        if self.root == MSTKey::default() {
            return;
        }

        let visitor = |event: TraversalEvent<MSTKey, Value>| {
            if let TraversalEvent::VisitEntry(_, entry) = event {
                target.insert(entry.key, entry.value);
            }
            TraversalControl::Continue
        };

        // Use MST-specific traversal order
        self.traverse_tree(TraversalStrategy::MSTOrder, visitor);
    }

    /// Get a specific value by key from the tree
    ///
    /// # Arguments
    ///
    /// * `search_key`: The key to search for
    ///
    /// # Returns
    ///
    /// Option containing the value if found, None otherwise
    pub fn get_value(&self, search_key: MSTKey) -> Option<Value> {
        // Start from the root
        self.get_value_from_node(self.root, search_key)
    }

    /// Helper function to search for a value starting from a specific node
    fn get_value_from_node(&self, node_key: MSTKey, search_key: MSTKey) -> Option<Value> {
        // Return None for empty tree
        if node_key == MSTKey::default() {
            return None;
        }

        // Get the page for this node
        let page = match self.store.get(node_key) {
            Some(p) => p,
            None => return None,
        };

        // Check low branch if list is empty
        if page.list.is_empty() {
            return match page.low {
                Some(low_key) => self.get_value_from_node(low_key, search_key),
                None => None,
            };
        }

        // Process the list of entries
        for i in 0..page.list.len() {
            let entry = &page.list[i];

            match Value::compare_keys(&search_key, &entry.key) {
                // Found the key
                Ordering::Equal => return Some(entry.value),

                // Search key is less than current entry, go to low branch
                Ordering::Less => {
                    if i == 0 {
                        // If this is the first entry, check the low branch
                        return match page.low {
                            Some(low_key) => self.get_value_from_node(low_key, search_key),
                            None => None,
                        };
                    } else {
                        // Otherwise, check the previous entry's next branch
                        return match page.list[i - 1].next {
                            Some(next_key) => self.get_value_from_node(next_key, search_key),
                            None => None,
                        };
                    }
                }

                // Search key is greater, continue to next entry or check this entry's next branch
                Ordering::Greater => {
                    if i == page.list.len() - 1 {
                        // This is the last entry, check its next branch
                        return match entry.next {
                            Some(next_key) => self.get_value_from_node(next_key, search_key),
                            None => None,
                        };
                    }
                    // Otherwise continue to next entry
                }
            }
        }

        // If we reach here, key wasn't found
        None
    }

    /// Debug function to dump the tree structure
    ///
    /// # Returns
    ///
    /// A string representation of the tree
    pub fn dump(&self) -> String {
        if self.root == MSTKey::default() {
            return String::new();
        }

        let mut output = String::new();
        let mut depth_map = HashMap::new();
        depth_map.insert(self.root, 0);

        let visitor = |event: TraversalEvent<MSTKey, Value>| {
            match event {
                TraversalEvent::VisitNode(node_key, page) => {
                    let depth = depth_map.get(&node_key).copied().unwrap_or(0);
                    let indent = "  ".repeat(depth);
                    output.push_str(&format!("{}{:?} ({})\n", indent, node_key, page.level));

                    // Store depths for children
                    if let Some(low) = page.low {
                        depth_map.insert(low, depth + 1);
                    }

                    for entry in &page.list {
                        if let Some(next) = entry.next {
                            depth_map.insert(next, depth + 1);
                        }
                    }

                    TraversalControl::Continue
                }
                TraversalEvent::VisitEntry(node_key, entry) => {
                    let depth = depth_map.get(&node_key).copied().unwrap_or(0);
                    let indent = "  ".repeat(depth);
                    output.push_str(&format!(
                        "{}- {:?} => {:?}\n",
                        indent, node_key, entry.value
                    ));
                    TraversalControl::Continue
                }
                _ => TraversalControl::Continue,
            }
        };

        self.traverse_tree(TraversalStrategy::DepthFirst, visitor);
        output
    }

    /// Creates and stores a page, returning its key
    fn create_and_store_page(
        &mut self,
        level: u32,
        low: Option<MSTKey>,
        entries: impl IntoIterator<Item = PageData<MSTKey, Value>>,
    ) -> MSTKey {
        let list = entries.into_iter().collect();
        let new_page = Page { level, low, list };
        let new_page_key = hash_page(&new_page);
        self.store.put(new_page_key, new_page);
        new_page_key
    }

    /// General-purpose tree traversal method that can be used by multiple functions
    fn traverse_tree<F>(&self, strategy: TraversalStrategy, mut visitor: F)
    where
        F: FnMut(TraversalEvent<MSTKey, Value>) -> TraversalControl<()>,
    {
        // Start from root
        let start_key = self.root;
        let mut visited = HashSet::new();

        // Choose traversal strategy
        match strategy {
            TraversalStrategy::DepthFirst => {
                self.depth_first_traverse(start_key, &mut visitor, &mut visited);
            }
            TraversalStrategy::MSTOrder => {
                self.mst_order_traverse(start_key, &mut visitor, &mut visited);
            }
        }
    }

    // And update traversal methods to return ()
    fn depth_first_traverse<F>(&self, start: MSTKey, visitor: &mut F, visited: &mut HashSet<MSTKey>)
    where
        F: FnMut(TraversalEvent<MSTKey, Value>) -> TraversalControl<()>,
    {
        if start == MSTKey::default() || visited.contains(&start) {
            return;
        }

        visited.insert(start);

        if let Some(page) = self.get(start) {
            // Visit node
            match visitor(TraversalEvent::VisitNode(start, page)) {
                TraversalControl::Return(()) => return,
                TraversalControl::Skip => {
                    visitor(TraversalEvent::ExitNode(start));
                    return;
                }
                TraversalControl::Continue => {}
            }

            // Process low child
            if let Some(low_key) = page.low {
                self.depth_first_traverse(low_key, visitor, visited);
            }

            // Process entries
            for entry in page.list.iter() {
                // Visit entry
                match visitor(TraversalEvent::VisitEntry(start, entry)) {
                    TraversalControl::Return(()) => return,
                    TraversalControl::Skip => continue,
                    TraversalControl::Continue => {}
                }

                // Process next pointer
                if let Some(next_key) = entry.next {
                    self.depth_first_traverse(next_key, visitor, visited);
                }
            }

            // Exit node
            visitor(TraversalEvent::ExitNode(start));
        }
    }

    /// Specific traversal for MST-ordered values that preserves the sorted order of keys.
    ///
    /// Unlike traditional tree traversals, MST Order follows the specific Merkle Search Tree
    /// structure to visit nodes in strict key order:
    ///
    /// ```text
    ///        [Entry1, Entry2, Entry3]
    ///        /       |       |      \
    ///     Low     Next1    Next2   Next3
    /// ```
    ///
    /// Where the traversal visits regions in this sequence:
    /// - Low: all keys < Entry1
    /// - Entry1
    /// - Next1: keys between Entry1 and Entry2
    /// - Entry2
    /// - Next2: keys between Entry2 and Entry3
    /// - Entry3
    /// - Next3: all keys > Entry3
    ///
    /// This ensures keys are visited in strictly ascending order - a fundamental
    /// requirement for many MST operations.
    fn mst_order_traverse<F>(&self, start: MSTKey, visitor: &mut F, visited: &mut HashSet<MSTKey>)
    where
        F: FnMut(TraversalEvent<MSTKey, Value>) -> TraversalControl<()>,
    {
        if start == MSTKey::default() || visited.contains(&start) {
            return;
        }

        visited.insert(start);

        if let Some(page) = self.get(start) {
            // Process according to the original collect_values algorithm
            // First, visit low child
            if let Some(low_key) = page.low {
                self.mst_order_traverse(low_key, visitor, visited);
            }

            // Visit node
            match visitor(TraversalEvent::VisitNode(start, page)) {
                TraversalControl::Return(()) => return,
                TraversalControl::Skip => return,
                TraversalControl::Continue => {}
            }

            // Process entries in order (with their next pointers)
            for entry in page.list.iter() {
                // Visit entry
                match visitor(TraversalEvent::VisitEntry(start, entry)) {
                    TraversalControl::Return(()) => return,
                    TraversalControl::Skip => continue,
                    TraversalControl::Continue => {}
                }

                // Process next pointer before moving to next entry
                if let Some(next_key) = entry.next {
                    self.mst_order_traverse(next_key, visitor, visited);
                }
            }

            visitor(TraversalEvent::ExitNode(start));
        }
    }
}

/// Defines different traversal strategies for navigating the tree structure
enum TraversalStrategy {
    /// Depth-first traversal visits nodes before their children, providing a
    /// comprehensive view of the tree structure in pre-order
    DepthFirst,

    /// MST Order traverses the tree in key-sorted order, essential for operations
    /// that need to process keys sequentially.
    ///
    /// The traversal sequence follows the MST structure:
    /// 1. Visit low child (keys < first entry)
    /// 2. Visit the node itself
    /// 3. For each entry in order:
    ///    a. Visit the entry
    ///    b. Visit entry's "next" branch (keys between current and next entry)
    ///    c. Continue to next entry
    ///
    /// This ensures we visit values in strictly ascending key order, which is
    /// critical for operations like to_list() and merging.
    MSTOrder,
}

/// Events that occur during traversal
enum TraversalEvent<'a, K: Hash, V: Hash> {
    VisitNode(K, &'a Page<K, V>),
    VisitEntry(K, &'a PageData<K, V>),
    ExitNode(K),
}

/// Controls how traversal should proceed
#[allow(dead_code)]
enum TraversalControl<R = ()> {
    Continue,
    Skip,      // Skip children of current node
    Return(R), // Return early with a value
}

impl<K: Hash, V: Hash> Page<K, V> {
    /// Creates a new page with the given properties
    ///
    /// # Arguments
    ///
    /// * `level`: The level of the page
    /// * `low`: The low child of the page
    /// * `entries`: The entries to store in the page, as tuples of (key, value, next)
    ///
    /// # Returns
    ///
    /// A new Page instance
    pub fn new(level: u32, low: Option<K>, entries: Vec<(K, V, Option<K>)>) -> Self {
        // Convert tuple list to PageData list
        let page_entries = entries
            .into_iter()
            .map(|(key, value, next)| PageData { key, value, next })
            .collect();

        Self {
            level,
            low,
            list: page_entries,
        }
    }
}

/// Generates a cryptographic hash key for a page.
///
/// The hash incorporates all content that defines the page:
/// - Page level (height in the tree)
/// - Low child pointer (for keys less than any in this page)
/// - All entries (keys, values, and next pointers)
///
/// This content-based addressing ensures that any change to the page content,
/// no matter how small, results in a completely different hash - the foundation
/// of the Merkle tree's ability to verify content integrity.
pub fn hash_page<K: AsRef<[u8]> + Hash, V: AsRef<[u8]> + Hash>(page: &Page<K, V>) -> MSTKey {
    let mut hasher = Sha256::new();
    hasher.update(&page.level.to_be_bytes());
    if let Some(ref low) = page.low {
        hasher.update(low.as_ref() as &[u8]);
    }
    for item in &page.list {
        hasher.update(item.key.as_ref());
        hasher.update(item.value.as_ref());
        if let Some(ref next) = item.next {
            hasher.update(next.as_ref() as &[u8]);
        }
    }
    hasher.finalize()
}
