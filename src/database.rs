//! A module for implementing database supporting `monotree`.
use crate::*;
use hashbrown::HashMap;
use utils::*;

mod cache {
    use super::*;
    use std::collections::HashSet;

    pub(crate) struct MemCache {
        pub(crate) set: HashSet<Hash>,
        pub(crate) map: HashMap<Hash, Vec<u8>>,
    }

    impl MemCache {
        pub(crate) fn new() -> Self {
            Self {
                set: HashSet::new(),
                map: HashMap::with_capacity(1 << 12),
            }
        }

        pub(crate) fn clear(&mut self) {
            self.set.clear();
            self.map.clear();
        }

        pub(crate) fn contains(&self, key: &[u8]) -> bool {
            !self.set.contains(key) && self.map.contains_key(key)
        }

        pub(crate) fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
            self.map.get(key).cloned()
        }

        pub(crate) fn put(&mut self, key: &[u8], value: Vec<u8>) {
            self.map.insert(slice_to_hash(key), value);
            self.set.remove(key);
        }

        pub(crate) fn delete(&mut self, key: &[u8]) {
            self.set.insert(slice_to_hash(key));
        }
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
    batch: cache::MemCache,
    batch_on: bool,
}

impl Database for MemoryDB {
    fn new(_dbname: &str) -> Self {
        Self {
            db: HashMap::new(),
            batch: cache::MemCache::new(),
            batch_on: false,
        }
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if self.batch_on && self.batch.contains(key) {
            return Ok(self.batch.get(key));
        }
        match self.db.get(key) {
            Some(v) => Ok(Some(v.to_owned())),
            None => Ok(None),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        if self.batch_on {
            self.batch.put(key, value);
        } else {
            self.db.insert(slice_to_hash(key), value);
        }
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        if self.batch_on {
            self.batch.delete(key);
        } else {
            self.db.remove(key);
        }
        Ok(())
    }

    fn init_batch(&mut self) -> Result<()> {
        if !self.batch_on {
            self.batch.clear();
            self.batch_on = true;
        }
        Ok(())
    }

    fn finish_batch(&mut self) -> Result<()> {
        if self.batch_on {
            for (key, value) in self.batch.map.drain() {
                self.db.insert(key, value);
            }
            for key in self.batch.set.drain() {
                self.db.remove(&key);
            }
            self.batch_on = false;
        }
        Ok(())
    }
}

#[cfg(feature = "db_rocksdb")]
pub mod rocksdb {
    use super::cache::MemCache;
    use crate::{Database, Errors, Result};
    use rocksdb::{WriteBatch, DB};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    /// A database using rust wrapper for `RocksDB`.
    pub struct RocksDB {
        db: Arc<Mutex<DB>>,
        cache: MemCache,
        batch: WriteBatch,
        batch_on: bool,
    }

    impl From<rocksdb::Error> for Errors {
        fn from(err: rocksdb::Error) -> Self {
            Errors::new(err.as_ref())
        }
    }

    impl Database for RocksDB {
        fn new(dbpath: &str) -> Self {
            let db = Arc::new(Mutex::new(
                DB::open_default(Path::new(dbpath)).expect("new: rocksdb"),
            ));
            Self {
                db,
                batch: WriteBatch::default(),
                cache: MemCache::new(),
                batch_on: false,
            }
        }

        fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
            if self.cache.contains(key) {
                return Ok(self.cache.get(key));
            }
            let db = self.db.lock().expect("get: rocksdb");
            match db.get(key)? {
                Some(value) => {
                    self.cache.put(key, value.to_owned());
                    Ok(Some(value))
                }
                None => Ok(None),
            }
        }

        fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
            self.cache.put(key, value.to_owned());
            if self.batch_on {
                self.batch.put(key, value);
            } else {
                let db = self.db.lock().expect("put: rocksdb");
                db.put(key, value)?
            }
            Ok(())
        }

        fn delete(&mut self, key: &[u8]) -> Result<()> {
            self.cache.delete(key);
            if self.batch_on {
                self.batch.delete(key);
            } else {
                let db = self.db.lock().expect("remove: rocksdb");
                db.delete(key)?
            }
            Ok(())
        }

        fn init_batch(&mut self) -> Result<()> {
            if !self.batch_on {
                self.batch = WriteBatch::default();
                self.cache.clear();
                self.batch_on = true;
            }
            Ok(())
        }

        fn finish_batch(&mut self) -> Result<()> {
            self.batch_on = false;
            if !self.batch.is_empty() {
                let batch = std::mem::take(&mut self.batch);
                let db = self.db.lock().expect("write_batch: rocksdb");
                db.write(batch)?;
            }
            Ok(())
        }
    }
}

#[cfg(feature = "db_sled")]
pub mod sled {
    use super::cache::MemCache;
    use crate::{Database, Errors, Result};

    /// A database using `Sled`, a pure-rust-implmented DB.
    pub struct Sled {
        db: sled::Db,
        cache: MemCache,
        batch: sled::Batch,
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
            let db = sled::open(dbpath).expect("new: sledDB");
            Self {
                db,
                batch: sled::Batch::default(),
                cache: MemCache::new(),
                batch_on: false,
            }
        }

        fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
            if self.cache.contains(key) {
                return Ok(self.cache.get(key));
            }
            match self.db.get(key)? {
                Some(value) => {
                    self.cache.put(key, value.to_vec());
                    Ok(Some(value.to_vec()))
                }
                None => Ok(None),
            }
        }

        fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
            self.cache.put(key, value.to_owned());
            if self.batch_on {
                self.batch.insert(key, value);
            } else {
                self.db.insert(key, value)?;
            }
            Ok(())
        }

        fn delete(&mut self, key: &[u8]) -> Result<()> {
            self.cache.delete(key);
            if self.batch_on {
                self.batch.remove(key);
            } else {
                self.db.remove(key)?;
            }
            Ok(())
        }

        fn init_batch(&mut self) -> Result<()> {
            if !self.batch_on {
                self.batch = sled::Batch::default();
                self.cache.clear();
                self.batch_on = true;
            }
            Ok(())
        }

        fn finish_batch(&mut self) -> Result<()> {
            self.batch_on = false;
            let batch = std::mem::take(&mut self.batch);
            self.db.apply_batch(batch)?;
            Ok(())
        }
    }
}
