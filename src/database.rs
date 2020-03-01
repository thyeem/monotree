use crate::{Database, Errors, Result};
use hashbrown::HashMap;
use rocksdb::DB;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MemoryDB {
    db: HashMap<Vec<u8>, Vec<u8>>,
    dbname: String,
}

impl Database for MemoryDB {
    fn new(dbname: &str) -> Self {
        MemoryDB {
            db: HashMap::new(),
            dbname: dbname.to_string(),
        }
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        self.db.get(key).map_or(Ok(vec![]), |r| Ok(r.to_vec()))
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.db.insert(key.to_vec(), value);
        Ok(())
    }
}

impl From<rocksdb::Error> for Errors {
    fn from(err: rocksdb::Error) -> Self {
        Errors::new(err.description())
    }
}

#[derive(Clone, Debug)]
pub struct RocksDB {
    db: Arc<Mutex<DB>>,
    dbpath: String,
}

impl Database for RocksDB {
    fn new(dbpath: &str) -> Self {
        let path = Path::new(dbpath);
        let db = Arc::new(Mutex::new(DB::open_default(path).unwrap()));
        RocksDB {
            db,
            dbpath: dbpath.to_string(),
        }
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let db = self.db.lock().unwrap();
        match db.get(key) {
            Ok(Some(value)) => Ok(value.to_vec()),
            Ok(None) => Ok(vec![]),
            Err(_) => Err(Errors::new("Erorr: rocksdb.get()")),
        }
    }
    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.put(key, value)
            .map_err(|_| Errors::new("Error: rocksdb.put()"))
    }
}
