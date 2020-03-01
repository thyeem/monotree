use failure::Error;
pub trait MerkletrieDatabase {
    fn compute_hash(&self, data: &[u8]) -> Vec<u8>;
    fn write(&mut self, key: &[u8], data: &[u8]) -> Result<(), Error>;
    fn read(&self, key: &[u8]) -> Result<Vec<u8>, Error>;
}

pub trait MerkletrieInterface {
    fn load(&mut self, hash: &[u8]) -> Result<(), Error>;
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn get(&mut self, key: &[u8]) -> Result<Vec<u8>, Error>;
    fn get_roothash(&self) -> Result<Vec<u8>, Error>;
}
