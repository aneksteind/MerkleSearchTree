use mst::store::{Page, PageData};
use mst::test_utils::{TestValue, create_key};
use mst::{MSTKey, Store};

/// # Store Tests
///
/// These tests verify the functionality of the content-addressable storage
/// used by the Merkle Search Tree. The store is a critical component that
/// maps hash keys to pages of the tree.

#[test]
fn test_store_basic_operations() {
    // This test verifies the core operations of the store:
    // - put: Add a page to the store
    // - get: Retrieve a page by its key
    // - has: Check if a key exists
    // - remove: Delete a page from the store

    let mut store = Store::<MSTKey, Page<MSTKey, TestValue>>::new();
    let key = create_key(b"test_key");

    // Test initially empty
    assert!(!store.has(key), "A new store should not contain any keys");
    assert!(
        store.get(key).is_none(),
        "Getting a non-existent key should return None"
    );

    // Create a simple empty page
    let page = Page {
        level: 1,
        low: None,
        list: vec![],
    };

    // Test put and get operations
    store.put(key, page);
    assert!(
        store.has(key),
        "After putting a page, the store should have the key"
    );
    assert!(
        store.get(key).is_some(),
        "After putting a page, get should return Some"
    );

    // Test remove operation
    store.remove(key);
    assert!(
        !store.has(key),
        "After removing a page, the store should not have the key"
    );
    assert!(
        store.get(key).is_none(),
        "After removing a page, get should return None"
    );
}

#[test]
fn test_page_references() {
    // This test verifies the reference structure of pages in the store,
    // demonstrating how pages connect to form a tree structure

    let mut store = Store::<MSTKey, Page<MSTKey, TestValue>>::new();

    // Create a root key and some child keys
    let root_key = create_key(b"root");
    let child1_key = create_key(b"child1");
    let child2_key = create_key(b"child2");
    let grandchild_key = create_key(b"grandchild");
    let grandchild_next_key = create_key(b"grandchild_next");

    // Create a grandchild page (leaf node)
    let grandchild_page = Page::<MSTKey, TestValue> {
        level: 3,
        low: None, // Leaf node has no children
        list: vec![],
    };

    // Store the grandchild page
    store.put(grandchild_key, grandchild_page);

    // Create a child page that references the grandchild
    let child1_page = Page {
        level: 2,
        low: None,
        list: vec![PageData {
            key: grandchild_key,
            value: TestValue {
                key: grandchild_key,
                data: [1, 2, 3, 4],
            },
            next: Some(grandchild_next_key), // Points to another node
        }],
    };

    // Store the first child
    store.put(child1_key, child1_page);

    // Create a second child page (sibling)
    let child2_page = Page {
        level: 2,
        low: None,
        list: vec![],
    };

    // Store the second child
    store.put(child2_key, child2_page);

    // Create a root page that references both children
    let root_page = Page {
        level: 1,
        low: Some(child1_key), // Low child points to first child
        list: vec![PageData {
            key: child2_key,
            value: TestValue {
                key: child2_key,
                data: [5, 6, 7, 8],
            },
            next: None,
        }],
    };

    // Store the root
    store.put(root_key, root_page);

    // Verify the reference structure
    let retrieved_root = store.get(root_key).unwrap();
    assert_eq!(
        retrieved_root.low,
        Some(child1_key),
        "Root's low child should point to child1"
    );
    assert_eq!(
        retrieved_root.list[0].key, child2_key,
        "Root's first entry should reference child2"
    );

    let retrieved_child1 = store.get(child1_key).unwrap();
    assert_eq!(
        retrieved_child1.list[0].key, grandchild_key,
        "Child1's first entry should reference grandchild"
    );
    assert_eq!(
        retrieved_child1.list[0].next,
        Some(grandchild_next_key),
        "Child1's first entry next should point to grandchild_next"
    );
}

#[test]
fn test_store_content_addressing() {
    // This test demonstrates how content addressing works in the store,
    // showing that identical content produces identical keys

    let mut store = Store::<MSTKey, Page<MSTKey, TestValue>>::new();

    // Create two identical pages
    let data_key = create_key(b"data");
    let test_value = TestValue {
        key: data_key,
        data: [1, 2, 3, 4],
    };

    let page1 = Page {
        level: 1,
        low: None,
        list: vec![PageData {
            key: data_key,
            value: test_value,
            next: None,
        }],
    };

    let page2 = Page {
        level: 1,
        low: None,
        list: vec![PageData {
            key: data_key,
            value: test_value,
            next: None,
        }],
    };

    // Hash and store both pages
    let key1 = mst::hash_page(&page1);
    let key2 = mst::hash_page(&page2);

    store.put(key1, page1);

    // Verify that identical content produces identical keys
    assert_eq!(
        key1, key2,
        "Identical pages should produce identical hash keys"
    );
    assert!(
        store.has(key2),
        "Store should recognize key2 as existing, even though we only stored with key1"
    );

    // Change the content slightly
    let page3 = Page {
        level: 1,
        low: None,
        list: vec![PageData {
            key: data_key,
            value: TestValue {
                key: data_key,
                data: [1, 2, 3, 5], // Changed last byte
            },
            next: None,
        }],
    };

    let key3 = mst::hash_page(&page3);

    // Verify that different content produces different keys
    assert_ne!(
        key1, key3,
        "Different pages should produce different hash keys"
    );
    assert!(
        !store.has(key3),
        "Store should not recognize key3 as existing"
    );
}
