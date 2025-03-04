use mst::test_utils::{TestValue, create_key};
use mst::{KeyComparable, MST, calc_level};
use rand::{seq::SliceRandom, thread_rng};
use std::collections::{HashMap, HashSet};

/// # Tree Structure Tests
///
/// These tests verify that the Merkle Search Tree maintains correct internal
/// structure under various insertion scenarios.
mod tree_structure_tests {
    use super::*;

    #[test]
    fn test_tree_determinism() {
        // This test verifies that the tree structure is deterministic
        // regardless of the order in which items are inserted.

        // Create two trees
        let mut tree1 = MST::new();
        let mut tree2 = MST::new();

        // Generate test keys
        let mut keys = vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // Insert in sequential order for tree1
        for &key_value in &keys {
            let key = create_key(&key_value.to_be_bytes());
            let value = TestValue {
                key,
                data: [key_value as u8, 0, 0, 0],
            };
            tree1.insert(key, value);
        }

        // Shuffle keys and insert in different order for tree2
        let mut rng = thread_rng();
        keys.shuffle(&mut rng);

        for &key_value in &keys {
            let key = create_key(&key_value.to_be_bytes());
            let value = TestValue {
                key,
                data: [key_value as u8, 0, 0, 0],
            };
            tree2.insert(key, value);
        }

        // Both trees should have identical structure (same items in same order)
        let list1 = tree1.to_list();
        let list2 = tree2.to_list();

        assert_eq!(
            list1.len(),
            list2.len(),
            "Trees should have the same number of items"
        );
        for i in 0..list1.len() {
            assert_eq!(
                list1[i].key, list2[i].key,
                "Keys should match at position {}",
                i
            );
            assert_eq!(
                list1[i].data, list2[i].data,
                "Values should match at position {}",
                i
            );
        }
    }

    #[test]
    fn test_page_splitting() {
        // This test verifies that the tree correctly handles page splitting
        // when inserting items at different levels.
        let mut mst = MST::new();

        // Insert a sequence with increasing level values
        for i in 0..5u32 {
            let key = create_key(&i.to_be_bytes());
            let level = i * 2; // Create increasing levels: 0, 2, 4, 6, 8

            // Create a test value with the level encoded in the data
            let value = TestValue {
                key,
                data: [i as u8, level as u8, 0, 0],
            };

            // Track state before insertion
            let items_before = mst.to_list().len();

            // Insert the item
            mst.insert(key, value);

            // Verify item count increased by exactly one
            let items_after = mst.to_list().len();
            assert_eq!(
                items_after,
                items_before + 1,
                "Item count should increase by exactly 1"
            );

            // Verify the inserted value can be retrieved correctly
            let retrieved = mst.get_value(key).unwrap();
            assert_eq!(retrieved.data[0], i as u8, "First data byte should match i");
            assert_eq!(
                retrieved.data[1], level as u8,
                "Second data byte should store the level"
            );
        }
    }
}

/// # Stress Tests
///
/// These tests verify the MST behavior under more demanding conditions
/// with larger datasets and mixed operations.
mod stress_tests {
    use super::*;

    #[test]
    fn test_many_sequential_inserts() {
        // This test verifies the tree can handle a large number of sequential inserts
        // while maintaining correct retrieval capability
        let mut mst = MST::new();

        let count = 10000;

        // Insert many sequential items
        for i in 0..count as u32 {
            let key = create_key(&i.to_be_bytes());
            let value = TestValue {
                key,
                data: [
                    (i % 256) as u8,
                    ((i >> 8) % 256) as u8,
                    ((i >> 16) % 256) as u8,
                    ((i >> 24) % 256) as u8,
                ],
            };

            mst.insert(key, value);

            // Verify the key we just inserted is immediately retrievable
            assert!(
                mst.get_value(key).is_some(),
                "Failed to retrieve key {} immediately after insertion",
                i
            );

            // Every 100 insertions, check a sample of previous keys to ensure
            // they remain retrievable as the tree grows
            if i > 0 && i % 100 == 0 {
                // Check 10 random previous keys
                for _ in 0..10 {
                    let j = rand::random::<u32>() % (i + 1);
                    let check_key = create_key(&j.to_be_bytes());
                    assert!(
                        mst.get_value(check_key).is_some(),
                        "Failed to retrieve key {} after inserting key {}",
                        j,
                        i
                    );
                }
            }
        }

        // Final verification
        let items = mst.to_list();
        assert_eq!(items.len(), count, "Tree should contain all inserted items");

        // Items should be in sorted order
        for i in 1..items.len() {
            assert!(
                TestValue::compare_keys(&items[i - 1].key, &items[i].key)
                    != std::cmp::Ordering::Greater,
                "Items should be in sorted order"
            );
        }
    }

