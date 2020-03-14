extern crate rocksdb;
use crate::consts::HASH_LEN;
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, Errors>;
pub type Hash = [u8; HASH_LEN];
pub type Proof = Vec<(bool, Vec<u8>)>;

#[derive(Debug)]
pub struct Errors {
    details: String,
}

impl Errors {
    pub fn new(msg: &str) -> Errors {
        Errors {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for Errors {
    fn description(&self) -> &str {
        &self.details
    }
}

pub trait Database {
    fn new(dbpath: &str) -> Self;
    fn close(&mut self) -> Result<()>;
    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>>;
    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;
    fn remove(&mut self, key: &[u8]) -> Result<()>;
    fn init_batch(&mut self) -> Result<()>;
    fn write_batch(&mut self) -> Result<()>;
}

pub mod consts;
#[macro_use]
pub mod utils;
pub mod bits;
pub mod database;
pub mod node;
pub mod perf;
pub mod tree;
