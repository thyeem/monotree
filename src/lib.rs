//! # Monotree
//! Rust implementation of an optimized Sparse Merkle Tree.
//! This is a kind of binary-radix tree based on bitwise branching, _currently_, no nibble of bit.
//! For now, branching unit is just ___a single bit___, _neither a 4-bit nor a byte nibble_.
//!
//! ## Features
//! - Very ___simple___ and __lightweight__, but ___fast___ and __robust__.
//! - __Fully featured__ Sparse Merkle Tree (SMT) as a storage
//! - <ins>This includes: __non-inclusion proof__ , as well as __inclusion proof__, and its verification.</ins>
//! - Again, _NOT verbose_ at all.
//!
//! This library mostly relies on the _Rust standard library only_ except for `database APIs` and `hashers`.
//! Currently, `monotree` supports these databases and hash functions following,
//! but is designed to be super easy to customize and add:
//!
//! _Databases include_:
//! - [`HashMap`](https://lib.rs/crates/hashbrown)
//! - [`RocksDB`](https://lib.rs/crates/rocksdb)
//! - [`Sled`](https://lib.rs/crates/sled)
//!
//! _Hashers include_:
//! - [`Blake3`](https://lib.rs/crates/blake3)
//! - [`Blake2s`](https://lib.rs/crates/blake2-rfc) and [`Blake2b`](https://lib.rs/crates/blake2-rfc)
//! - [`SHA-2`](https://lib.rs/crates/sha2)
//! - [`SHA-3 (Keccak)`](https://lib.rs/crates/sha3)
//!
//! ## Quick start
//! > Refer to _`examples/basic.rs`_ for a complete working example.
//! >
//! > Regarding __non-inclusion proof__ and __inclusion proof__, See _Merkle proof_ section in [More Examples](#more-examples) below.
//!
//! ### Initialize
//! ```rust,ignore
//! // If no database or hash function is specified,
//! // the tree defaults to using a HashMap and the Blake3 hash function.
//! let mut tree = Monotree::default();
//!
//! // It is natural the tree root initially has 'None'.
//! let root = None;
//! ```
//! ### Insert
//! ```rust,ignore
//! // Generate a random key and leaf pair.
//! // random_hash() creates a fixed-length random array of 32 bytes.
//! let key = random_hash();
//! let leaf = random_hash();
//!
//! // Insert the entry (key, leaf) into tree, yielding a new root of tree
//! let root = tree.insert(root.as_ref(), &key, &leaf)?;
//! assert_ne!(root, None);
//! ```
//! ### Retrieve
//! ```rust,ignore
//! // Retrieve the leaf inserted just before. Note that the last root was used.
//! let found = tree.get(root.as_ref(), &key)?;
//! assert_eq!(found, Some(leaf));
//! ```
//! ### Remove
//! ```rust,ignore
//! // Remove the entry
//! let root = tree.remove(root.as_ref(), &key)?;
//!
//! // The tree is empty now and the root back to 'None'
//! assert_eq!(tree.get(root.as_ref(), &key)?, None);
//! assert_eq!(root, None);
//! ```
//! ### Batch: Atomic Transaction
//! Instead of executing each operation one by one, write them in a batch and then commit them all at once.
//! One can do the same thing above using batch operation for ___performance gain___ and ___atomicity___. In short,
//!
//! `prepare` → many of {`insert`, `get`, `remove`} → `commit`
//!
//! #### Prepare Transaction
//! ```rust,ignore
//! // initialize an empty batch: prepare transaction
//! tree.prepare();
//! ```
//! #### Freely `insert`, `get`, and `remove`
//! ```rust,ignore
//! // Insert the entry (key, leaf) within the batch
//! let root = tree.insert(root.as_ref(), &key, &leaf)?;
//! assert_ne!(root, None);
//!
//! // Retrieve the inserted leaf using the batch root
//! let found = tree.get(root.as_ref(), &key)?;
//! assert_eq!(found, Some(leaf));
//!
//! // Remove the entry within the same batch
//! let root = tree.remove(root.as_ref(), &key)?;
//!
//! // Ensure the entry was removed within the batch
//! assert_eq!(tree.get(root.as_ref(), &key)?, None);
//! ```
//! ##### Commit Transaction
//! ```rust,ignore
//! // Commit the batch operations: commit transaction
//! tree.commit();
//!
//! // Verify that the final root is `None` after commit
//! assert_eq!(tree.get(root.as_ref(), &key)?, None);
//! ```
//! ### Initialize with a specific database and hash function
//! ```rust,ignore
//! // Init a monotree instance with a database and hash function
//! //
//! // Monotree::<DATABASE, HASHER>::new(DB_PATH)
//! //      where DATABASE = {MemoryDB, RocksDB, Sled}
//! //            HASHER = {Blake3, Blake2s, Blake2b, Sha2, Sha3}
//! let mut tree = Monotree::<RocksDB, Blake2b>::new("/tmp/monotree");
//!
//! // It is natural the tree root initially has 'None'
//! let root = None;
//! ```
//!
//! ### Intrinsic batch processing: `inserts` and `removes`.
//! As shown in [Quick Start](#quick-start) above, __operations executed between__ `tree.prepare()` __and__ `tree.commit()` can be viewed as _a single batch operation_. By default, they are automatically cached and are written together in a batch unit.
//!
//! However, if you need to repeat the same operation, such as `insert` or `remove`, you can easily optimize performance by using `inserts` and `removes`.
//!
//! `inserts()` is ___significantly faster___ than `insert()` for the following reason:
//! - Batch writes
//! - Sorting keys prior to insertion
//! - In-memory caching
//! ```rust,ignore
//! // Prepare 100 random pairs of key and leaf.
//! // random_hash(SIZE) creates a vector of fixed-length random array of 32 bytes.
//! let keys = random_hashes(100);
//! let leaves = random_hashes(100);
//!
//! // Insert a vector of entries of (key, leaf) into tree.
//! let root = tree.inserts(root.as_ref(), &keys, &leaves)?;
//! assert_ne!(root, None);
//! ```
//!
//! Similarly, `gets()` and `removes()` also are designed for batch usage.
//! ```rust,ignore
//! let result = tree.gets(root.as_ref(), &keys)?;
//! assert_eq!(result.len(), keys.len());
//!
//! let root = tree.removes(root.as_ref(), &keys)?;
//! // surely, the tree has nothing nad the root back to 'None'
//! assert_eq!(root, None);
//! ```
//!
//! ### Non-inclusion proof and inclusion proof, _Merkle Proof_
//! `monotree` has compressed representations, but it fully retains the core property of the ___Sparse Merkle Trie___.
//! _non-inclusion proof_ is quite straightforward: Just go walk down the tree with a key (or a path) given.
//! If we ___cannot successfully get a leaf___, we can assure that ___the leaf is not a part of the tree___.
//!
//! The process of __inclusion proof__ is outlined below:
//!
//! #### Generate a Merkle Proof
//! Prepare a random tree for testing.
//! ```rust,ignore
//! // random insertions for testing Merkle proof generation
//! let root = tree.inserts(root.as_ref(), &keys, &leaves)?;
//!
//! // pick a random key from the keys among inserted just before
//! let key = keys[99];
//! ```
//!
//! Generate a Merkle proof for a given root and key.
//! ```rust,ignore
//! let proof = tree.get_merkle_proof(root.as_ref(), &key)?;
//! ```
//!
//! #### Verify the Merkle Proof
//! Prepare a __target leaf__ matched with the __target root__ above.
//! ```rust,ignore
//! // where the Merkle proof verification starts off
//! let leaf = leaves[99];
//! ```
//! To verify the proof correctly, you ___need to provide a hasher matched___.
//! ```rust,ignore
//! // Previously the tree was initialized with `Blake2b`
//! let hasher = Blake2b::new();
//! ```
//!
//! Just call `verify_proof(&HASHER, &ROOT, &LEAF, &PROOF)`. That's all!
//! ```rust,ignore
//! let verified = verify_proof(&hasher, root.as_ref(), &leaf, proof.as_ref());
//! assert_eq!(verified, true);
//! ```
//!
//! ### Tracking the Latest Root
//! ___Sparse Merkle Trie___ is a space where ___multiple states (roots) coexist simultaneously___.
//! There is no reason why a particular state must be stored more preciously than another. The __lastest root__ is _nothing more than the most recently updated state_.
//!
//! `monotree` has a minimal design. It __does not automatically update or track the latest root__.
//! However, `monotree` provides tools to update and fetch the latest root.
//!
//! Use `set_headroot(&LATEST_ROOT)` to set the latest root to the database.
//! ```rust,ignore
//! tree.set_headroot(root.as_ref());
//! ```
//! Use `get_headroot()` to get the latest root from the database.
//! ```rust,ignore
//! let headroot = tree.get_headroot()?;
//! assert_eq!(headroot, root);
//! ```

