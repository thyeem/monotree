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
//! # Quick start
//! ```
//! use monotree::{Monotree, Result};
//! use monotree::utils::random_hash;
//!
//! fn example() -> Result<()> {
//!     // Init a monotree instance
//!     // by default, with 'HashMap' and 'Blake3' hash function
//!     let mut tree = Monotree::default();
//!
//!     // It is natural the tree root initially has 'None'
//!     let root = None;
//!
//!     // Prepare a random pair of key and leaf.
//!     // random_hashes() gives a fixed length of random array,
//!     // where Hash -> [u8; HASH_LEN], HASH_LEN = 32
//!     let key = random_hash();
//!     let leaf = random_hash();
//!
//!     // Insert the entry (key, leaf) into tree, yielding a new root of tree
//!     let root = tree.insert(root.as_ref(), &key, &leaf)?;
//!     assert_ne!(root, None);
//!
//!     // Get the leaf inserted just before. Note that the last root was used.
//!     let found = tree.get(root.as_ref(), &key)?;
//!     assert_eq!(found, Some(leaf));
//!
//!     // Remove the entry
//!     let root = tree.remove(root.as_ref(), &key)?;
//!
//!     // surely, the tree has nothing and the root back to 'None'
//!     assert_eq!(tree.get(root.as_ref(), &key)?, None);
//!     assert_eq!(root, None);
//!     Ok(())
//! }
//! ```
//!
//! # Generate/verify Merkle proof  
//! `monotree` has compressed representation, but it fully retains
//! the properties of the Sparse Merkle Tree (SMT).   
//! Thus, `non-inclusion proof` is quite straightforward. Just go walk down
//! the tree with a key (or a path) given. If we cannot successfully get a leaf,
//! we can assure that the leaf is not a part of the tree.   
//!
//! The process of verifying inclusion of data (inclusion proof) is below:
//!
//! # Example
//! ```
//! use monotree::utils::random_hashes;
//! use monotree::hasher::Blake3;
//! use monotree::{verify_proof, Hasher, Monotree, Result};
//!
//! fn example() -> Result<()> {
//!     // random pre-insertion for Merkle proof test
//!     let mut tree = Monotree::default();
//!     let root = None;
//!     let keys = random_hashes(500);
//!     let leaves = random_hashes(500);
//!     let root = tree.inserts(root.as_ref(), &keys, &leaves)?;
//!
//!     // pick a random key from keys among inserted just before
//!     let key = keys[99];
//!
//!     // generate the Merkle proof for the root and the key
//!     let proof = tree.get_merkle_proof(root.as_ref(), &key)?;
//!
//!     // To verify the proof correctly, you need to provide a hasher matched
//!     // the default tree was initialized with `Blake3`
//!     let hasher = Blake3::new();
//!
//!     // get a leaf matched with the key: where the Merkle proof starts off
//!     let leaf = leaves[99];
//!
//!     // verify the Merkle proof using all those above
//!     let verified = verify_proof(&hasher, root.as_ref(), &leaf, proof.as_ref());
//!     assert_eq!(verified, true);
//!     Ok(())
//! }
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
