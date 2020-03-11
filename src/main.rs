#![allow(dead_code, unused_variables, unused_imports)]
use monotree::bits::Bits;
use monotree::consts::HASH_LEN;
use monotree::database::{MemoryDB, RocksDB};
use monotree::node::{Cell, Node, Unit};
use monotree::tree::MonoTree;
use monotree::utils::*;
use monotree::*;
use starling::hash_tree::HashTree;
use starminer::database::MemoryDatabase;
use starminer::dynamic_smt::SparseMerkletrie;

const N: usize = 10000;

fn main() {
    benchmark();
    // benchmark_serde();
}

fn benchmark() {
    let pairs: Vec<(Hash, Hash)> = (0..N)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .collect();
    println!("random (key, value) pairs:  #{:?}", &pairs.len());

    let monotree = || {
        let mut tree = MonoTree::<MemoryDB>::new("memdb");
        let mut root = tree.new_tree();
        perf!(1, "MonoTree (MemDB)", {
            pairs.iter().for_each(|(key, value)| {
                root = tree.insert(root.as_ref(), key, value);
            });
        });
    };

    // https://github.com/leejw51/RustTutorial/tree/master/test_sparse_merkletrie
    let startree = || {
        let mut tree = SparseMerkletrie::new(MemoryDatabase::default());
        perf!(1, "StarTree by JW (MemDB)", {
            pairs.iter().for_each(|(key, value)| {
                tree.put(key, value);
            });
        });
    };

    // https://crates.io/crates/starling
    let merklebit = || {
        let mut tree = HashTree::<Hash, Vec<u8>>::new(256).unwrap();
        let mut root: Option<Hash> = None;
        perf!(1, "Merkle-Bit (MemDB)", {
            pairs.iter().for_each(|(key, value)| {
                root = Some(
                    tree.insert(root.as_ref(), &mut [*key], &[value.to_vec()])
                        .unwrap(),
                );
            });
        });
    };

    let monotree_rocksdb = || {
        let mut tree = MonoTree::<RocksDB>::new("testdb");
        let mut root = tree.new_tree();
        perf!(1, "MonoTree (RocksDB)", {
            pairs.iter().for_each(|(key, value)| {
                root = tree.insert(root.as_ref(), key, value);
            });
        });
    };

    //--- run-run-run
    startree();
    merklebit();
    monotree();
    // monotree_rocksdb();
    funtional_test_monotree(&pairs);
}

fn benchmark_serde() {
    let rb: Vec<Hash> = (0..10000)
        .map(|_| random_bytes(HASH_LEN))
        .map(|x| (slice_to_hash(&x).unwrap()))
        .collect();

    let handmade = || {
        perf!(1, "handmade", {
            rb.iter().for_each(|x| {
                let bits = Bits::new(&x[..]);
                let ser = bits.to_bytes().unwrap();
                let de = Bits::from_bytes(&ser);
                // assert_eq!(bits, de);
                // debug(&ser);
            })
        })
    };

    let serde = || {
        perf!(1, "serde", {
            rb.iter().for_each(|x| {
                let bits = Bits::new(&x[..]);
                let ser: Vec<u8> = bincode::serialize(&bits).unwrap();
                let de: Bits = bincode::deserialize(&ser[..]).unwrap();
                // assert_eq!(bits, de);
                // debug(&ser);
            })
        })
    };

    serde();
    handmade();
}

fn funtional_test_monotree(pairs: &[(Hash, Hash)]) {
    let mut tree = MonoTree::<MemoryDB>::new("memdb");
    // let mut tree = MonoTree::<RocksDB>::new("testdb");
    let mut root = tree.new_tree();
    pairs.iter().for_each(|(key, value)| {
        // insert a key
        root = tree.insert(root.as_ref(), key, value);

        // check if the key-value pair was correctly inserted
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), *value);

        // gen merkle proof on the key
        let proof = tree.get_merkle_proof(root.as_ref(), key).unwrap();

        // verify the proof
        assert_eq!(tree::verify_proof(root.as_ref(), value, &proof), true);
    });

    // test all of those on the final root
    pairs.iter().for_each(|(key, value)| {
        assert_eq!(tree.get(root.as_ref(), key).unwrap(), *value);
        let proof = tree.get_merkle_proof(root.as_ref(), key).unwrap();
        assert_eq!(tree::verify_proof(root.as_ref(), value, &proof), true);
    });
}
