use monotree::utils::random_hash;
use monotree::{Monotree, Result};

fn main() -> Result<()> {
    // Init a monotree instance
    // by default, with 'HashMap' and 'Blake3' hash function
    let mut tree = Monotree::default();

    // It is natural the tree root initially has 'None'
    let root = None;

    // Prepare a random pair of key and leaf.
    // random_hash() gives a fixed length of random array,
    // where Hash -> [u8; HASH_LEN], HASH_LEN = 32
    let key = random_hash();
    let leaf = random_hash();

    // Insert the entry (key, leaf) into tree, yielding a new root of tree
    let root = tree.insert(root.as_ref(), &key, &leaf)?;
    assert_ne!(root, None);

    // Get the leaf inserted just before. Note that the last root was used.
    let found = tree.get(root.as_ref(), &key)?;
    assert_eq!(found, Some(leaf));

    // Remove the entry
    let root = tree.remove(root.as_ref(), &key)?;

    // surely, the tree has nothing nad the root back to 'None'
    assert_eq!(tree.get(root.as_ref(), &key)?, None);
    assert_eq!(root, None);
    Ok(())
}
