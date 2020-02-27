#![allow(non_snake_case)]

use crate::utils::*;
use crate::{
    BytesResult, BytesTuple, Database, Node, ParseHardResult, ParseSoftResult, Proof, ProofResult,
    Result,
};

/// How to generate a new binary radix tree
/// ```rust, ignore
///     //--- gen random key-value pair
///     let const HASH_BYTE = 32;
///     let kv_pair: Vec<BytesTuple> = (0..10000)
///         .map(|_| {
///             (
///                 random_bytes(HASH_BYTE).unwrap(),
///                 random_bytes(HASH_BYTE).unwrap()
///             )
///         })
///         .collect();

///     //--- init tree using either MemDB or RocksDB
///     let mut tree = tree::BinaryRadix::<database::MemoryDB>::new(HASH_BYTE, "");
///     let mut tree = tree::BinaryRadix::<database::RocksdbDB>::new(HASH_BYTE, "");
///     let mut root = tree.new_tree()?;
///
///     //--- simple, but rubust basic operation test.
///     //--- uncomment the last part when testing inclusion-merkle-proof
///     for (k, v) in kv_pair {
///        root = tree.update(&root, &key, &value)?;
///        assert_eq!(t.get_leaf(&r, &k).unwrap(), *v);
///        // let pf = t.get_merkle_proof(&r, &k)?;
///        // assert_eq!(verify_proof(32, &r, &v, &pf), true);
///    }
/// ```

pub struct BinaryRadix<T>
where
    T: Database,
{
    db: T,
    pub nbyte: usize,
}

