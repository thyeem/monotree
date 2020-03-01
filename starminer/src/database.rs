use super::merkletrie_interface::MerkletrieDatabase;
use blake2::{Blake2s, Digest};
use failure::Error;
use rocksdb::DB;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type DBShared = Arc<Mutex<DB>>;

#[derive(Clone, Debug)]
pub struct Database {
    path: String,
    db: DBShared,
}

impl MerkletrieDatabase for Database {
    fn compute_hash(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Blake2s::new();
        hasher.input(data);
        hasher.result().to_vec()
    }
    fn write(&mut self, key: &[u8], data: &[u8]) -> Result<(), Error> {
        self.write_db(key, data)
    }
    fn read(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.read_db(key)
    }
}
impl Database {
    pub fn new(path: &str) -> Database {
        let db = Arc::new(Mutex::new(DB::open_default(path).unwrap()));
        Database {
            path: path.to_string(),
            db,
        }
    }

    pub fn write_db(&self, key: &[u8], data: &[u8]) -> Result<(), Error> {
        let db = self.db.lock().unwrap();
        db.put(key, data)
            .map_err(|_e| format_err!("write_db error"))
    }

    pub fn read_db(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        let db = self.db.lock().unwrap();
        match db.get(key) {
            Ok(Some(value)) => Ok(value.to_vec()),
            Ok(None) => Err(format_err!("no db value")),
            Err(_e) => Err(format_err!("no db value2")),
        }
    }

    /*

        pub fn write_u64(&self, key: &str, data: u64) -> Result<(), Error> {
            self.write_db(key.as_bytes(), &data.to_be_bytes())
        }

        pub fn read_u64(&self, key: &str) -> Result<u64, Error> {
            self.read_db(key.as_bytes()).map(|a| {
                let mut m: [u8; 8] = [0; 8];
                m.copy_from_slice(&a[0..8]);
                u64::from_be_bytes(m)
            })
        }
    */

    #[allow(dead_code)]
    pub fn write_db2(&self, key: &[u8], data: &[u8]) -> Result<(), Error> {
        self.write_db(key, data)
    }

    #[allow(dead_code)]
    pub fn read_db2(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        self.read_db(key)
    }

    #[allow(dead_code)]
    pub fn write_string(&self, key: &str, data: &str) -> Result<(), Error> {
        self.write_db(key.as_bytes(), data.as_bytes())
    }

    #[allow(dead_code)]
    pub fn read_string(&self, key: &str) -> Result<String, Error> {
        self.read_db(key.as_bytes())
            .map(|a| std::str::from_utf8(a.as_slice()).unwrap().to_string())
    }

    #[allow(dead_code)]
    pub fn write_i64(&self, key: &str, data: i64) -> Result<(), Error> {
        self.write_db(key.as_bytes(), &data.to_be_bytes())
    }

    #[allow(dead_code)]
    pub fn read_i64(&self, key: &str) -> Result<i64, Error> {
        self.read_db(key.as_bytes()).map(|a| {
            let mut m: [u8; 8] = [0; 8];
            m.copy_from_slice(&a[0..8]);
            i64::from_be_bytes(m)
        })
    }

    fn get_new_key(&self, category: &[u8], key: &[u8]) -> Result<Vec<u8>, Error> {
        let mut a: Vec<u8> = Vec::new();
        a.append(&mut category.to_vec());
        a.append(&mut key.to_vec());
        assert!(a.len() == category.len() + key.len());
        Ok(a)
    }

    pub fn put(&self, category: &[u8], key: &[u8], data: &[u8]) -> Result<(), Error> {
        self.write_db(self.get_new_key(category, key)?.as_slice(), data)
    }

    pub fn get(&self, category: &[u8], key: &[u8]) -> Result<Vec<u8>, Error> {
        self.read_db(self.get_new_key(category, key)?.as_slice())
    }

    // for debug
    #[allow(dead_code)]
    pub fn read_string_debug(&self, key: &[u8]) -> Result<String, Error> {
        let value = self.read(key)?;
        String::from_utf8(value).map_err(|e| format_err!("{}", e.to_string()))
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn check_read_string() {}
}

#[derive(Default)]
pub struct MemoryDatabase {
    nodes: HashMap<Vec<u8>, Vec<u8>>,
}

impl MerkletrieDatabase for MemoryDatabase {
    fn compute_hash(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Blake2s::new();
        hasher.input(data);
        hasher.result().to_vec()
    }
    fn write(&mut self, key: &[u8], data: &[u8]) -> Result<(), Error> {
        self.nodes.insert(key.to_vec(), data.to_vec());
        Ok(())
    }
    fn read(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(self.nodes.get(key).unwrap()[..].into())
    }
}
