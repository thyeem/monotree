use crate::{Bytes, BytesResult, Database, VoidResult};
use hashbrown::HashMap;

#[derive(Debug)]
pub struct MemoryDB {
    db: HashMap<Bytes, Bytes>,
    dbname: String,
}

impl Database for MemoryDB {
    fn new(dbname: &str) -> MemoryDB {
        MemoryDB {
            db: HashMap::new(),
            dbname: dbname.to_string(),
        }
    }

    fn get(&self, k: &[u8]) -> BytesResult {
        self.db.get(k).map_or(Ok(vec![]), |r| Ok(r.to_vec()))
    }

    fn put(&mut self, k: &[u8], v: Vec<u8>) -> VoidResult {
        self.db.insert(k.to_vec(), v);
        Ok(())
    }
}

#[derive(Debug)]
pub struct RocksDB {}
