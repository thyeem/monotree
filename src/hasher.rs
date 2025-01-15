//! A module for implementing hash functions supporting `monotree`.
use crate::utils::*;
use crate::*;
use digest::Digest;

/// A trait defining hashers used for `monotree`
pub trait Hasher {
    fn new() -> Self;
    fn digest(&self, bytes: &[u8]) -> Hash;
}

#[derive(Clone, Debug)]
/// A hasher using `Blake2s` hash function
pub struct Blake2s;
impl Hasher for Blake2s {
    fn new() -> Self {
        Blake2s
    }

    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake2_rfc::blake2s::Blake2s::new(HASH_LEN);
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
/// A hasher using `Blake2b` hash function
pub struct Blake2b;
impl Hasher for Blake2b {
    fn new() -> Self {
        Blake2b
    }

    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake2_rfc::blake2b::Blake2b::new(HASH_LEN);
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
/// A hasher using `Blake3` hash function
pub struct Blake3;
impl Hasher for Blake3 {
    fn new() -> Self {
        Blake3
    }

    /// Currently supports 256-bit or 32-byte only.
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_bytes())
    }
}

#[derive(Clone, Debug)]
/// A hasher using `SHA2` hash function
pub struct Sha2;
impl Hasher for Sha2 {
    fn new() -> Self {
        Sha2
    }

    /// Currently supports 256-bit or 32-byte only.
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_slice())
    }
}

#[derive(Clone, Debug)]
/// A hasher using `SHA3` or `Keccak` hash function
pub struct Sha3;
impl Hasher for Sha3 {
    fn new() -> Self {
        Sha3
    }

    /// Currently supports 256-bit or 32-byte only.
    fn digest(&self, bytes: &[u8]) -> Hash {
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        slice_to_hash(hash.as_slice())
    }
}
