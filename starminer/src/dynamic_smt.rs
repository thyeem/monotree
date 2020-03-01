use super::database::MemoryDatabase;
use super::merkletrie_interface::MerkletrieDatabase;
use super::merkletrie_interface::MerkletrieInterface;
use bitvec::prelude::*;
use failure::Error;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Instant;
type SmtBytes = BitVec<Msb0, u8>; // big endian
type SmtSlice = BitSlice<Msb0, u8>; // big endian

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SparseMerkletrie<T>
where
    T: MerkletrieDatabase,
{
    database: T,
    root: Node,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Node {
    pub children: BTreeMap<SmtBytes, Vec<u8>>,
    pub value: Vec<u8>,
}

impl<T> MerkletrieInterface for SparseMerkletrie<T>
where
    T: MerkletrieDatabase,
{
    fn load(&mut self, hash: &[u8]) -> Result<(), Error> {
        let node_found = self.read_node(&hash)?;
        self.root = node_found;
        Ok(())
    }

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.put(key, value);
        Ok(())
    }

    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.get(key)
    }

    fn get_roothash(&self) -> Result<Vec<u8>, Error> {
        self.get_hash(&self.root)
    }
}

impl<T> SparseMerkletrie<T>
where
    T: MerkletrieDatabase,
{
    pub fn new(database: T) -> Self {
        SparseMerkletrie {
            database,
            root: Node::default(),
        }
    }

    // encoded, hash
    fn get_hash(&self, n: &Node) -> Result<Vec<u8>, Error> {
        Ok(self.get_encoded_hash(n)?.1)
    }

    // encoded, hash
    pub fn get_encoded_hash(&self, n: &Node) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let encoded: Vec<u8> = bincode::serialize(&n)?;
        let hash = self.database.compute_hash(&encoded.as_slice());
        Ok((encoded.to_vec(), hash))
    }

    fn write_node(&mut self, n: &Node) -> Result<Vec<u8>, Error> {
        let (encoded, hash) = self.get_encoded_hash(n)?;
        self.database.write(&hash, &encoded[..])?;
        Ok(hash)
    }

    fn read_node(&self, key: &[u8]) -> Result<Node, Error> {
        let data = self.database.read(key)?;
        let decoded: Node = bincode::deserialize(&data[..])?;
        Ok(decoded)
    }

    pub fn show_root(&self) {
        let (_encoded, hash) = self.get_encoded_hash(&self.root).expect("compute hash");
        println!("hash= {}", hex::encode(&hash));
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        let mut root = self.root.clone();
        let bits = SmtBytes::from_slice(key);
        let roothash = self.do_put(&bits, value, &mut root).expect("ok");
        let (_encoded, hash) = self.get_encoded_hash(&root).expect("compute hash");
        assert!(hash == roothash);
        self.root = root;
    }

    fn get_common(&self, src: &SmtSlice, src2: &SmtSlice) -> usize {
        let mut n = src.len();
        if src2.len() < n {
            n = src2.len();
        }

        for i in 0..n {
            if src[i] != src2[i] {
                return i;
            }
        }
        n
    }
    pub fn do_put(
        &mut self,
        key_bits: &SmtBytes,
        value: &[u8],
        parent: &mut Node,
    ) -> Result<Vec<u8>, Error> {
        let mut i: usize = key_bits.len();
        let mut common: usize = 0;
        let mut oldbranch: SmtBytes = SmtBytes::default();

        // update
        if parent.children.contains_key(key_bits) {
            let oldhash = parent.children[key_bits].clone();
            let mut oldnode = self.read_node(&oldhash)?;
            oldnode.value = value.to_vec();
            let hash = self.write_node(&oldnode)?;
            parent.children.insert(key_bits.clone(), hash);
            let parenthash = self.write_node(&parent)?;
            return Ok(parenthash);
        }

        // find common key
        loop {
            let key = &key_bits[0..i];
            for k in parent.children.keys() {
                common = self.get_common(&key, &k);
                if common > 0 {
                    oldbranch = k.to_vec();
                    break;
                }
            }
            if common > 0 {
                break;
            }
            //process

            if 0 == i {
                break;
            } else {
                i -= 1;
            }
        }

        //0(includiing) ~ common(excluding)
        let is_leaf = 0 == common;
        if is_leaf {
            let mut new_leaf = Node::default();
            new_leaf.value = value.to_vec();
            let hash = self.write_node(&new_leaf)?;
            parent.children.insert(key_bits.clone(), hash);
            let parenthash = self.write_node(&parent)?;
            Ok(parenthash)
        } else {
            let oldhash = parent.children[&oldbranch].clone();
            let _oldnode = self.read_node(&oldhash)?;
            parent.children.remove(&oldbranch);
            assert!(!parent.children.contains_key(&oldbranch));

            // remove old branch
            let new_branchkey = &oldbranch[0..common];
            let mut new_branch = Node::default();

            // make new children
            // oldnode
            let new_branchkey_a = &oldbranch[common..];
            let new_branchkey_b = &key_bits[common..];

            let new_branch_a_hash = oldhash;
            let _new_branch_b_hash =
                self.do_put(&new_branchkey_b.to_vec(), value, &mut new_branch)?;
            // link
            new_branch
                .children
                .insert(new_branchkey_a.to_vec(), new_branch_a_hash);
            let new_branch_hash = self.write_node(&new_branch)?;
            parent
                .children
                .insert(new_branchkey.to_vec(), new_branch_hash);
            let hash = self.write_node(&parent)?;
            Ok(hash)
        }
    }

    pub fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error> {
        let key_bits = SmtBytes::from_slice(key);
        self.do_get(&key_bits, &self.root)
    }

    pub fn do_get(&self, key_bits: &SmtBytes, parent: &Node) -> Result<Vec<u8>, Error> {
        if parent.children.contains_key(key_bits) {
            let oldhash = parent.children[key_bits].clone();
            let oldnode = self.read_node(&oldhash)?;
            return Ok(oldnode.value);
        }

        let mut i: usize = key_bits.len();
        let mut common: usize = 0;
        let mut oldbranch: SmtBytes = SmtBytes::default();

        // find common key
        loop {
            let key = &key_bits[0..i];
            for k in parent.children.keys() {
                common = self.get_common(&key, &k);
                if common > 0 {
                    oldbranch = k.to_vec();
                    break;
                }
            }
            if common > 0 {
                break;
            }
            if 0 == i {
                break;
            } else {
                i -= 1;
            }
        }

        if 0 == common {
            return Err(format_err!("not found"));
        }
        let is_leaf = 0 == common;
        if is_leaf {
            assert!(parent.children.contains_key(&key_bits.clone()));
            let found = parent.children[&key_bits.clone()].clone();
            let node = self.read_node(&found)?;
            Ok(node.value)
        } else {
            let oldhash = parent.children[&oldbranch].clone();
            let oldnode = self.read_node(&oldhash)?;

            let _new_branchkey = &oldbranch[0..common];
            let new_branchkey_b = &key_bits[common..];
            self.do_get(&new_branchkey_b.to_vec(), &oldnode)
        }
    }
}

pub fn dynamic_sparse_main() -> Result<(), failure::Error> {
    let database = MemoryDatabase::default();
    let mut smt = SparseMerkletrie::new(MemoryDatabase::default());
    //let database = Database::new("./data");
    //let mut smt = SparseMerkletrie::new(database.clone());

    let n = 20000;
    let now = Instant::now();
    for i in 0..n {
        let b = i as i32;
        let value = b.to_le_bytes();
        let key = database.compute_hash(&value);
        smt.put(&key, &value);
    }
    println!("dynamic sparse merkletrie= {}", now.elapsed().as_millis());
    Ok(())
}

pub fn dynamic_sparse_main2() -> Result<(), failure::Error> {
    println!("dynamic_sparse_main");
    let mut smt = SparseMerkletrie::new(MemoryDatabase::default());
    smt.put(&hex::decode("f103")?, &hex::decode("0523")?);
    smt.show_root();
    smt.put(&hex::decode("f101")?, &hex::decode("01")?);
    smt.show_root();
    // smt.put(&hex::decode("f2")?, &hex::decode("01")?);
    //  smt.show_root();
    let a = smt.get(&hex::decode("f101")?)?;
    println!("value={}", hex::encode(&a));

    Ok(())
}
