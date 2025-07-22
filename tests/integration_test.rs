use monotree::database::{rocksdb::RocksDB, sled::Sled, MemoryDB};
use monotree::hasher::*;
use monotree::utils::*;
use monotree::*;
use rand::{random, Rng};
use std::fs;
extern crate paste;
extern crate scopeguard;

fn insert_keys_then_verify_values<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    _hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    for (i, (key, value)) in keys.iter().zip(leaves.iter()).enumerate() {
        // insert a key into tree
        root = tree.insert(root.as_ref(), key, value)?;

        // check if the key-value pair was correctly inserted so far
        for (k, v) in keys.iter().zip(leaves.iter()).take(i + 1) {
            assert_eq!(tree.get(root.as_ref(), k)?, Some(*v));
        }
    }
    assert_ne!(root, None);
    Ok(())
}

fn insert_keys_then_gen_and_verify_proof<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    for (i, (key, value)) in keys.iter().zip(leaves.iter()).enumerate() {
        // insert a key into tree
        root = tree.insert(root.as_ref(), key, value)?;

        // generate and verify Merkle proof with all keys so far
        for (k, v) in keys.iter().zip(leaves.iter()).take(i + 1) {
            let proof = tree.get_merkle_proof(root.as_ref(), k)?;
            assert_eq!(
                tree::verify_proof(hasher, root.as_ref(), v, proof.as_ref()),
                true
            );
        }
    }
    assert_ne!(root, None);
    Ok(())
}

fn insert_keys_then_delete_keys_in_order<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    // pre-insertion for removal test
    root = tree.inserts(root.as_ref(), keys, leaves)?;

    // removal test with keys in order
    for (i, (key, _)) in keys.iter().zip(leaves.iter()).enumerate() {
        assert_ne!(root, None);
        // assert that all other values are fine after deletion
        for (k, v) in keys.iter().zip(leaves.iter()).skip(i) {
            assert_eq!(tree.get(root.as_ref(), k)?, Some(*v));
            let proof = tree.get_merkle_proof(root.as_ref(), k)?;
            assert_eq!(
                tree::verify_proof(hasher, root.as_ref(), v, proof.as_ref()),
                true
            );
        }

        // delete a key and check if it worked
        root = tree.remove(root.as_ref(), key)?;
        assert_eq!(tree.get(root.as_ref(), key)?, None);
    }
    // back to inital state of tree
    assert_eq!(root, None);
    Ok(())
}

fn insert_keys_then_delete_keys_reversely<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    // pre-insertion for removal test
    root = tree.inserts(root.as_ref(), keys, leaves)?;

    // removal test with keys in reverse order
    for (i, (key, _)) in keys.iter().zip(leaves.iter()).rev().enumerate() {
        assert_ne!(root, None);

        // assert that all other values are fine after deletion
        for (k, v) in keys.iter().zip(leaves.iter()).rev().skip(i) {
            assert_eq!(tree.get(root.as_ref(), k)?, Some(*v));
            let proof = tree.get_merkle_proof(root.as_ref(), k)?;
            assert_eq!(
                tree::verify_proof(hasher, root.as_ref(), v, proof.as_ref()),
                true
            );
        }

        // delete a key and check if it worked
        root = tree.remove(root.as_ref(), key)?;
        assert_eq!(tree.get(root.as_ref(), key)?, None);
    }
    // back to inital state of tree
    assert_eq!(root, None);
    Ok(())
}

fn insert_keys_then_delete_keys_randomly<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    // pre-insertion for removal test
    root = tree.inserts(root.as_ref(), keys, leaves)?;

    // shuffles keys/leaves' index for imitating random-access
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    shuffle(&mut idx);

    //test with shuffled keys
    for (n, i) in idx.iter().enumerate() {
        assert_ne!(root, None);

        // assert that all values are fine after deletion
        for j in idx.iter().skip(n) {
            assert_eq!(tree.get(root.as_ref(), &keys[*j])?, Some(leaves[*j]));
            let proof = tree.get_merkle_proof(root.as_ref(), &keys[*j])?;
            assert_eq!(
                tree::verify_proof(hasher, root.as_ref(), &leaves[*j], proof.as_ref()),
                true
            );
        }
        // delete a key by random index and check if it worked
        root = tree.remove(root.as_ref(), &keys[*i])?;
        assert_eq!(tree.get(root.as_ref(), &leaves[*i])?, None);
    }
    // back to inital state of tree
    assert_eq!(root, None);
    Ok(())
}

