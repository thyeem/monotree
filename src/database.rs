use crate::{Database, Errors, Result};
use hashbrown::{HashMap, HashSet};
use rocksdb::{Options, WriteBatch, DB};
use std::mem;
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

    fn close(&mut self) -> Result<()> {
        self.db.clear();
        Ok(())
    }

    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>> {
        self.db.get(key).map_or(Ok(vec![]), |r| Ok(r.to_vec()))
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.db.insert(key.to_vec(), value);
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> Result<()> {
        self.db.remove(key);
        Ok(())
    }

    fn init_batch(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_batch(&mut self) -> Result<()> {
        Ok(())
    }
}

impl From<rocksdb::Error> for Errors {
    fn from(err: rocksdb::Error) -> Self {
        Errors::new(&err.to_string())
    }
}

pub struct RocksDB {
    db: Arc<Mutex<DB>>,
    dbpath: String,
    batch: WriteBatch,
    meta: HashSet<Vec<u8>>,
    cache: HashMap<Vec<u8>, Vec<u8>>,
    use_batch: bool,
}

impl Database for RocksDB {
    fn new(dbpath: &str) -> Self {
        let path = Path::new(dbpath);
        let db = Arc::new(Mutex::new(DB::open_default(path).expect("new(): rocksdb")));
        RocksDB {
            db,
            dbpath: dbpath.to_string(),
            batch: WriteBatch::default(),
            meta: HashSet::new(),
            cache: HashMap::new(),
            use_batch: false,
        }
    }

    fn close(&mut self) -> Result<()> {
        DB::destroy(&Options::default(), &self.dbpath)
            .map_err(|_| Errors::new("Error: rocksdb.close()"))
    }

    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>> {
        if self.use_batch && self.meta.contains(key) || self.cache.contains_key(key) {
            return self.cache.get(key).map_or(Ok(vec![]), |r| Ok(r.to_vec()));
        }
        let db = self.db.lock().expect("get(): rocksdb");
        match db.get(key) {
            Ok(Some(value)) => {
                if self.use_batch {
                    self.cache.insert(key.to_owned(), value.to_owned());
                }
                Ok(value.to_vec())
            }
            Ok(None) => Ok(vec![]),
            Err(_) => Err(Errors::new("Erorr: rocksdb.get()")),
        }
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        if self.use_batch {
            self.cache.insert(key.to_owned(), value.to_owned());
            self.batch
                .put(key, value)
                .map_err(|_| Errors::new("Error: rocksdb.batch.put()"))
        } else {
            let db = self.db.lock().expect("put(): rocksdb");
            db.put(key, value)
                .map_err(|_| Errors::new("Error: rocksdb.put()"))
        }
    }

    fn remove(&mut self, key: &[u8]) -> Result<()> {
        if self.use_batch {
            if self.cache.contains_key(key) {
                self.cache.remove(key);
            }
            self.meta.insert(key.to_owned());
            self.batch
                .delete(key)
                .map_err(|_| Errors::new("Error: rocksdb.batch.delete()"))
        } else {
            let db = self.db.lock().expect("remove(): rocksdb");
            db.delete(key)
                .map_err(|_| Errors::new("Error: rocksdb.delete()"))
        }
    }

    fn init_batch(&mut self) -> Result<()> {
        self.batch = WriteBatch::default();
        self.meta.clear();
        self.cache.clear();
        self.use_batch = true;
        Ok(())
    }

    fn write_batch(&mut self) -> Result<()> {
        self.use_batch = false;
        if !self.batch.is_empty() {
            let batch = mem::replace(&mut self.batch, WriteBatch::default());
            let db = self.db.lock().expect("write_batch(): rocksdb");
            db.write(batch)?
        }
        Ok(())
    }
}