impl<T> BinaryRadix<T>
where
    T: Database,
{
    pub fn new(nbyte: usize, dbpath: &str) -> Self {
        let db = Database::new(dbpath);
        BinaryRadix { db, nbyte }
    }

    pub fn new_tree(&mut self) -> BytesResult {
        self.db.put(&[], vec![])?;
        Ok(vec![])
    }

    fn encode_soft_node(&self, h: &[u8], b: &[bool]) -> BytesResult {
        Ok([encode_node(h, b, false)?.as_slice(), &[0u8]].concat())
    }

    fn decode_soft_node(&self, bytes: &[u8]) -> ParseSoftResult {
        let (h, b, _) = decode_node(&bytes[..bytes.len() - 1], self.nbyte, false)?;
        Ok((h, b))
    }

    fn encode_hard_node(&self, h: &[u8], b: &[bool], H: &[u8], B: &[bool]) -> BytesResult {
        Ok([
            encode_node(h, b, false)?.as_slice(),
            encode_node(H, B, true)?.as_slice(),
            &[1u8],
        ]
        .concat())
    }

    fn decode_hard_node(&self, bytes: &[u8]) -> ParseHardResult {
        let (h, b, size) = decode_node(&bytes[..bytes.len() - 1], self.nbyte, false)?;
        let (H, B, _) = decode_node(&bytes[size..bytes.len() - 1], self.nbyte, true)?;
        Ok((h, b, H, B))
    }

    fn gen_node(&self, h: &[u8], b: &[bool], H: &[u8], B: &[bool]) -> BytesResult {
        match type_from_parsed(h, b, H, B) {
            Node::Soft => self.encode_soft_node(h, b),
            Node::Hard => match is_rbit!(b) {
                true => self.encode_hard_node(H, B, h, b),
                false => self.encode_hard_node(h, b, H, B),
            },
        }
    }

    fn put_node(&mut self, h: &[u8], b: &[bool], H: &[u8], B: &[bool]) -> BytesResult {
        let node = self.gen_node(h, b, H, B)?;
        let hash = hash(self.nbyte, &node)?;
        self.db.put(&hash, node)?;
        Ok(hash)
    }

    fn get_node(&self, hash: &[u8], bits: &[bool]) -> ParseHardResult {
        let node = self.db.get(hash)?;
        match type_from_bytes(&node) {
            Node::Soft => {
                let (h, b) = self.decode_soft_node(&node)?;
                Ok((h, b, vec![], vec![]))
            }
            Node::Hard => {
                let (h, b, H, B) = self.decode_hard_node(&node)?;
                match is_rbit!(bits) {
                    true => Ok((H, B, h, b)),
                    false => Ok((h, b, H, B)),
                }
            }
        }
    }

    fn put(&mut self, root: &[u8], bits: &[bool], leaf: &[u8]) -> BytesResult {
        let (h, b, H, B) = self.get_node(root, bits)?;
        let n = len_lcp(&b, bits);
        if n == 0 {
            self.put_node(&h, &b, leaf, bits)
        } else if n == bits.len() {
            self.put_node(leaf, &b, &H, &B)
        } else if n == b.len() {
            let h = self.put(&h, &bits[n..], leaf)?;
            self.put_node(&h, &b, &H, &B)
        } else {
            let (h, b) = (self.put_node(&h, &b[n..], leaf, &bits[n..])?, &b[..n]);
            self.put_node(&h, &b, &H, &B)
        }
    }

    pub fn update(&mut self, root: &[u8], key: &[u8], leaf: &[u8]) -> BytesResult {
        let bits = bytes_to_bits(key)?;
        if root.is_empty() {
            self.put_node(leaf, &bits, &[], &[])
        } else {
            self.put(root, &bits, leaf)
        }
    }

    pub fn get_leaf(&self, root: &[u8], key: &[u8]) -> BytesResult {
        let bits = bytes_to_bits(key)?;
        return self.get(root, &bits);
    }

    fn get(&self, root: &[u8], bits: &[bool]) -> BytesResult {
        let (h, b, _, _) = self.get_node(root, bits)?;
        let n = len_lcp(&b, bits);
        if n == bits.len() {
            Ok(h)
        } else if n == b.len() {
            self.get(&h, &bits[n..])
        } else {
            Ok(vec![])
        }
    }

    /// generating proof -------------------------------------------------------
    /// in order to prove proofs, use verify_proof() in utils.rs
    /// ```rust, ignore
    ///     let proof = tree.get_merkle_proof(&root, &key)?;
    ///     assert_eq!(utils::verify_proof(32, &root, &value, &proof), true);
    /// ```

    pub fn get_merkle_proof(&self, root: &[u8], key: &[u8]) -> ProofResult {
        let mut proof: Proof = Vec::new();
        if root.is_empty() {
            Ok(proof)
        } else {
            let bits = bytes_to_bits(key)?;
            Ok(self.get_proof(root, &bits, &mut proof)?)
        }
    }

    fn get_proof(&self, root: &[u8], bits: &[bool], proof: &mut Proof) -> ProofResult {
        let (h, b, H, B) = self.get_node(root, bits)?;
        let n = len_lcp(&b, bits);
        if n == bits.len() {
            let node = self.gen_node(&h, bits, &H, &B)?;
            proof.push(self.encode_proof(&node, bits)?);
            return Ok(proof.to_owned());
        }
        if n == b.len() {
            let node = self.gen_node(&h, &b, &H, &B)?;
            proof.push(self.encode_proof(&node, bits)?);
            return Ok(self.get_proof(&h, &bits[n..], proof)?);
        }
        Ok(proof.to_owned())
    }

    fn encode_proof(&self, bytes: &[u8], bits: &[bool]) -> Result<BytesTuple> {
        match type_from_bytes(&bytes) {
            Node::Soft => Ok((vec![0u8], bytes[self.nbyte..].to_vec())),
            Node::Hard => match is_rbit!(bits) {
                true => Ok((
                    vec![1u8],
                    [&bytes[..bytes.len() - 1 - self.nbyte], &[1u8]].concat(),
                )),
                false => Ok((vec![0u8], bytes[self.nbyte..].to_vec())),
            },
        }
    }
}