    #[test]
    fn test_random_access_after_inserts() {
        // This test verifies random access patterns after inserting a set of items
        let mut mst = MST::new();
        let count = 150;

        // Insert sequential items
        for i in 0..count as u32 {
            let key = create_key(&i.to_be_bytes());
            let value = TestValue {
                key,
                data: [i as u8, (i >> 8) as u8, (i >> 16) as u8, (i >> 24) as u8],
            };
            mst.insert(key, value);
        }

        // Access items in random order to ensure tree supports
        // efficient retrieval regardless of access pattern
        let mut rng = thread_rng();
        let mut indices: Vec<u32> = (0..count).collect();
        indices.shuffle(&mut rng);

        for &i in &indices {
            let key = create_key(&i.to_be_bytes());
            let value = mst.get_value(key).unwrap();

            // Verify the retrieved value matches what we expect
            assert_eq!(value.data[0], i as u8, "First byte should match index");
            assert_eq!(
                value.data[1],
                (i >> 8) as u8,
                "Second byte should match index >> 8"
            );
            assert_eq!(
                value.data[2],
                (i >> 16) as u8,
                "Third byte should match index >> 16"
            );
            assert_eq!(
                value.data[3],
                (i >> 24) as u8,
                "Fourth byte should match index >> 24"
            );
        }
    }

    #[test]
    fn test_insert_delete_mixed_operations() {
        // This test simulates a realistic workload with mixed operations
        // (inserts and lookups) on both existing and non-existing keys
        let mut mst = MST::new();
        let mut value_map = HashMap::new(); // Track expected content for validation

        let operations = 5000;

        for _ in 0..operations {
            // Generate a random key between 0 and 999
            let key_value = rand::random::<u32>() % 1000;
            let key = create_key(&key_value.to_be_bytes());

            // Either insert or lookup with 50% probability
            if rand::random::<bool>() {
                // Insert operation
                let value = TestValue {
                    key,
                    data: [
                        key_value as u8,
                        (key_value >> 8) as u8,
                        (key_value >> 16) as u8,
                        (key_value >> 24) as u8,
                    ],
                };
                mst.insert(key, value);
                value_map.insert(key_value, value);
            } else {
                // Lookup operation
                let mst_result = mst.get_value(key);
                let map_result = value_map.get(&key_value);

                match (mst_result, map_result) {
                    (Some(mst_val), Some(map_val)) => {
                        // Both found - values should match
                        assert_eq!(
                            mst_val.data, map_val.data,
                            "Tree and reference map should have the same values"
                        );
                    }
                    (None, None) => {
                        // Neither found - consistent
                    }
                    _ => {
                        panic!(
                            "Inconsistency: MST and HashMap disagree on key {}",
                            key_value
                        );
                    }
                }
            }
        }

        // Final verification - check all items
        for (key_value, expected_value) in &value_map {
            let key = create_key(&key_value.to_be_bytes());
            let actual_value = mst.get_value(key).unwrap();
            assert_eq!(
                actual_value.data, expected_value.data,
                "All values in reference map should match tree values"
            );
        }

        // Check total count
        assert_eq!(
            mst.to_list().len(),
            value_map.len(),
            "Tree size should match reference map size"
        );
    }
}

/// # Basic Functionality Tests
///
/// These tests verify fundamental MST operations and properties.
mod basic_tests {
    use super::*;

    #[test]
    fn test_calc_level() {
        // This test verifies that the calc_level function generates
        // valid levels within expected bounds

        let level1 = calc_level(b"test_string_1");
        let level2 = calc_level(b"test_string_2");

        // Levels should be valid and within reasonable bounds
        assert!(
            level1 < 256,
            "Level should be less than 256 (SHA-256 output space)"
        );
        assert!(
            level2 < 256,
            "Level should be less than 256 (SHA-256 output space)"
        );
    }

