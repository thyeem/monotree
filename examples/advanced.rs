use monotree::database::rocksdb::RocksDB;
use monotree::hasher::*;
use monotree::utils::*;
use monotree::*;

fn main() -> Result<()> {
    // Init a monotree instance with a database and hash function
    //
    // Monotree::<DATABASE, HASHER>::new(DB_PATH)
    //      where DATABASE = {MemoryDB, RocksDB, Sled}
    //            HASHER = {Blake3, Blake2s, Blake2b, Sha2, Sha3}
    let mut tree = Monotree::<RocksDB, Blake2b>::new("/tmp/monotree");

    // It is natural the tree root initially has 'None'
    let root = None;

    // Prepare 500 random pairs of key and leaf.
    // random_hash(SIZE) creates a vector of fixed-length random array of 32 bytes.
    let n = 500;
    let mut keys = random_hashes(n);
    let leaves = random_hashes(n);

    // Insert a vector of entries of (key, leaf) into tree.
    // 'inserts()' is significantly faster than 'insert()' for the following reason.
    // (1) batch writes, (2) sorting keys prior to insertion, and (3) in-memory caching
    let root = tree.inserts(root.as_ref(), &keys, &leaves)?;
    assert_ne!(root, None);

    // Similarly, `gets()` and `removes()` also are designed for batch usage.
    let result = tree.gets(root.as_ref(), &keys)?;
    assert_eq!(result.len(), keys.len());

    let root = tree.removes(root.as_ref(), &keys)?;
    // surely, the tree has nothing and the root back to 'None'
    assert_eq!(root, None);

    /////////////////////////////////////////////////////////////////////
    // `Merkle proof` secion: verifying inclusion of data (inclusion proof)

    // `Monotree` has compressed representations, but it fully retains
    // the core property of the Sparse Merkle Tree (SMT).
    // Thus, `non-inclusion proof` is quite straightforward. Just go walk down
    // the tree with a key (or a path) given. If we cannot successfully get a leaf,
    // we can assure that the leaf is not a part of the tree.
    // The process of inclusion proof is outlined below:

    // random insertions for testing Merkle proof generation
    let root = tree.inserts(root.as_ref(), &keys, &leaves)?;

    // pick a random key from the keys among inserted just before
    let key = keys[99];

    // generate a Merkle proof for a given root and key.
    let proof = tree.get_merkle_proof(root.as_ref(), &key)?;

    // To verify the proof correctly, you need to provide a hasher matched
    // Previously the tree was initialized with `Blake2b`
    let hasher = Blake2b::new();

    // get a leaf matched with the key: where the Merkle proof verification starts off
    let leaf = leaves[99];

    // verify the Merkle proof using all those above
    let verified = verify_proof(&hasher, root.as_ref(), &leaf, proof.as_ref());
    assert_eq!(verified, true);

    /////////////////////////////////////////////////////////////////////
    // Tracking the latest root(state)
    //
    // set the latest state or root to the database
    tree.set_headroot(root.as_ref());

    // get the lastest state or root from the database
    let headroot = tree.get_headroot()?;
    assert_eq!(headroot, root);

    /////////////////////////////////////////////////////////////////////
    // Usage examples with some functional tests
    // Carefully trace the variable `root` as they are frequently shadowed.

    let mut tree = Monotree::default();
    let mut root = None;
    let hasher = Blake3::new();

    //--- insert/get and gen_proof/verify_proof over iterator
    for (i, (key, value)) in keys.iter().zip(leaves.iter()).enumerate() {
        // insert a key into tree
        root = tree.insert(root.as_ref(), key, value)?;

        // inserted a key and yields a root, where cumulative check-up goes on
        for (k, v) in keys.iter().zip(leaves.iter()).take(i + 1) {
            // check if the key-value pair was correctly inserted so far
            assert_eq!(tree.get(root.as_ref(), k)?, Some(*v));

            // generates a Merkle proof with all keys so far
            let proof = tree.get_merkle_proof(root.as_ref(), k)?;

            // verify the Merkle proof with all keys so far
            assert_eq!(
                verify_proof(&hasher, root.as_ref(), v, proof.as_ref()),
                true
            );
        }
    }
    assert_ne!(root, None);

    //--- insert/get and gen_proof/verify_proof after each deletion of entry
    for (i, (key, _)) in keys.iter().zip(leaves.iter()).enumerate() {
        assert_ne!(root, None);

        // assert that all other values are fine after each deletion
        for (k, v) in keys.iter().zip(leaves.iter()).skip(i) {
            // check in the same way as the previous example
            assert_eq!(tree.get(root.as_ref(), k)?, Some(*v));
            let proof = tree.get_merkle_proof(root.as_ref(), k)?;
            assert_eq!(
                verify_proof(&hasher, root.as_ref(), v, proof.as_ref()),
                true
            );
        }
        // delete a key and check if it was correctly removed
        root = tree.remove(root.as_ref(), key)?;
        assert_eq!(tree.get(root.as_ref(), key)?, None);
    }
    // must be back to inital state of tree
    assert_eq!(root, None);

    //--- faster way to insert/remove entries
    // Now tree is empty, and root is back to `None` again
    // Redo all those above using methods supporting batch operations
    root = tree.inserts(root.as_ref(), &keys, &leaves)?;
    assert_ne!(root, None);

    // Even if we shuffle the keys when removing,
    shuffle(&mut keys);

    // there's no difference. Back to `None` of root and empty tree again.
    // that's why the `root` plays a role as _"state index of tree"_
    root = tree.removes(root.as_ref(), &keys)?;
    assert_eq!(root, None);

    Ok(())
}
