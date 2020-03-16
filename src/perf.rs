use crate::consts::HASH_LEN;
use crate::database::{MemoryDB, RocksDB};
use crate::tree::Monotree;
use crate::utils::*;
use crate::*;
use std::fs;

const N: usize = 10000;

pub fn perf() {
    let (keys, leaves): (Vec<Hash>, Vec<Hash>) = (0..N)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .unzip();
    println!("random keys/leaves vec: #{:?}", keys.len());

    let monotree_hashmap = || {
        let mut tree = Monotree::<MemoryDB>::new("hashmap");
        let root = tree.new_tree();
        let mut keys = keys.clone();
        perf!(1, "Monotree: HashMap", {
            keys.sort();
            tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
        });
    };

    let monotree_rocksdb_batch = || {
        let dbname = hex!(random_bytes(4));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = Monotree::<RocksDB>::new(&dbname);
        let mut root = tree.new_tree();
        let mut keys = keys.clone();
        perf!(1, "Monotree: RocksDB with batch", {
            keys.sort();
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
        let mut tree = Monotree::<RocksDB>::new(&dbname);
        let mut root = tree.new_tree();
        let mut keys = keys.clone();
        perf!(1, "Monotree: RocksDB without batch", {
            keys.sort();
            for (key, value) in keys.iter().zip(leaves.iter()) {
                root = tree.insert(root.as_ref(), key, value).unwrap();
            }
        });
        assert_ne!(root, None);
    };

    monotree_hashmap();
    monotree_rocksdb_batch();
    monotree_rocksdb_no_batch();
}
