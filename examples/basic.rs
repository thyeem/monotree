use monotree::utils::random_hash;
use monotree::{Monotree, Result};

fn main() -> Result<()> {
    // If no database or hash function is specified,
    // the tree defaults to using a HashMap and the Blake3 hash function.
    let mut tree = Monotree::default();

    // It is natural the tree root initially has 'None'
    let root = None;

    // Generate a random key and leaf pair.
    // random_hash() creates a fixed-length random array of 32 bytes.
    let key = random_hash();
    let leaf = random_hash();

    // Insert the entry (key, leaf) into tree, yielding a new root of tree
    let root = tree.insert(root.as_ref(), &key, &leaf)?;
    assert_ne!(root, None);

    // Retrieve the leaf inserted just before. Note that the last root was used.
    let found = tree.get(root.as_ref(), &key)?;
    assert_eq!(found, Some(leaf));

    // Remove the entry
    let root = tree.remove(root.as_ref(), &key)?;

    // The tree is empty now and the root back to 'None'
    assert_eq!(tree.get(root.as_ref(), &key)?, None);
    assert_eq!(root, None);

    // Do the same thing using batch operations
    //
    // initialize an empty batch: prepare transaction
    tree.prepare();

    // Insert the entry (key, leaf) within the batch
    let root = tree.insert(root.as_ref(), &key, &leaf)?;
    assert_ne!(root, None);

    // Retrieve the inserted leaf using the batch root
    let found = tree.get(root.as_ref(), &key)?;
    assert_eq!(found, Some(leaf));

    // Remove the entry within the same batch
    let root = tree.remove(root.as_ref(), &key)?;

    // Ensure the entry was removed within the batch
    assert_eq!(tree.get(root.as_ref(), &key)?, None);

    // Commit the batch operations: commit transaction
    tree.commit();

    // Verify that the final root is `None` after commit
    assert_eq!(tree.get(root.as_ref(), &key)?, None);
    Ok(())
}
