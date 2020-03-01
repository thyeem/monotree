use simple_trie::database::Database;
use simple_trie::dynamic_smt::SparseMerkletrie;
use simple_trie::merkletrie_interface::MerkletrieDatabase;
use std::time::Instant;

pub fn main() {
    let database = Database::new("./data");
    let mut smt = SparseMerkletrie::new(database.clone());
    let n = 10;
    let now = Instant::now();
    for i in 0..n {
        let b = i as i32;
        let value = b.to_le_bytes();
        let key = database.compute_hash(&value);
        smt.put(&key, &value);
    }
    println!("sparse merkletrie= {}", now.elapsed().as_millis());
}
