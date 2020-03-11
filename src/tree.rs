use crate::bits::Bits;
use crate::consts::*;
use crate::node::{Node, Unit};
use crate::utils::*;
use crate::{Database, Errors, Hash, Proof, Result};
use blake2_rfc::blake2b::blake2b;

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
pub struct MonoTree<D: Database> {
    db: D,
}

impl<D> MonoTree<D>
where
    D: Database,
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
                let (hash, bits) = (leaf, Bits::new(key));
                self.put_node(Node::new(Some(Unit { hash, bits }), None))
                    .ok()
            }
            Some(root) => self.put(root, Bits::new(key), leaf).ok(),
        }
    }

    fn put_node(&mut self, node: Node) -> Result<Hash> {
        let bytes = node.to_bytes()?;
        let hash = blake2b(HASH_LEN, &[], &bytes);
        self.db.put(hash.as_bytes(), bytes)?;
        slice_to_hash(hash.as_bytes())
    }

    fn put(&mut self, root: &[u8], bits: Bits, leaf: &[u8]) -> Result<Hash> {
        let bytes = self.db.get(root)?;
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().unwrap();
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == 0 => self.put_node(Node::new(lc, Some(Unit { hash: leaf, bits }))),
            n if n == bits.len() => self.put_node(Node::new(Some(Unit { hash: leaf, bits }), rc)),
            n if n == unit.bits.len() => {
                let hash = &self.put(unit.hash, bits.shift(n, false), leaf)?;
                let unit = unit.to_owned();
                self.put_node(Node::new(Some(Unit { hash, ..unit }), rc))
            }
            _ => {
                let bits = bits.shift(n, false);
                let ru = Unit { hash: leaf, bits };

                let (cloned, unit) = (unit.bits.clone(), unit.to_owned());
                let (hash, bits) = (unit.hash, unit.bits.shift(n, false));
                let lu = Unit { hash, bits };

                let hash = &self.put_node(Node::new(Some(lu), Some(ru)))?;
                let bits = cloned.shift(n, true);
                self.put_node(Node::new(Some(Unit { hash, bits }), rc))
            }
        }
    }

    pub fn get(&self, root: Option<&Hash>, key: &[u8]) -> Option<Hash> {
        match root {
            None => None,
            Some(root) => self.find_key(root, Bits::new(key)).ok(),
        }
    }

    fn find_key(&self, root: &[u8], bits: Bits) -> Result<Hash> {
        let bytes = self.db.get(root)?;
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().unwrap();
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => Ok(slice_to_hash(unit.hash)?),
            n if n == unit.bits.len() => self.find_key(&unit.hash, bits.shift(n, false)),
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
            Some(root) => self.gen_proof(root, Bits::new(key), &mut proof).ok(),
        }
    }

    fn gen_proof(&self, root: &[u8], bits: Bits, proof: &mut Proof) -> Result<Proof> {
        let bytes = self.db.get(root)?;
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().unwrap();
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                Ok(proof.to_owned())
            }
            n if n == unit.bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                self.gen_proof(unit.hash, bits.shift(n, false), proof)
            }
            _ => Err(Errors::new("Abort: key not found")),
        }
    }

    fn encode_proof(&self, bytes: &[u8], right: bool) -> Result<(bool, Vec<u8>)> {
        match Node::from_bytes(bytes)? {
            Node::Soft(_) => Ok((false, bytes[HASH_LEN..].to_vec())),
            Node::Hard(_, _) => {
                if right {
                    Ok((
                        true,
                        [&bytes[..bytes.len() - HASH_LEN - 1], &[0x01]].concat(),
                    ))
                } else {
                    Ok((false, bytes[HASH_LEN..].to_vec()))
                }
            }
        }
    }
}

/// No need to be a member of MonoTree.
/// This verification must be called independantly upon request.
pub fn verify_proof(root: Option<&Hash>, leaf: &Hash, proof: &[(bool, Vec<u8>)]) -> bool {
    let mut hash = leaf.to_vec();
    proof.iter().rev().for_each(|(right, cut)| {
        if *right {
            let l = cut.len();
            let o = [&cut[..l - 1], &hash[..], &cut[l - 1..]].concat();
            hash = blake2b(HASH_LEN, &[], &o).as_bytes().to_vec();
        } else {
            let o = [&hash[..], &cut[..]].concat();
            hash = blake2b(HASH_LEN, &[], &o).as_bytes().to_vec();
        }
    });
    root.unwrap().to_vec() == hash
}