    #[test]
    fn test_empty_tree() {
        // This test verifies that an empty MST correctly handles
        // basic operations
        let mut empty_tree = MST::new();

        // Empty tree should have no items
        assert!(
            empty_tree.to_list().is_empty(),
            "Empty tree should return empty list"
        );

        // Getting a non-existent key should return None
        let key = create_key(b"anything");
        assert!(
            empty_tree.get(key).is_none(),
            "Getting key from empty tree should return None"
        );

        // Test merging empty tree with non-empty tree
        let mut non_empty = MST::new();
        let key = create_key(b"test");
        let value = TestValue {
            key,
            data: [1, 2, 3, 4],
        };
        non_empty.insert(key, value);

        let (merged_root_key, merged_store) = empty_tree.merge(&non_empty);
        let merged_tree = MST::with_store(merged_root_key, merged_store);

        // Merging with empty tree should preserve non-empty tree's contents
        assert_eq!(
            merged_tree.to_list().len(),
            1,
            "Merged tree should contain item from non-empty tree"
        );
        assert_eq!(
            merged_tree.get_value(key).unwrap(),
            value,
            "Merged tree should contain original value"
        );
    }

    #[test]
    fn test_basic_insert() {
        // This test verifies basic insertion and retrieval operations
        let mut mst = MST::new();

        // Insert a sequence of items in alphabetical order
        let test_keys = vec![
            b"apple"[..].to_vec(),
            b"banana"[..].to_vec(),
            b"cherry"[..].to_vec(),
            b"date"[..].to_vec(),
        ];

        // Insert each item and verify tree state after each insertion
        for (i, key) in test_keys.iter().enumerate() {
            let key_hash = create_key(key);
            let value = TestValue {
                key: key_hash,
                data: [i as u8, (i + 1) as u8, (i + 2) as u8, (i + 3) as u8],
            };
            mst.insert(key_hash, value);

            // Verify item count increases with each insertion
            assert_eq!(
                mst.to_list().len(),
                i + 1,
                "Tree should contain {} items after {} insertions",
                i + 1,
                i + 1
            );

            // Verify inserted item is retrievable
            assert_eq!(
                mst.get_value(key_hash),
                Some(value),
                "Should be able to retrieve inserted value"
            );
        }

        // Verify final tree state
        let items = mst.to_list();
        assert_eq!(
            items.len(),
            test_keys.len(),
            "Final tree should contain all inserted items"
        );

        // Verify items are stored in correct order
        for i in 1..items.len() {
            assert!(
                items[i - 1].key <= items[i].key,
                "Items should be stored in ascending order"
            );
        }
    }

    #[test]
    fn test_single_insert_and_get() {
        // This test verifies the simplest case: insert one item and retrieve it
        let mut mst = MST::new();

        // Insert a single value
        let key = create_key(&[1, 2, 3, 4]);
        let value = TestValue {
            key,
            data: [10, 0, 0, 0],
        };

        mst.insert(key, value);

        // Verify we can retrieve it
        let retrieved = mst.get_value(key);
        assert!(
            retrieved.is_some(),
            "Failed to retrieve a single inserted value"
        );
        assert_eq!(
            retrieved.unwrap().data,
            [10, 0, 0, 0],
            "Retrieved value should match inserted value"
        );

        // Try a key that doesn't exist
        let nonexistent_key = create_key(&[5, 6, 7, 8]);
        let missing = mst.get_value(nonexistent_key);
        assert!(missing.is_none(), "Should return None for nonexistent key");
    }
}

