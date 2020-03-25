//! A module for implementing database supporting `monotree`.
use crate::*;
use hashbrown::{HashMap, HashSet};
use rocksdb::{WriteBatch, DB};
use std::path::Path;
use std::sync::{Arc, Mutex};
use utils::*;

struct MemCache {
    set: HashSet<Hash>,
    map: HashMap<Hash, Vec<u8>>,
}

impl MemCache {
    fn new() -> Self {
        MemCache {
            set: HashSet::new(),
            map: HashMap::with_capacity(1 << 12),
        }
    }

    fn clear(&mut self) {
        self.set.clear();
        self.map.clear();
    }

    fn contains(&self, key: &[u8]) -> bool {
        self.set.contains(key) || self.map.contains_key(key)
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.map.get(key) {
            Some(v) => Ok(Some(v.to_owned())),
            None => Ok(None),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.map.insert(slice_to_hash(key), value);
        if self.set.contains(key) {
            self.set.remove(key);
        }
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.map.remove(key);
        self.set.insert(slice_to_hash(key));
        Ok(())
    }
}

/// A trait defining databases used for `monotree`.
pub trait Database {
    fn new(dbpath: &str) -> Self;
    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn init_batch(&mut self) -> Result<()>;
    fn finish_batch(&mut self) -> Result<()>;
}

/// A database using `HashMap`.
pub struct MemoryDB {
    db: HashMap<Hash, Vec<u8>>,
}

impl Database for MemoryDB {
    fn new(_dbname: &str) -> Self {
        MemoryDB { db: HashMap::new() }
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.db.get(key) {
            Some(v) => Ok(Some(v.to_owned())),
            None => Ok(None),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.db.insert(slice_to_hash(key), value);
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.db.remove(key);
        Ok(())
    }

    fn init_batch(&mut self) -> Result<()> {
        Ok(())
    }

    fn finish_batch(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A database using rust wrapper for `RocksDB`.
pub struct RocksDB {
    db: Arc<Mutex<DB>>,
    batch: WriteBatch,
    cache: MemCache,
    batch_on: bool,
}

impl From<rocksdb::Error> for Errors {
    fn from(err: rocksdb::Error) -> Self {
        Errors::new(&err.to_string())
    }
}

impl Database for RocksDB {
    fn new(dbpath: &str) -> Self {
        let db = Arc::new(Mutex::new(
            DB::open_default(Path::new(dbpath)).expect("new(): rocksdb"),
        ));
        RocksDB {
            db,
            batch: WriteBatch::default(),
            cache: MemCache::new(),
            batch_on: false,
        }
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if self.cache.contains(key) {
            return self.cache.get(key);
        }
        let db = self.db.lock().expect("get(): rocksdb");
        match db.get(key)? {
            Some(value) => {
                self.cache.put(key, value.to_owned())?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.cache.put(key, value.to_owned())?;
        if self.batch_on {
            Ok(self.batch.put(key, value)?)
        } else {
            let db = self.db.lock().expect("put(): rocksdb");
            Ok(db.put(key, value)?)
        }
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.cache.delete(key)?;
        if self.batch_on {
            Ok(self.batch.delete(key)?)
        } else {
            let db = self.db.lock().expect("remove(): rocksdb");
            Ok(db.delete(key)?)
        }
    }

    fn init_batch(&mut self) -> Result<()> {
        self.batch = WriteBatch::default();
        self.cache.clear();
        self.batch_on = true;
        Ok(())
    }

    fn finish_batch(&mut self) -> Result<()> {
        self.batch_on = false;
        if !self.batch.is_empty() {
            let batch = std::mem::take(&mut self.batch);
            let db = self.db.lock().expect("write_batch(): rocksdb");
            db.write(batch)?;
        }
        Ok(())
    }
}

/// A database using `Sled`, a pure-rust-implmented DB.
pub struct Sled {
    db: sled::Db,
    batch: sled::Batch,
    cache: MemCache,
    batch_on: bool,
}

impl From<sled::Error> for Errors {
    fn from(err: sled::Error) -> Self {
        Errors::new(&err.to_string())
    }
}

impl Sled {
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

impl Database for Sled {
    fn new(dbpath: &str) -> Self {
        let db = sled::open(dbpath).expect("new(): sledDB");
        Sled {
            db,
            batch: sled::Batch::default(),
            cache: MemCache::new(),
            batch_on: false,
        }
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if self.cache.contains(key) {
            return self.cache.get(key);
        }
        match self.db.get(key)? {
            Some(value) => {
                self.cache.put(key, value.to_vec())?;
                Ok(Some(value.to_vec()))
            }
            None => Ok(None),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.cache.put(key, value.to_owned())?;
        if self.batch_on {
            self.batch.insert(key, value);
        } else {
            self.db.insert(key, value)?;
        }
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.cache.delete(key)?;
        if self.batch_on {
            self.batch.remove(key);
        } else {
            self.db.remove(key)?;
        }
        Ok(())
    }

    fn init_batch(&mut self) -> Result<()> {
        self.batch = sled::Batch::default();
        self.cache.clear();
        self.batch_on = true;
        Ok(())
    }

    fn finish_batch(&mut self) -> Result<()> {
        self.batch_on = false;
        let batch = std::mem::take(&mut self.batch);
        self.db.apply_batch(batch)?;
        Ok(())
    }
}