/// Size of fixed length byte-array from a `Hasher`. Equivalent to `key` length of `monotree`.
pub const HASH_LEN: usize = 32;

/// A type representing length of `Bits`.
pub type BitsLen = u16;

/// A `Result` type redefined for error handling. The same as `std::result::Result<T, Errors>`.
pub type Result<T> = std::result::Result<T, Errors>;

/// A type indicating fixed length byte-array. This has the length of `HASH_LEN`.
pub type Hash = [u8; HASH_LEN];

/// A type representing _Merkle proof_.
pub type Proof = Vec<(bool, Vec<u8>)>;

/// The key to be used to restore the latest `root`
pub const ROOT_KEY: &Hash = b"_______monotree::headroot_______";

/// A type indicating database selected by default.
pub type DefaultDatabase = database::MemoryDB;

/// A type indicating hasher selected by default.
pub type DefaultHasher = hasher::Blake3;

pub use self::bits::Bits;
pub use self::database::Database;
pub use self::hasher::Hasher;
pub use self::node::{Cell, Node, Unit};
pub use self::tree::{verify_proof, Monotree};

#[derive(Debug)]
/// An `Error` type defiend for handling general errors.
pub struct Errors {
    details: String,
}

impl Errors {
    pub fn new(msg: &str) -> Errors {
        Errors {
            details: msg.to_string(),
        }
    }
}

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for Errors {
    fn description(&self) -> &str {
        &self.details
    }
}

#[macro_use]
pub mod utils;
pub mod bits;
pub mod database;
pub mod hasher;
pub mod node;
pub mod tree;
