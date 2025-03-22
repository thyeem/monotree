//! A module implementing `monotree`.
use crate::utils::*;
use crate::*;

/// A structure for `monotree`.
#[derive(Debug)]
pub struct Monotree<D = DefaultDatabase, H = DefaultHasher> {
    db: D,
    hasher: H,
}

impl Default for Monotree<DefaultDatabase, DefaultHasher> {
    fn default() -> Self {
        Self::new("monotree")
    }
}

impl<D, H> Monotree<D, H>
where
    D: Database,
    H: Hasher,
{
    pub fn new(dbpath: &str) -> Self {
        let db = Database::new(dbpath);
        let hasher = Hasher::new();
        Monotree { db, hasher }
    }

    /// Retrieves the latest state (root) from the database.
    pub fn get_headroot(&mut self) -> Result<Option<Hash>> {
        let headroot = self.db.get(ROOT_KEY)?;
        match headroot {
            Some(root) => Ok(Some(slice_to_hash(&root))),
            None => Ok(None),
        }
    }

    /// Sets the latest state (root) to the database.
    pub fn set_headroot(&mut self, headroot: Option<&Hash>) {
        if let Some(root) = headroot {
            self.db
                .put(ROOT_KEY, root.to_vec())
                .expect("set_headroot(): hash");
        }
    }

    pub fn prepare(&mut self) {
        self.db
            .init_batch()
            .expect("prepare(): failed to initialize batch");
    }

    pub fn commit(&mut self) {
        self.db
            .finish_batch()
            .expect("commit(): failed to finalize batch");
    }

    /// Insert key-leaf entry into the `monotree`. Returns a new root hash.
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
        let hash = self.hasher.digest(&bytes);
        self.db.put(&hash, bytes)?;
        Ok(Some(hash))
    }

    /// Recursively insert a bytes (in forms of Bits) and a leaf into the tree.
    ///
    /// Optimization in `monotree` is mainly to compress the path as much as possible
    /// while reducing the number of db accesses using the most intuitive model.
    /// As a result, compared to the standard Sparse Merkle Tree,
    /// this reduces the number of DB accesses from `N` to `log2(N)` in both reads and writes.
    ///
    /// Whenever invoked a `put()` call, at least, more than one `put_node()` called,
    /// which triggers a single hash digest + a single DB write.
    /// Compressing the path recudes the number of `put()` calls, which yields
    /// reducing the number of hash function calls as well as the number of DB writes.
    ///
    /// There are four modes when putting the entries.
    /// And each of them is processed in a (recursive) `put()` call.
    /// The number in parenthesis refers to the minimum of DB access and hash fn call required.
    ///
    /// * set-aside (1)
    ///     putting the leaf to the next node in the current depth.
    /// * replacement (1)
    ///     replacement the existing node on the path with the new leaf.
    /// * consume & pass-over (2+)
    ///     consuming the path on the way, then pass the rest of work to their child node.
    /// * split-node (2)
    ///     immediately split node into two with the longest common prefix,
    ///     then wind the recursive stack from there returning resulting hashes.
    fn put(&mut self, root: &[u8], bits: Bits, leaf: &[u8]) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("put(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            0 => self.put_node(Node::new(lc, Some(Unit { hash: leaf, bits }))),
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

                // ENFORCE DETERMINISTIC ORDERING
                let (left, right) = if lu.bits < ru.bits {
                    (lu, ru)
                } else {
                    (ru, lu)
                };

                let hash = &self
                    .put_node(Node::new(Some(left), Some(right)))?
                    .expect("put(): hash");
                let bits = cloned.shift(n, true);
                self.put_node(Node::new(Some(Unit { hash, bits }), rc))
            }
        }
    }

    /// Get a leaf hash for the given root and key.
    pub fn get(&mut self, root: Option<&Hash>, key: &Hash) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.find_key(root, Bits::new(key)),
        }
    }

    fn find_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (cell, _) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = cell.as_ref().expect("find_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => Ok(Some(slice_to_hash(unit.hash))),
            n if n == unit.bits.len() => self.find_key(unit.hash, bits.shift(n, false)),
            _ => Ok(None),
        }
    }

    /// Remove the given key and its corresponding leaf from the tree. Returns a new root hash.
    pub fn remove(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Hash>> {
        match root {
            None => Ok(None),
            Some(root) => self.delete_key(root, Bits::new(key)),
        }
    }

    fn delete_key(&mut self, root: &[u8], bits: Bits) -> Result<Option<Hash>> {
        let bytes = self.db.get(root)?.expect("bytes");
        let (lc, rc) = Node::cells_from_bytes(&bytes, bits.first())?;
        let unit = lc.as_ref().expect("delete_key(): left-unit");
        let n = Bits::len_common_bits(&unit.bits, &bits);
        match n {
            n if n == bits.len() => match rc {
                Some(_) => self.put_node(Node::new(None, rc)),
                None => Ok(None),
            },
            n if n == unit.bits.len() => {
                let hash = self.delete_key(unit.hash, bits.shift(n, false))?;
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

    /// This method is intended to use the `insert()` method in batch mode.
    /// Note that `inserts()` forces the batch to 'commit'.
    pub fn inserts(
        &mut self,
        root: Option<&Hash>,
        keys: &[Hash],
        leaves: &[Hash],
    ) -> Result<Option<Hash>> {
        let indices = get_sorted_indices(keys, false);
        self.prepare();
        let mut root = root.cloned();
        for i in indices.iter() {
            root = self.insert(root.as_ref(), &keys[*i], &leaves[*i])?;
        }
        self.commit();
        Ok(root)
    }

    /// This method is intended to use the `get()` method in batch mode.
    pub fn gets(&mut self, root: Option<&Hash>, keys: &[Hash]) -> Result<Vec<Option<Hash>>> {
        let mut leaves: Vec<Option<Hash>> = Vec::new();
        for key in keys.iter() {
            leaves.push(self.get(root, key)?);
        }
        Ok(leaves)
    }

    /// This method is intended to use the `remove()` method in batch mode.
    /// Note that `removes()` forces the batch to 'commit'.
    pub fn removes(&mut self, root: Option<&Hash>, keys: &[Hash]) -> Result<Option<Hash>> {
        let indices = get_sorted_indices(keys, false);
        let mut root = root.cloned();
        self.prepare();
        for i in indices.iter() {
            root = self.remove(root.as_ref(), &keys[*i])?;
        }
        self.commit();
        Ok(root)
    }

    /// Generate a Merkle proof for the given root and key.
    pub fn get_merkle_proof(&mut self, root: Option<&Hash>, key: &[u8]) -> Result<Option<Proof>> {
        let mut proof: Proof = Vec::new();
        match root {
            None => Ok(None),
            Some(root) => self.gen_proof(root, Bits::new(key), &mut proof),
        }
    }

    fn gen_proof(&mut self, root: &[u8], bits: Bits, proof: &mut Proof) -> Result<Option<Proof>> {
        let bytes = self.db.get(root)?.expect("bytes");
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

/// Verify a Merkle proof with the given root, leaf and hasher if the proof is valid or not.
///
/// Be aware of that it fails if not provided a suitable hasher used in the tree
/// This generic fn must be independantly called upon request, not a member of Monotree.
pub fn verify_proof<H: Hasher>(
    hasher: &H,
    root: Option<&Hash>,
    leaf: &Hash,
    proof: Option<&Proof>,
) -> bool {
    match proof {
        None => false,
        Some(proof) => {
            let mut hash = leaf.to_owned();
            proof.iter().rev().for_each(|(right, cut)| {
                if *right {
                    let l = cut.len();
                    let o = [&cut[..l - 1], &hash[..], &cut[l - 1..]].concat();
                    hash = hasher.digest(&o);
                } else {
                    let o = [&hash[..], &cut[..]].concat();
                    hash = hasher.digest(&o);
                }
            });
            root.expect("verify_proof(): root") == &hash
        }
    }
}
