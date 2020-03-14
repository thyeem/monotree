use crate::consts::HASH_LEN;
use crate::database::{MemoryDB, RocksDB};
use crate::tree::MonoTree;
use crate::utils::*;
use crate::*;
use starling::hash_tree::HashTree;
use std::fs;

const N: usize = 50000;

// merklebit item will be eraseda when released
pub fn perf() {
    let pairs: Vec<(Hash, Hash)> = (0..N)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .collect();
    println!("random (key, value) pairs:  #{:?}", &pairs.len());

    let monotree_hashmap = || {
        let mut tree = MonoTree::<MemoryDB>::new("hashmap");
        let mut root = tree.new_tree();
        perf!(1, "MonoTree: HashMap", {
            pairs.iter().for_each(|(key, value)| {
                root = tree.insert(root.as_ref(), key, value).unwrap();
            });
        });
    };

    // https://crates.io/crates/starling
    let merklebit_hashmap = || {
        let mut tree = HashTree::<Hash, Vec<u8>>::new(256).unwrap();
        let mut root: Option<Hash> = None;
        perf!(1, "Merkle-Bit: HashMap", {
            pairs.iter().for_each(|(key, value)| {
                root = Some(
                    tree.insert(root.as_ref(), &mut [*key], &[value.to_vec()])
                        .unwrap(),
                );
            });
        });
    };

    let monotree_rocksdb_batch = || {
        let dbname = hex!(random_bytes(4));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = MonoTree::<RocksDB>::new(&dbname);
        let mut root = tree.new_tree();
        let (keys, leaves): (Vec<Hash>, Vec<Hash>) = pairs.iter().cloned().unzip();
        perf!(1, "MonoTree: RocksDB with batch", {
            root = tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
        });
        assert_ne!(root, None);
    };

    let monotree_rocksdb_no_batch = || {
        let dbname = hex!(random_bytes(4));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = MonoTree::<RocksDB>::new(&dbname);
        let mut root = tree.new_tree();
        perf!(1, "MonoTree: RocksDB without batch", {
            pairs.iter().for_each(|(key, value)| {
                root = tree.insert(root.as_ref(), key, value).unwrap();
            });
        });
        assert_ne!(root, None);
    };

    merklebit_hashmap();
    monotree_hashmap();
    monotree_rocksdb_batch();
    monotree_rocksdb_no_batch();
}