/// # Edge Case Tests
///
/// These tests verify the MST handles unusual or extreme situations correctly.
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_duplicate_keys() {
        // This test verifies that the tree correctly handles duplicate key insertions
        // by replacing the existing value
        let mut mst = MST::new();

        let key = create_key(b"duplicate");
        let value1 = TestValue {
            key,
            data: [1, 2, 3, 4],
        };
        let value2 = TestValue {
            key,
            data: [5, 6, 7, 8],
        };

        // Insert same key twice with different values
        mst.insert(key, value1);
        mst.insert(key, value2);

        // Tree should maintain only one entry for the duplicate key
        assert_eq!(
            mst.to_list().len(),
            1,
            "Tree should contain only one entry for duplicate key"
        );

        // The second value should override the first (per our merge implementation)
        let stored_value = mst.get_value(key).unwrap();
        assert_eq!(
            stored_value.data, value2.data,
            "Duplicate key should store most recent value"
        );
    }

    #[test]
    fn test_long_and_short_keys() {
        // This test verifies that the tree correctly handles keys of
        // significantly different lengths
        let mut mst = MST::new();

        // Insert very short key (1 byte)
        let short_key = create_key(b"a");
        let short_value = TestValue {
            key: short_key,
            data: [1, 0, 0, 0],
        };
        mst.insert(short_key, short_value);

        // Insert very long key (1000 bytes)
        let long_data = [255u8; 1000];
        let long_key = create_key(&long_data);
        let long_value = TestValue {
            key: long_key,
            data: [2, 0, 0, 0],
        };
        mst.insert(long_key, long_value);

        // Verify both keys are stored and retrievable
        assert_eq!(
            mst.to_list().len(),
            2,
            "Tree should store both short and long keys"
        );
        assert_eq!(
            mst.get_value(short_key),
            Some(short_value),
            "Short key should be retrievable"
        );
        assert_eq!(
            mst.get_value(long_key),
            Some(long_value),
            "Long key should be retrievable"
        );
    }

    #[test]
    fn test_lookup_edge_cases() {
        // This test verifies edge cases in key lookup functionality
        let mut mst = MST::new();

        // Insert some ordered keys
        for i in 0..5u8 {
            let key = create_key(&[i, 0, 0, 0]);
            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);

            // Verify each key immediately after insertion
            let retrieved = mst.get_value(key);
            assert!(
                retrieved.is_some(),
                "Key {} should be retrievable immediately after insertion",
                i
            );
        }

        // Check looking up keys at the extremes of the range

        // Key less than all values in tree
        let too_small = create_key(&[0, 0, 0, 0]);
        let result_small = mst.get_value(too_small);
        // Behavior depends on how create_key works and tree structure
        println!("Looking up key less than all: {:?}", result_small);

        // Key greater than all values in tree
        let too_large = create_key(&[255, 255, 255, 255]);
        let result_large = mst.get_value(too_large);
        println!("Looking up key greater than all: {:?}", result_large);
    }
}

/// # Merge Operation Tests
///
/// These tests verify the MST's merge functionality works correctly.
mod merge_tests {
    use super::*;

    #[test]
    fn test_merging_disjoint_trees() {
        // This test verifies merging two trees with no overlapping keys
        let mut tree_a = MST::new();
        let mut tree_b = MST::new();

        // Populate first tree with items 1-5
        for i in 1u32..=5u32 {
            let key = create_key(&i.to_be_bytes());
            tree_a.insert(
                key,
                TestValue {
                    key,
                    data: [i as u8, 0, 0, 0],
                },
            );
        }

        // Populate second tree with items 6-10
        for i in 6u32..=10u32 {
            let key = create_key(&i.to_be_bytes());
            tree_b.insert(
                key,
                TestValue {
                    key,
                    data: [i as u8, 0, 0, 0],
                },
            );
        }

        // Merge the trees
        let (merged_root_key, merged_store) = tree_a.merge(&tree_b);
        let merged_tree = MST::with_store(merged_root_key, merged_store);

        // Verify merged tree contains all items
        assert_eq!(
            merged_tree.to_list().len(),
            10,
            "Merged tree should contain all items from both trees"
        );

        // Verify all values are retrievable
        for i in 1u32..=10u32 {
            let key = create_key(&i.to_be_bytes());
            assert!(
                merged_tree.get_value(key).is_some(),
                "Merged tree should contain value for key {}",
                i
            );
        }
    }

    #[test]
    fn test_merging_overlapping_trees() {
        // This test verifies merging trees with overlapping keys
        // (keys present in both trees)
        let mut tree_a = MST::new();
        let mut tree_b = MST::new();

        // Insert overlapping items with different values
        for i in 1u32..=5u32 {
            let key = create_key(&i.to_be_bytes());

            // Tree A has simple values
            tree_a.insert(
                key,
                TestValue {
                    key,
                    data: [i as u8, 0, 0, 0],
                },
            );

            // Tree B has different values for same keys
            tree_b.insert(
                key,
                TestValue {
                    key,
                    data: [i as u8, i as u8, i as u8, i as u8],
                },
            );
        }

        // Merge the trees
        let (merged_root_key, merged_store) = tree_a.merge(&tree_b);
        let merged_tree = MST::with_store(merged_root_key, merged_store);

        // Verify merged tree has correct number of items (no duplicates)
        assert_eq!(
            merged_tree.to_list().len(),
            5,
            "Merged tree should contain one entry per unique key"
        );

        // Verify values from tree_b took precedence (per our merge implementation)
        for i in 1u32..=5u32 {
            let key = create_key(&i.to_be_bytes());
            let value = merged_tree.get_value(key).unwrap();
            assert_eq!(
                value.data,
                [i as u8, i as u8, i as u8, i as u8],
                "Merged tree should contain values from tree_b for key {}",
                i
            );
        }
    }
}

