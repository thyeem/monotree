use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, Errors>;
pub type Bytes = Vec<u8>;
pub type Bits = Vec<bool>;
pub type BytesTuple = (Bytes, Bytes);
pub type Proof = Vec<BytesTuple>;
pub type VoidResult = Result<()>;
pub type BitsResult = Result<Bits>;
pub type BytesResult = Result<Bytes>;
pub type ParseResult = Result<(Bytes, Bits, usize)>;
pub type ParseSoftResult = Result<(Bytes, Bits)>;
pub type ParseHardResult = Result<(Bytes, Bits, Bytes, Bits)>;
pub type ProofResult = Result<Proof>;

#[derive(Debug)]
pub enum Node {
    Soft,
    Hard,
}

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
    fn get(&self, k: &[u8]) -> BytesResult;
    fn put(&mut self, k: &[u8], v: Vec<u8>) -> VoidResult;
}

#[macro_use]
pub mod utils;
pub mod database;
pub mod tree;
