#[macro_use]
extern crate criterion;
use criterion::{black_box, Criterion};

use monotree::consts::HASH_LEN;
use monotree::database::{MemoryDB, RocksDB};
use monotree::tree::MonoTree;
use monotree::utils::*;
use monotree::Hash;
use monotree::*;
use starling::hash_tree::HashTree;
use std::fs;

const N: usize = 100;

fn prepare(n: usize) -> Vec<(Hash, Hash)> {
    (0..n)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .collect()
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchGroup");
    let pairs = prepare(N);

    group.bench_function("merklebit_hashmap", |b| {
        let mut tree = HashTree::<Hash, Vec<u8>>::new(256).unwrap();
        let root: Option<Hash> = None;
        b.iter(|| bench_merklebit_hashmap(black_box(&mut tree), black_box(root), black_box(&pairs)))
    });

    group.bench_function("monotree_hashmap", |b| {
        let mut tree = MonoTree::<MemoryDB>::new("hashmap");
        let root = tree.new_tree();
        b.iter(|| bench_monotree_hashmap(black_box(&mut tree), black_box(root), black_box(&pairs)))
    });

    group.bench_function("monotree_rocksdb", |b| {
        let dbname = hex!(random_bytes(4));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = MonoTree::<RocksDB>::new(&dbname);
        let root = tree.new_tree();
        let (keys, leaves): (Vec<Hash>, Vec<Hash>) = pairs.iter().cloned().unzip();
        b.iter(|| {
            bench_monotree_rocksdb(
                black_box(&mut tree),
                black_box(root),
                black_box(&keys),
                black_box(&leaves),
            )
        })
    });
    group.finish();
}

fn bench_monotree_hashmap(
    tree: &mut MonoTree<MemoryDB>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
    });
}

fn bench_monotree_rocksdb(
    tree: &mut MonoTree<RocksDB>,
    root: Option<Hash>,
    keys: &[Hash],
    leaves: &[Hash],
) {
    tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
}

fn bench_merklebit_hashmap(
    tree: &mut HashTree<Hash>,
    mut root: Option<Hash>,
    pairs: &[(Hash, Hash)],
) {
    pairs.iter().for_each(|(key, value)| {
        root = Some(
            tree.insert(root.as_ref(), &mut [*key], &[value.to_vec()])
                .unwrap(),
        );
    });
}

criterion_group!(benches, bench_group);
criterion_main!(benches);