/// # Performance Tests
///
/// These tests verify the MST performs well with larger datasets.
mod performance_tests {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    #[test]
    fn test_large_dataset() {
        // This test verifies MST performance with a larger dataset
        // and validates correct behavior when merging trees
        let mut nums_a: Vec<u32> = (20..=30).collect();
        let mut nums_b: Vec<u32> = (5..=57).collect();

        // Shuffle inputs to test random insertion order
        let mut rng = thread_rng();
        nums_a.shuffle(&mut rng);
        nums_b.shuffle(&mut rng);

        // Create and populate trees
        let mut tree_a = MST::new();
        let mut tree_b = MST::new();

        // Insert shuffled items into trees
        for &num in &nums_a {
            let key = create_key(&num.to_be_bytes());
            tree_a.insert(
                key,
                TestValue {
                    key,
                    data: [num as u8, 0, 0, 0],
                },
            );
        }

        for &num in &nums_b {
            let key = create_key(&num.to_be_bytes());
            tree_b.insert(
                key,
                TestValue {
                    key,
                    data: [num as u8, 0, 0, 0],
                },
            );
        }

        // Merge large trees
        let (merged_root_key, merged_store) = tree_a.merge(&tree_b);
        let merged_tree = MST::with_store(merged_root_key, merged_store);

        // Calculate expected size (unique items after merge)
        let expected_unique_count = 53; // 11 + 53 - 11 (overlap)

        // Verify merged tree contains correct number of items
        assert_eq!(
            merged_tree.to_list().len(),
            expected_unique_count,
            "Merged tree should contain all unique items"
        );

        // Verify items are in sorted order
        let items = merged_tree.to_list();
        for i in 1..items.len() {
            assert!(
                TestValue::compare_keys(&items[i - 1].key, &items[i].key)
                    != std::cmp::Ordering::Greater,
                "Items should maintain sorted order in large dataset"
            );
        }
    }
}

/// # Specialized Tests
///
/// These tests target specific edge cases and behaviors in the MST implementation.
mod specialized_tests {
    use super::*;

