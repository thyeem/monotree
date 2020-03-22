use monotree::consts::HASH_LEN;
use monotree::database::{MemoryDB, RocksDB};
use monotree::tree::Monotree;
use monotree::utils::*;
use monotree::*;
use std::fs;

#[macro_use]
extern crate paste;
extern crate scopeguard;

fn gen_random_pairs(n: usize) -> Vec<(Hash, Hash)> {
    (0..n)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .collect()
}

fn insert_keys_then_verify_values<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().enumerate().for_each(|(i, (key, value))| {
        // insert a key
        root = tree.insert(root.as_ref(), key, value).unwrap();
        pairs.iter().take(i + 1).for_each(|(k, v)| {
            // check if the key-value pair was correctly inserted so far
            assert_eq!(tree.get(root.as_ref(), k).unwrap(), Some(*v));
        });
    });
    assert_ne!(root, None);
}

fn insert_keys_then_gen_and_verify_proof<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().enumerate().for_each(|(i, (key, value))| {
        // insert a key
        root = tree.insert(root.as_ref(), key, value).unwrap();
        pairs.iter().take(i + 1).for_each(|(k, v)| {
            // gen/verify Merkle proof with all keys so far
            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
            assert_eq!(tree::verify_proof(root.as_ref(), v, &proof), true);
        });
    });
    assert_ne!(root, None);
}

fn insert_keys_then_delete_keys_in_order<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
    });
    //test with keys in order
    pairs.iter().enumerate().for_each(|(i, (key, _))| {
        assert_ne!(root, None);
        // assert that all values are fine after deletion
        pairs.iter().skip(i).for_each(|(k, v)| {
            assert_eq!(tree.get(root.as_ref(), k).unwrap(), Some(*v));
            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
            assert_eq!(tree::verify_proof(root.as_ref(), v, &proof), true);
        });
        // delete a key
        root = tree.remove(root.as_ref(), key).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
    });
    // back to inital state of tree
    assert_eq!(root, None);
}

fn insert_keys_then_delete_keys_reversely<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
    });
    //test with keys in reverse order
    pairs.iter().rev().enumerate().for_each(|(i, (key, _))| {
        assert_ne!(root, None);
        // assert that all values are fine after deletion
        pairs.iter().rev().skip(i).for_each(|(k, v)| {
            assert_eq!(tree.get(root.as_ref(), k).unwrap(), Some(*v));
            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
            assert_eq!(tree::verify_proof(root.as_ref(), v, &proof), true);
        });
        // delete a key
        root = tree.remove(root.as_ref(), key).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
    });
    // back to inital state of tree
    assert_eq!(root, None);
}

fn insert_keys_then_delete_keys_randomly<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
    });

    // shuffles pairs' index for imitating random-access
    let mut idx: Vec<usize> = (0..pairs.len()).collect();
    shuffle(&mut idx);

    //test with shuffled keys
    idx.iter().enumerate().for_each(|(n, i)| {
        assert_ne!(root, None);
        // assert that all values are fine after deletion
        idx.iter().skip(n).for_each(|j| {
            assert_eq!(
                tree.get(root.as_ref(), &pairs[*j].0).unwrap(),
                Some(pairs[*j].1)
            );
            let proof = tree
                .get_merkle_proof(root.as_ref(), &pairs[*j].0)
                .unwrap()
                .unwrap();
            assert_eq!(
                tree::verify_proof(root.as_ref(), &pairs[*j].1, &proof),
                true
            );
        });
        // delete a key by random index
        root = tree.remove(root.as_ref(), &pairs[*i].0).unwrap();
        assert_eq!(tree.get(root.as_ref(), &pairs[*i].1).unwrap(), None);
    });
    // back to inital state of tree
    assert_eq!(root, None);
}

fn insert_keys_then_delete_keys_immediately<D: Database>(
    mut tree: Monotree<D>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), Some(*value));
        root = tree.remove(root.as_ref(), key).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
        assert_eq!(root, None);
    });
}

macro_rules! impl_integration_test {
    ($fn: ident, $db: expr, $DB:ident, $n:expr) => {
        item_with_macros! {
            #[test]
            fn [<test_ $db _ $fn _ $n>]() {
                let dbname = hex!(random_bytes(4));
                let _g = scopeguard::guard((), |_| {
                    if fs::metadata(&dbname).is_ok() {
                        fs::remove_dir_all(&dbname).unwrap()
                    }
                });
                let pairs = gen_random_pairs($n);
                let mut tree = Monotree::<$DB>::new(&dbname);
                let root = tree.new_tree();
                $fn(tree, root, &pairs);
            }
        }
    };
}

macro_rules! impl_test_with_all_params {
    ([$($fn: tt),*], [$(($db: tt, $DB: tt)),*], [$n: expr, $($n_: tt),*]) => {
        impl_test_with_all_params!([$($fn),*], [$(($db, $DB)),*], [$n]);
        impl_test_with_all_params!([$($fn),*], [$(($db, $DB)),*], [$($n_),*]);
    };


    ([$($fn: tt),*], [($db: expr, $DB: ident), $(($db_: tt, $DB_: tt)),*], [$n: expr]) => {
        impl_test_with_all_params!([$($fn),*], [($db, $DB)], [$n]);
        impl_test_with_all_params!([$($fn),*], [$(($db_, $DB_)),*], [$n]);
    };


    ([$fn: ident, $($fn_: tt),*], [($db: expr, $DB: ident)], [$n: expr]) => {
        impl_integration_test!($fn, $db, $DB, $n);
        impl_test_with_all_params!([$($fn_),*], [($db, $DB)], [$n]);
    };

    ([$fn: ident], [($db: expr, $DB: ident)], [$n: expr]) => {
        impl_integration_test!($fn, $db, $DB, $n);
    };

    ($($other:tt)*) => {};
}

impl_test_with_all_params!(
    [
        insert_keys_then_verify_values,
        insert_keys_then_gen_and_verify_proof,
        insert_keys_then_delete_keys_immediately,
        insert_keys_then_delete_keys_in_order,
        insert_keys_then_delete_keys_reversely,
        insert_keys_then_delete_keys_randomly
    ],
    [("hashmap", MemoryDB), ("rocksdb", RocksDB)],
    [100, 500, 1000]
);
