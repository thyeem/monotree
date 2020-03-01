use super::database::MemoryDatabase;
use super::merkletrie_interface::MerkletrieDatabase;
use starling::constants::KEY_LEN;
use starling::hash_tree::HashTree;
use starling::merkle_bit::BinaryMerkleTreeResult;
use std::time::Instant;

pub fn starling_main() -> BinaryMerkleTreeResult<()> {
    let mut tree: HashTree<[u8; KEY_LEN], Vec<u8>> = HashTree::new(256)?;

    let database = MemoryDatabase::default();

    let n = 20000;
    let now = Instant::now();
    let mut root: Option<&[u8; KEY_LEN]> = None;
    let mut root_value: [u8; KEY_LEN];
    for i in 0..n {
        let b = i as i32;
        let value2 = b.to_le_bytes();
        let key2 = database.compute_hash(&value2);

        //smt.put(&key, &value, &mut output);
        let mut key = [0x00; KEY_LEN];

        let mut value = vec![0x00; 4];
        key.copy_from_slice(&key2[0..KEY_LEN]);
        value.copy_from_slice(&value2[0..4]);
        // Inserting and getting from a tree
        let new_root = tree.insert(root, &mut [key], &[value.clone()]).unwrap();
        root_value = new_root;
        root = Some(&root_value);
    }
    println!("hashtree merkletrie= {}", now.elapsed().as_millis());

    Ok(())
}
