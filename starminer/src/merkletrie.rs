/*
byte merkletrie
written by Jongwhan Lee
*/

use super::database::MemoryDatabase;
use super::merkletrie_interface::{MerkletrieDatabase, MerkletrieInterface};
use failure::Error;
use log::debug;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Instant;
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Node {
    pub children: BTreeMap<Vec<u8>, Vec<u8>>, // key: address, value: Node Hash
    pub value: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Merkletrie<T>
where
    T: MerkletrieDatabase,
{
    database: T,
    root: Node,
}

impl<T> MerkletrieInterface for Merkletrie<T>
where
    T: MerkletrieDatabase,
{
    fn load(&mut self, hash: &[u8]) -> Result<(), Error> {
        let node_found = self.read_node(&hash)?;
        self.root = node_found;
        Ok(())
    }

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let mut root = self.root.clone();
        let _final_root = self.do_put(&key, 0, &value, &mut root)?;

        self.root = root;
        Ok(())
    }

    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.do_get(&key, 0, &self.root)
    }

    fn get_roothash(&self) -> Result<Vec<u8>, Error> {
        self.get_hash(&self.root)
    }
}

impl<T> Merkletrie<T>
where
    T: MerkletrieDatabase,
{
    #[allow(dead_code)]
    pub fn new(database: T) -> Self {
        Merkletrie {
            database,
            root: Node::default(),
        }
    }

    #[allow(dead_code)]
    pub fn load_hex(&mut self, hash: &str) -> Result<(), Error> {
        self.load(&hex::decode(hash)?)
    }

    #[allow(dead_code)]
    pub fn initialize(&self) {
        debug!("merkletrie initialized");
    }

    // encoded, hash
    fn get_hash(&self, n: &Node) -> Result<Vec<u8>, Error> {
        Ok(self.get_encoded_hash(n)?.1)
    }

    // encoded, hash
    #[allow(dead_code)]
    fn show_node(&self, n: &Node) -> Result<(), Error> {
        debug!(
            "hash={} json={}",
            hex::encode(self.get_hash(n)?),
            serde_json::to_string(&n)?
        );
        Ok(())
    }

    // encoded, hash
    fn get_encoded_hash(&self, n: &Node) -> Result<(Vec<u8>, Vec<u8>), Error> {
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

    pub fn do_put(
        &mut self,
        key: &[u8],
        index: usize,
        value: &[u8],
        parent: &mut Node,
    ) -> Result<Vec<u8>, Error> {
        let current = &key[index..=index];

        let next_index = index + 1;
        let is_leaf = key.len() == next_index;

        if is_leaf {
            let mut newleaf = if parent.children.contains_key(current) {
                let hash_found = &parent.children[current];
                self.read_node(&hash_found)?
            } else {
                Node::default()
            };

            // update
            newleaf.value = value.to_vec();

            // update hash write
            let hash = self.write_node(&newleaf)?;

            parent.children.insert(current.to_vec(), hash);
            // update hash, write
            let parenthash = self.write_node(&parent)?;
            Ok(parenthash)
        } else {
            let mut newbranch = if parent.children.contains_key(current) {
                let hash_found = &parent.children[current];
                self.read_node(&hash_found)?
            } else {
                Node::default()
            };

            // update children
            let child_hash = self.do_put(&key, next_index, &value, &mut newbranch)?;
            // upsert

            parent
                .children
                .insert(current.to_vec(), child_hash.to_vec());

            // update hash, write
            let hash = self.write_node(&parent)?;
            Ok(hash)
        }
    }

    #[allow(dead_code)]
    pub fn put_string(&mut self, key: &str, value: &str) -> Result<(), Error> {
        let key1 = key.as_bytes().to_vec();
        let value1 = value.as_bytes().to_vec();
        self.put(&key1, &value1)
    }

    #[allow(dead_code)]
    pub fn put_hex(&mut self, key: &str, value: &str) -> Result<(), Error> {
        let key1 = hex::decode(key)?;
        let value1 = hex::decode(value)?;
        self.put(&key1, &value1)
    }

    pub fn do_get(&self, key: &[u8], index: usize, parent: &Node) -> Result<Vec<u8>, Error> {
        let current = &key[index..=index];

        let next_index = index + 1;
        let is_leaf = key.len() == next_index;
        if is_leaf {
            let hash = parent.children[current].clone();
            let found_node = self.read_node(&hash)?;
            Ok(found_node.value)
        } else if parent.children.contains_key(current) {
            let childnode = self.read_node(&parent.children[current])?;

            //let next =  Vec<u8>::from(&key[next_index..next_index + 1]);
            self.do_get(&key, next_index, &childnode)
        } else {
            Err(format_err!("key doesn't exist"))
        }
    }

    #[allow(dead_code)]
    pub fn get_hex(&mut self, key: &str) -> Result<String, Error> {
        let key1 = hex::decode(key)?;
        let r = self.get(&key1)?;
        Ok(hex::encode(&r))
    }

    #[allow(dead_code)]
    pub fn get_string(&mut self, key: &str) -> Result<String, Error> {
        let key1 = key.as_bytes().to_vec();
        let r = self.get(&key1)?;
        Ok(std::str::from_utf8(&r)?
            //  .map_err(|e| e.to_string())?
            .to_string())
    }

    #[allow(dead_code)]
    pub fn show_roothash(&self) -> Result<(), Error> {
        println!("final root={}", hex::encode(&self.get_roothash()?));
        Ok(())
    }
}

pub fn test_merkletrie() -> Result<(), failure::Error> {
    let m = MemoryDatabase::default();
    let trie = Merkletrie::new(m);
    trie.show_roothash()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_inserting_order() {
        let mut trie = Merkletrie::new(MemoryDatabase::default());

        trie.put_hex("11cdef", "112200").unwrap();
        trie.put_hex("abcdef", "11223344").unwrap();
        trie.put_hex("11cd", "1122").unwrap();

        let mut trie2 = Merkletrie::new(MemoryDatabase::default());
        trie2.put_hex("11cd", "1122").unwrap();
        trie2.put_hex("11cdef", "112200").unwrap();
        trie2.put_hex("abcdef", "11223344").unwrap();

        assert!(trie.get_roothash().unwrap() == trie2.get_roothash().unwrap());
    }
}

pub fn patricia_main() -> Result<(), failure::Error> {
    //let database = Database::new("./data");
    //let mut smt = Merkletrie::new(database.clone());
    let database = MemoryDatabase::default();
    let mut smt = Merkletrie::new(MemoryDatabase::default());

    let n = 1000;
    let now = Instant::now();
    for i in 0..n {
        let b = i as i32;
        let value = b.to_le_bytes();
        let key = database.compute_hash(&value);

        smt.put(&key, &value)?;
    }
    println!("patricia merkletrie= {}", now.elapsed().as_millis());
    Ok(())
}

fn test1() -> Result<(), failure::Error> {
    let _database = MemoryDatabase::default();
    let mut smt = Merkletrie::new(MemoryDatabase::default());

    smt.put(&hex::decode("1234")?, &hex::decode("fe2a")?)?;
    smt.put(&hex::decode("5212")?, &hex::decode("3f4b")?)?;
    println!("{}", &hex::encode(&smt.get_roothash()?));
    Ok(())
}

fn test2() -> Result<(), failure::Error> {
    let _database = MemoryDatabase::default();
    let mut smt = Merkletrie::new(MemoryDatabase::default());

    smt.put(&hex::decode("5212")?, &hex::decode("3f4b")?)?;

    smt.put(&hex::decode("1234")?, &hex::decode("fe2a")?)?;
    println!("{}", &hex::encode(&smt.get_roothash()?));
    Ok(())
}

pub fn patricia_order() -> Result<(), failure::Error> {
    test1()?;
    test2()?;
    Ok(())
}
