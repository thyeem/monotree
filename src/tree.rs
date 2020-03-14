use crate::bits::Bits;
use crate::consts::*;
use crate::node::{Node, Unit};
use crate::utils::*;
use crate::{Database, Hash, Proof, Result};
use blake2_rfc::blake2b::blake2b;

/// Example: How to use MonoTree
/// ```rust, ignore
///     //--- prepare random key-value pair like:
///     type Hash = [u8; HASH_LEN]
///     let pairs: Vec<(Hash, Hash)> = (0..1000)
///         .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
///         .collect();
///
///     //--- init tree using either In-Memory HashMap
///     let mut tree = tree::MonoTree::<MemoryDB>::new("MemDB");
///     //--- or RocksDB
///     let mut tree = tree::MonoTree::<RocksdbDB>::new("RocksDB");
///     let mut root = tree.new_tree();
///
///    //--- functional test: insert/get
///    pairs.iter().enumerate().for_each(|(i, (key, value))| {
///        // insert a key
///        root = tree.insert(root.as_ref(), key, value).unwrap();
///        pairs.iter().take(i + 1).for_each(|(k, v)| {
///            // check if the key-value pair was correctly inserted so far
///            assert_eq!(tree.get(root.as_ref(), k).unwrap(), Some(*v));
///        });
///    });
///    assert_ne!(root, None);
///
///    //--- functional test: remove
///    pairs.iter().enumerate().for_each(|(i, (key, _))| {
///        assert_ne!(root, None);
///        // assert that all values are fine after deletion
///        pairs.iter().skip(i).for_each(|(k, v)| {
///            assert_eq!(tree.get(root.as_ref(), k).unwrap(), Some(*v));
///            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
///            assert_eq!(tree::verify_proof(root.as_ref(), v, &proof), true);
///        });
///        // delete a key
///        root = tree.remove(root.as_ref(), key).unwrap();
///        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
///    });
///    // back to inital state of tree
///    assert_eq!(root, None);
///    });
/// ```

#[derive(Clone, Debug)]
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

    pub fn close(&mut self) -> Result<()> {
        self.db.close()
    }

    pub fn new_tree(&mut self) -> Option<Hash> {
        None
    }

    pub fn inserts(
        &mut self,
        root: Option<&Hash>,
        keys: &[Hash],
        leaves: &[Hash],
    ) -> Result<Option<Hash>> {
        let mut root = root.cloned();
        self.db.init_batch()?;
        for (key, leaf) in keys.iter().zip(leaves.iter()) {
            root = self.insert(root.as_ref(), key, leaf)?;
        }
        self.db.write_batch()?;
        Ok(root)
    }

    pub fn insert(&mut self, root: Option<&Hash>, key: &Hash, leaf: &Hash) -> Result<Option<Hash>> {
        match root {
            None => {
                let (hash, bits) = (leaf, Bits::new(key));
                self.put_node(Node::new(Some(Unit { hash, bits }), None))
            }
            Some(root) => self.put(root, Bits::new(key), leaf),
        }
    }

    fn put_node(&mut self, node: Node) -> Result<Option<Hash>> {
        let bytes = node.to_bytes()?;
        let hash = blake2b(HASH_LEN, &[], &bytes);
        self.db.put(hash.as_bytes(), bytes)?;
        Ok(slice_to_hash(hash.as_bytes()))
    }

    fn put(&mut self, root: &[u8], bits: Bits, leaf: &[u8]) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?;
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("put(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == 0 => self.put_node(Node::new(lc, Some(Unit { hash: leaf, bits }))),
            n if n == bits.len() => self.put_node(Node::new(Some(Unit { hash: leaf, bits }), rc)),
            n if n == unit.bits.len() => {
                let hash = &self
                    .put(unit.hash, bits.shift(n, false), leaf)?
                    .expect("put(): hash");
                let unit = unit.to_owned();
                self.put_node(Node::new(Some(Unit { hash, ..unit }), rc))
            }
            _ => {
                let bits = bits.shift(n, false);
                let ru = Unit { hash: leaf, bits };

                let (cloned, unit) = (unit.bits.clone(), unit.to_owned());
                let (hash, bits) = (unit.hash, unit.bits.shift(n, false));
                let lu = Unit { hash, bits };

                let hash = &self
                    .put_node(Node::new(Some(lu), Some(ru)))?
                    .expect("put(): hash");
                let bits = cloned.shift(n, true);
                self.put_node(Node::new(Some(Unit { hash, bits }), rc))
            }
        }
    }

    pub fn get(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.find_key(root, Bits::new(key)),
        }
    }

    fn find_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?;
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().expect("find_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => Ok(slice_to_hash(unit.hash)),
            n if n == unit.bits.len() => self.find_key(&unit.hash, bits.shift(n, false)),
            _ => Ok(None),
        }
    }

    pub fn remove(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.delete_key(root, Bits::new(key)),
        }
    }

    fn delete_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?;
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("delete_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => {
                self.db.remove(&unit.hash)?;
                match rc {
                    Some(_) => self.put_node(Node::new(None, rc)),
                    None => Ok(None),
                }
            }
            n if n == unit.bits.len() => {
                let hash = self.delete_key(&unit.hash, bits.shift(n, false))?;
                match (hash, &rc) {
                    (None, None) => Ok(None),
                    (None, Some(_)) => self.put_node(Node::new(None, rc)),
                    (Some(ref hash), _) => {
                        let unit = unit.to_owned();
                        let lc = Some(Unit { hash, ..unit });
                        self.put_node(Node::new(lc, rc))
                    }
                }
            }
            _ => Ok(None),
        }
    }

    /// Merkle proof secion: verifying inclusion of data -----------------------
    /// In order to prove proofs, use verify_proof() at the end of file below.
    /// Example:
    /// ```rust, ignore
    ///     // suppose (key: Hash, value: Hash) alreay prepared.
    ///     // let mut root = ...
    ///     root = tree.insert(&root, &key, &value);
    ///     ...
    ///     let leaf = tree.get(root.as_ref(), &key).unwrap();
    ///     let proof = tree.get_merkle_proof(root.as_ref(), &key)?.unwrap();
    ///     assert_eq!(tree::verify_proof(root.as_ref(), &leaf, &proof), true);
    /// ```
    ///
    /// Test operation guaranteed:
    /// ```rust, ignore
    ///    pairs.iter().enumerate().for_each(|(i, (key, value))| {
    ///        // insert a key
    ///        root = tree.insert(root.as_ref(), key, value).unwrap();
    ///        pairs.iter().take(i + 1).for_each(|(k, v)| {
    ///            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
    ///            // gen/verify Merkle proof with all keys so far
    ///            assert_eq!(tree::verify_proof(root.as_ref(), v, &proof), true);
    ///        });
    ///    });
    /// ```

    pub fn get_merkle_proof(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Proof>> {
        let mut proof: Proof = Vec::new();
        match root {
            None => Ok(None),
            Some(root) => self.gen_proof(root, Bits::new(key), &mut proof),
        }
    }

    fn gen_proof(&mut self, root: &[u8], bits: Bits, proof: &mut Proof) -> Result<Option<Proof>> {
        let bytes = self.db.get(root)?;
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().expect("gen_proof(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                Ok(Some(proof.to_owned()))
            }
            n if n == unit.bits.len() => {
                proof.push(self.encode_proof(&bytes, bits.first())?);
                self.gen_proof(unit.hash, bits.shift(n, false), proof)
            }
            _ => Ok(None),
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
    root.expect("verify_proof(): root").to_vec() == hash
}