fn insert_keys_then_delete_keys_immediately<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    _hasher: &H,
    mut root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    for (key, value) in keys.iter().zip(leaves.iter()) {
        // insert a key into tree
        root = tree.insert(root.as_ref(), key, value)?;
        // check if the key-value pair was correctly inserted
        assert_eq!(tree.get(root.as_ref(), key)?, Some(*value));

        // delete the key inserted just before
        root = tree.remove(root.as_ref(), key)?;
        // check if the key-value pair was correctly deleted
        assert_eq!(tree.get(root.as_ref(), key)?, None);
        // must be inital state of tree
        assert_eq!(root, None);
    }
    Ok(())
}

fn deterministic_ordering<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) -> Result<()> {
    // Create a second independent tree instance
    let mut tree1 = Monotree::<D, H>::new(&format!(".tmp/{}", hex!(random_bytes(4))));
    let mut tree2 = Monotree::<D, H>::new(&format!(".tmp/{}", hex!(random_bytes(4))));

    // Insert in normal order
    let mut root1 = root.clone();
    root1 = tree1.inserts(root1.as_ref(), keys, leaves)?;

    // Insert in reverse order
    let mut root2 = root.clone();
    let reversed_keys: Vec<Hash> = keys.iter().rev().cloned().collect();
    let reversed_leaves: Vec<Hash> = leaves.iter().rev().cloned().collect();
    root2 = tree2.inserts(root2.as_ref(), &reversed_keys, &reversed_leaves)?;

    // Verify roots match
    assert_eq!(root1, root2, "Root hashes differ for same dataset");
    // // // Verify removal consistency
    for key in keys {
        root1 = tree1.remove(root1.as_ref(), key)?;
        root2 = tree2.remove(root2.as_ref(), key)?;
        assert_eq!(root1, root2, "Root hashes differ after deletion");
    }
    Ok(())
}

macro_rules! impl_integration_test {
    ($fn:ident, ($d:expr, $db:ident), ($h:expr, $hasher:ident), $n:expr) => {
        paste::item! {
            #[test]
            fn [<test_ $d _ $h _ $fn _ $n>]() -> Result<()> {
                let dbname = format!(".tmp/{}", hex!(random_bytes(4)));
                let _g = scopeguard::guard((), |_| {
                    if fs::metadata(&dbname).is_ok() {
                        fs::remove_dir_all(&dbname).unwrap()
                    }
                });
                let keys = random_hashes($n);
                let leaves = random_hashes($n);
                let tree = Monotree::<$db, $hasher>::new(&dbname);
                let hasher = $hasher::new();
                let root: Option<Hash> = None;
                $fn(tree, &hasher, root, &keys, &leaves)?;
                Ok(())
            }
        }
    };
}

macro_rules! impl_test_with_params {
    ([$($fn:tt)+], [$($db:tt)+], [$($hasher:tt)+], [$n:tt, $($ns:tt),*]) => {
        impl_test_with_params!([$($fn)+], [$($db)+], [$($hasher)+], [$n]);
        impl_test_with_params!([$($fn)+], [$($db)+], [$($hasher)+], [$($ns),*]);
    };

    ([$($fn:tt)+], [$($db:tt)+], [($($h:tt)+), $($hasher:tt),*], [$n:tt]) => {
        impl_test_with_params!([$($fn)+], [$($db)+], [($($h)+)], [$n]);
        impl_test_with_params!([$($fn)+], [$($db)+], [$($hasher),*], [$n]);
    };

    ([$($fn:tt)+], [($($d:tt)+), $($db:tt),*], [($($h:tt)+)], [$n:tt]) => {
        impl_test_with_params!([$($fn)+], [($($d)+)], [($($h)+)], [$n]);
        impl_test_with_params!([$($fn)+], [$($db),*], [($($h)+)], [$n]);
    };

    ([$f:tt, $($fn:tt),*], [($($d:tt)+)], [($($h:tt)+)], [$n:tt]) => {
        impl_integration_test!($f, ($($d)+), ($($h)+), $n);
        impl_test_with_params!([$($fn),*], [($($d)+)], [($($h)+)], [$n]);
    };

    ([$f:tt], [($($d:tt)+)], [($($h:tt)+)], [$n:tt]) => {
        impl_integration_test!($f, ($($d)+), ($($h)+), $n);
    };

    ($($other:tt)*) => {};
}

impl_test_with_params!(
    [
        insert_keys_then_verify_values,
        insert_keys_then_gen_and_verify_proof,
        insert_keys_then_delete_keys_immediately,
        insert_keys_then_delete_keys_in_order,
        insert_keys_then_delete_keys_reversely,
        insert_keys_then_delete_keys_randomly,
        deterministic_ordering
    ],
    [("hashmap", MemoryDB), ("rocksdb", RocksDB), ("sled", Sled)],
    [
        ("blake3", Blake3),
        ("blake2s", Blake2s),
        ("blake2b", Blake2b),
        ("sha2", Sha2),
        ("sha3", Sha3)
    ],
    [100, 500, 1000]
);
