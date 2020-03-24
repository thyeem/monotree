use monotree::database::{MemoryDB, RocksDB};
use monotree::hasher::{Blake2b, Blake3, Sha3};
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

fn insert_keys_then_verify_values<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    _hasher: &H,
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

fn insert_keys_then_gen_and_verify_proof<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().enumerate().for_each(|(i, (key, value))| {
        // insert a key
        root = tree.insert(root.as_ref(), key, value).unwrap();
        pairs.iter().take(i + 1).for_each(|(k, v)| {
            // gen/verify Merkle proof with all keys so far
            let proof = tree.get_merkle_proof(root.as_ref(), k).unwrap().unwrap();
            assert_eq!(tree::verify_proof(hasher, root.as_ref(), v, &proof), true);
        });
    });
    assert_ne!(root, None);
}

fn insert_keys_then_delete_keys_in_order<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
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
            assert_eq!(tree::verify_proof(hasher, root.as_ref(), v, &proof), true);
        });
        // delete a key
        root = tree.remove(root.as_ref(), key).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
    });
    // back to inital state of tree
    assert_eq!(root, None);
}

fn insert_keys_then_delete_keys_reversely<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
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
            assert_eq!(tree::verify_proof(hasher, root.as_ref(), v, &proof), true);
        });
        // delete a key
        root = tree.remove(root.as_ref(), key).unwrap();
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), None);
    });
    // back to inital state of tree
    assert_eq!(root, None);
}

fn insert_keys_then_delete_keys_randomly<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    hasher: &H,
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
                tree::verify_proof(hasher, root.as_ref(), &pairs[*j].1, &proof),
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

fn insert_keys_then_delete_keys_immediately<D: Database, H: Hasher>(
    mut tree: Monotree<D, H>,
    _hasher: &H,
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
    ($fn: ident, $d: expr, $D:ident, $h: expr, $H: ident, $n:expr) => {
        item_with_macros! {
            #[test]
            fn [<test_ $d _ $h _ $fn _ $n>]() {
                let dbname = hex!(random_bytes(4));
                let _g = scopeguard::guard((), |_| {
                    if fs::metadata(&dbname).is_ok() {
                        fs::remove_dir_all(&dbname).unwrap()
                    }
                });
                let pairs = gen_random_pairs($n);
                let mut tree = Monotree::<$D, $H>::new(&dbname);
                let hasher = $H::new();
                let root = tree.new_tree();
                $fn(tree, &hasher, root, &pairs);
            }
        }
    };
}

macro_rules! impl_test_with_all_params {
    ([$($fn: tt),*], [$(($d: tt, $D: tt)),*], [$(($h: tt, $H: tt)),*], [$n: expr, $($n_: tt),*]) => {
        impl_test_with_all_params!([$($fn),*], [$(($d, $D)),*], [$(($h, $H)),*], [$n]);
        impl_test_with_all_params!([$($fn),*], [$(($d, $D)),*], [$(($h, $H)),*], [$($n_),*]);
    };

    ([$($fn: tt),*], [$(($d: tt, $D: tt)),*], [($h: expr, $H: ident), $(($h_: tt, $H_: tt)),*], [$n: expr]) => {
        impl_test_with_all_params!([$($fn),*], [$(($d, $D)),*], [($h, $H)], [$n]);
        impl_test_with_all_params!([$($fn),*], [$(($d, $D)),*], [$(($h_, $H_)),*], [$n]);
    };

    ([$($fn: tt),*], [($d: expr, $D: ident), $(($d_: tt, $D_: tt)),*], [($h: expr, $H: ident)], [$n: expr]) => {
        impl_test_with_all_params!([$($fn),*], [($d, $D)], [($h, $H)], [$n]);
        impl_test_with_all_params!([$($fn),*], [$(($d_, $D_)),*], [($h, $H)], [$n]);
    };

    ([$fn: ident, $($fn_: tt),*], [($d: expr, $D: ident)], [($h: expr, $H: ident)], [$n: expr]) => {
        impl_integration_test!($fn, $d, $D, $h, $H, $n);
        impl_test_with_all_params!([$($fn_),*], [($d, $D)], [($h, $H)], [$n]);
    };

    ([$fn: ident], [($d: expr, $D: ident)], [($h: expr, $H: ident)], [$n: expr]) => {
        impl_integration_test!($fn, $d, $D, $h, $H, $n);
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
    [("blake2b", Blake2b), ("blake3", Blake3), ("sha3", Sha3)],
    [100, 500, 1000]
);