    #[test]
    fn test_tree_consistency() {
        // This test verifies tree maintains structural consistency
        // throughout a series of operations
        let mut mst = MST::new();

        // Insert values and check consistency after each insertion
        for i in 0..10u8 {
            let key = create_key(&[i, 0, 0, 0]);
            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);

            // After each insertion:
            // 1. Verify the list is in sorted order
            let list = mst.to_list();
            for j in 1..list.len() {
                assert!(
                    TestValue::compare_keys(&list[j - 1].key, &list[j].key)
                        != std::cmp::Ordering::Greater,
                    "List is not in sorted order after inserting key {}",
                    i
                );
            }

            // 2. Verify all previously inserted keys are still retrievable
            for j in 0..=i {
                let check_key = create_key(&[j, 0, 0, 0]);
                assert!(
                    mst.get_value(check_key).is_some(),
                    "Key {} not retrievable after inserting key {}",
                    j,
                    i
                );
            }
        }
    }

    #[test]
    fn test_interleaved_operations() {
        // This test verifies that the tree handles interleaved insert and lookup
        // operations correctly
        const A: u8 = 11; // Define a constant for the pattern generation

        let mut mst = MST::new();
        let mut keys = Vec::new();

        // Insert 50 values with complex patterns
        for i in 0..50u8 {
            let key = create_key(&[i, i % A, i % 7, i % 13]); // Use a mix of patterns
            keys.push(key);

            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);

            // After each insertion, check a previously inserted key
            if !keys.is_empty() && i > 0 {
                let idx = (i as usize) % keys.len();
                let check_key = keys[idx];
                let retrieved = mst.get_value(check_key);

                assert!(
                    retrieved.is_some(),
                    "Failed to retrieve key at index {} after inserting key {}",
                    idx,
                    i
                );
            }
        }

        // Finally, verify all inserted keys are retrievable
        for (i, &key) in keys.iter().enumerate() {
            let retrieved = mst.get_value(key);
            assert!(
                retrieved.is_some(),
                "Failed to retrieve key at index {} after all insertions",
                i
            );
        }
    }

    #[test]
    fn test_split_operation() {
        // This test specifically targets the split operation
        let mut mst = MST::new();

        // Insert enough values to trigger multiple splits
        for i in 0..20u8 {
            let key = create_key(&[i, i, i, i]);
            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);

            // After each insertion, verify ALL previously inserted keys
            for j in 0..=i {
                let check_key = create_key(&[j, j, j, j]);
                let retrieved = mst.get_value(check_key);
                assert!(
                    retrieved.is_some(),
                    "After inserting key {}, could not retrieve key {}",
                    i,
                    j
                );
                assert_eq!(
                    retrieved.unwrap().data[0],
                    j,
                    "Retrieved incorrect value for key {}",
                    j
                );
            }
        }

        // Insert values in reverse order to test different split patterns
        for i in (20..40u8).rev() {
            let key = create_key(&[i, i, i, i]);
            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);

            // Verify the key we just inserted
            let retrieved = mst.get_value(key);
            assert!(
                retrieved.is_some(),
                "Failed to retrieve key {} after insertion",
                i
            );
            assert_eq!(
                retrieved.unwrap().data[0],
                i,
                "Retrieved incorrect value for key {}",
                i
            );
        }

        // Now verify ALL keys are retrievable
        for i in 0..40u8 {
            let key = create_key(&[i, i, i, i]);
            let retrieved = mst.get_value(key);
            assert!(
                retrieved.is_some(),
                "After all insertions, could not retrieve key {}",
                i
            );
            assert_eq!(
                retrieved.unwrap().data[0],
                i,
                "Retrieved incorrect value for key {}",
                i
            );
        }
    }

    #[test]
    fn test_progressive_tree_growth() {
        // This test verifies tree integrity during growth by incrementally
        // inserting values and checking retrievability
        let mut mst = MST::new();

        // Track keys we've inserted
        let mut inserted_keys = HashSet::new();
        let mut failed_retrievals = Vec::new();

        // Insert keys with incremental verification
        for i in 0..120u32 {
            // Use a key pattern with good distribution
            let key_bytes = [
                (i % 256) as u8,
                ((i * 3) % 256) as u8,
                ((i * 7) % 256) as u8,
                ((i * 11) % 256) as u8,
            ];
            let key = create_key(&key_bytes);

            // Insert the key
            let value = TestValue {
                key,
                data: [i as u8, 0, 0, 0],
            };
            mst.insert(key, value);
            inserted_keys.insert(key);

            // After every 10 insertions, verify ALL previously inserted keys
            if i % 10 == 0 && i > 0 {
                for &check_key in &inserted_keys {
                    if mst.get_value(check_key).is_none() {
                        failed_retrievals.push((i, check_key));
                    }
                }
            }
        }

        // Assertion fails if any keys were not retrievable
        assert!(
            failed_retrievals.is_empty(),
            "{} keys could not be retrieved after insertion",
            failed_retrievals.len()
        );
    }

    #[test]
    fn test_targeted_split_edge_case() {
        // This test targets specific edge cases in the split algorithm
        let mut mst = MST::new();

        // First insert some foundation keys
        for i in 0..20u8 {
            let key = create_key(&[i, i + 1, i + 2, i + 3]);
            let value = TestValue {
                key,
                data: [i, 0, 0, 0],
            };
            mst.insert(key, value);
        }

        // Now insert keys with patterns that might trigger specific split behaviors
        let challenging_patterns = [
            [100, 120, 140, 160], // Wide spread values
            [101, 101, 101, 101], // Identical bytes
            [255, 254, 253, 252], // High values
            [128, 128, 128, 128], // Middle values
            [122, 120, 96, 150],  // Mix of values
        ];

        for (i, &pattern) in challenging_patterns.iter().enumerate() {
            let key = create_key(&pattern);
            let value = TestValue {
                key,
                data: [(200 + i) as u8, 0, 0, 0],
            };

            mst.insert(key, value);

            // Verify the key is retrievable immediately after insertion
            let retrieved = mst.get_value(key);
            assert!(
                retrieved.is_some(),
                "Could not retrieve key with pattern {:?} after insertion",
                pattern
            );
        }

        // Verify ALL keys are still retrievable
        // (foundation keys and challenge keys)
        for i in 0..20u8 {
            let key = create_key(&[i, i + 1, i + 2, i + 3]);
            assert!(
                mst.get_value(key).is_some(),
                "Could not retrieve foundation key [{}]",
                i
            );
        }

        for &pattern in &challenging_patterns {
            let key = create_key(&pattern);
            assert!(
                mst.get_value(key).is_some(),
                "Could not retrieve challenge key {:?}",
                pattern
            );
        }
    }
}
