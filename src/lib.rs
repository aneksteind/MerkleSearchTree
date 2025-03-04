pub mod mst;
pub mod store;
pub mod test_utils;
pub mod utils;

// Re-export main items for convenience
pub use mst::MST;
pub use store::Store;
pub use store::{Page, PageData};
pub use utils::{KeyComparable, MSTKey, Merge, calc_level, compare, hash};

// Re-export hash_page at the crate root
pub use mst::hash_page;

/// A trait for types that can reference other objects via keys
///
/// Used to define types that hold references to other objects,
/// typically via cryptographic hashes or unique identifiers.
///
/// # Example
///
/// ```
/// use mst::{Reference, MSTKey};
///
/// struct MyPage {
///     next: Option<MSTKey>,
///     data: Vec<u8>
/// }
///
/// impl Reference for MyPage {
///     type Key = MSTKey;
///     
///     fn refs(&self) -> Vec<Self::Key> {
///         self.next.map_or(vec![], |key| vec![key])
///     }
/// }
/// ```
pub trait Reference {
    /// The type of key used for references
    type Key;

    /// Returns a vector of keys that this object references
    fn refs(&self) -> Vec<Self::Key>;
}
