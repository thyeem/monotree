use crate::consts::*;
use crate::utils::*;
use crate::{Cell, Database, Errors, Hash, Node, Proof, Result, Unit};
use blake2_rfc::blake2b::blake2b;
use std::ops::Range;

/// Example: How to use MonoTree
/// ```rust, ignore
///     //--- prepare random key-value pair like:
///     type Hash = [u8; HASH_LEN]
///     let pairs: Vec<(Hash, Hash)> = (0..1000)
///         .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
///         .collect();
///
///     //--- init tree using either MemDB or RocksDB
///     let mut tree = tree::MonoTree::<MemoryDB>::new("MemDB");
///     // let mut tree = tree::MonoTree::<RocksdbDB>::new("RocksDB");
///     let mut root = tree.new_tree();
///
///     //--- functional test: insert/get
///     pairs.iter().for_each(|(key, value)| {
///         root = tree.insert(root.as_ref(), key, value);
///         assert_eq!(tree.get(root.as_ref(), key).unwrap(), *value);
///    });
/// ```

pub struct MonoTree<T>
where
    T: Database,
{
    pub db: T,
}

impl<T> MonoTree<T>
where
    T: Database,
{
    pub fn new(dbpath: &str) -> Self {
        let db = Database::new(dbpath);
        MonoTree { db }
    }

    pub fn new_tree(&mut self) -> Option<Hash> {
        None
    }

    pub fn insert(&mut self, root: Option<&Hash>, key: &Hash, leaf: &Hash) -> Option<Hash> {
        match root {
            None => {
                let (hash, path, range) = (leaf.to_owned(), key.to_vec(), 0..key.len() * 8);
                self.put_node(Node::new(Some(Unit { hash, path, range }), None))
                    .ok()
            }
            Some(root) => self.put(root, key, 0..key.len() * 8, leaf).ok(),
        }
    }

    fn get_node_cells(&self, hash: &Hash, right: bool) -> Result<(Cell, Cell)> {
        let bytes = self.db.get(hash)?;
        match Node::from_bytes(&bytes)? {
            Node::Soft(cell) => Ok((cell, None)),
            Node::Hard(lc, rc) => match right {
                true => Ok((rc, lc)),
                false => Ok((lc, rc)),
            },
        }
    }

    fn put_node(&mut self, node: Node) -> Result<Hash> {
        let bytes = node.to_bytes()?;
        let hash = blake2b(HASH_LEN, &[], &bytes);
        self.db.put(hash.as_bytes(), bytes)?;
        slice_to_hash(hash.as_bytes())
    }

    fn put(&mut self, root: &Hash, key: &[u8], range: Range<usize>, leaf: &Hash) -> Result<Hash> {
        let (lc, rc) = self.get_node_cells(root, bit(key, range.start))?;
        let unit = lc.as_ref().unwrap();
        let n = len_lcp(&unit.path, &unit.range, key, &range);
        match n {
            n if n == 0 => {
                let (hash, path) = (leaf.to_owned(), key.to_vec());
                self.put_node(Node::new(lc, Some(Unit { hash, path, range })))
            }
            n if n == range.end - range.start => {
                let (hash, unit) = (leaf.to_owned(), lc.unwrap());
                self.put_node(Node::new(Some(Unit { hash, ..unit }), rc))
            }
            n if n == unit.range.end - unit.range.start => {
                let (q, range) = offsets(&range, n, false);
                let hash = self.put(&unit.hash, &key[q..], range, leaf)?;
                let unit = lc.unwrap();
                self.put_node(Node::new(Some(Unit { hash, ..unit }), rc))
            }
            _ => {
                let (q, range) = offsets(&range, n, false);
                let (hash, path) = (leaf.to_owned(), key[q..].to_vec());
                let ru = Unit { hash, path, range };

                let (q, range) = offsets(&unit.range, n, false);
                let (hash, path) = (unit.hash, unit.path[q..].to_vec());
                let lu = Unit { hash, path, range };

                let (q, range) = offsets(&unit.range, n, true);
                let hash = self.put_node(Node::new(Some(lu), Some(ru)))?;
                let path = unit.path[..q].to_vec();
                self.put_node(Node::new(Some(Unit { hash, path, range }), rc))
            }
        }
    }

    pub fn get(&self, root: Option<&Hash>, key: &[u8]) -> Option<Hash> {
        match root {
            None => None,
            Some(root) => self.find_key(root, key, 0..key.len() * 8).ok(),
        }
    }

    fn find_key(&self, root: &Hash, key: &[u8], range: Range<usize>) -> Result<Hash> {
        let (cell, _) = self.get_node_cells(root, bit(key, range.start))?;
        let unit = cell.as_ref().unwrap();
        let n = len_lcp(&unit.path, &unit.range, key, &range);
        match n {
            n if n == range.end - range.start => Ok(unit.hash),
            n if n == unit.range.end - unit.range.start => {
                let (q, range) = offsets(&range, n, false);
                self.find_key(&unit.hash, &key[q..], range)
            }
            _ => Err(Errors::new("Not found")),
        }
    }

    /// Merkle proof secion: verifying inclusion of data ----------------------
    /// In order to prove proofs, use verify_proof() at the end of file below.
    /// Example:
    /// ```rust, ignore
    ///     // suppose (key: Hash, value: Hash) alreay prepared.
    ///     // let mut root = ...
    ///     root = tree.insert(&root, &key, &value);
    ///     ...
    ///     let leaf = tree.get(root.as_ref(), &key).unwrap();
    ///     let proof = tree.get_merkle_proof(root.as_ref(), &key).unwrap();
    ///     assert_eq!(tree::verify_proof(root.as_ref(), &leaf, &proof), true);
    /// ```

    pub fn get_merkle_proof(&self, root: Option<&Hash>, key: &[u8]) -> Option<Proof> {
        let mut proof: Proof = Vec::new();
        match root {
            None => None,
            Some(root) => self.gen_proof(root, key, 0..key.len() * 8, &mut proof).ok(),
        }
    }

    fn gen_proof(
        &self,
        root: &Hash,
        key: &[u8],
        range: Range<usize>,
        proof: &mut Proof,
    ) -> Result<Proof> {
        let bytes = self.db.get(root)?;
        let right = bit(key, range.start);
        let (cell, _) = self.get_node_cells(root, right)?;
        let unit = cell.as_ref().unwrap();
        let n = len_lcp(&unit.path, &unit.range, key, &range);
        match n {
            n if n == range.end - range.start => {
                proof.push(self.encode_proof(&bytes, right)?);
                Ok(proof.to_owned())
            }
            n if n == unit.range.end - unit.range.start => {
                proof.push(self.encode_proof(&bytes, right)?);
                let (q, range) = offsets(&range, n, false);
                self.gen_proof(&unit.hash, &key[q..], range, proof)
            }
            _ => Err(Errors::new("Abort: key not found")),
        }
    }

    fn encode_proof(&self, bytes: &[u8], right: bool) -> Result<(bool, Vec<u8>)> {
        match Node::from_bytes(bytes)? {
            Node::Soft(_) => Ok((false, bytes[HASH_LEN..].to_vec())),
            Node::Hard(_, _) => match right {
                true => Ok((
                    true,
                    [&bytes[..bytes.len() - HASH_LEN - 1], &[0x01]].concat(),
                )),
                false => Ok((false, bytes[HASH_LEN..].to_vec())),
            },
        }
    }
}

/// No need to be a member of MonoTree.
/// This verification must be called independantly upon request.
pub fn verify_proof(root: Option<&Hash>, leaf: &Hash, proof: &Proof) -> bool {
    let mut hash = leaf.to_vec();
    proof.iter().rev().for_each(|(right, cut)| match *right {
        false => {
            let o = [&hash[..], &cut[..]].concat();
            hash = blake2b(HASH_LEN, &[], &o).as_bytes().to_vec();
        }
        true => {
            let l = cut.len();
            let o = [&cut[..l - 1], &hash[..], &cut[l - 1..]].concat();
            hash = blake2b(HASH_LEN, &[], &o).as_bytes().to_vec();
        }
    });
    root.unwrap().to_vec() == hash
}
