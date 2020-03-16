#[macro_use]
extern crate criterion;
use criterion::{black_box, Criterion};

use monotree::consts::HASH_LEN;
use monotree::database::{MemoryDB, RocksDB};
use monotree::tree::Monotree;
use monotree::utils::*;
use monotree::Hash;
use monotree::*;
use std::fs;

const N: usize = 100;

fn prepare(n: usize) -> (Vec<Hash>, Vec<Hash>) {
    (0..n)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .unzip()
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchGroup");
    let (keys, leaves) = prepare(N);

    group.bench_function("monotree_hashmap", |b| {
        let mut tree = Monotree::<MemoryDB>::new("hashmap");
        let root = tree.new_tree();
        let mut keys = keys.clone();
        b.iter(|| {
            bench_monotree_hashmap(
                black_box(&mut tree),
                black_box(root),
                black_box(&mut keys),
                black_box(&leaves),
            )
        })
    });

    group.bench_function("monotree_rocksdb", |b| {
        let dbname = hex!(random_bytes(4));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = Monotree::<RocksDB>::new(&dbname);
        let root = tree.new_tree();
        let mut keys = keys.clone();
        b.iter(|| {
            bench_monotree_rocksdb(
                black_box(&mut tree),
                black_box(root),
                black_box(&mut keys),
                black_box(&leaves),
            )
        })
    });
    group.finish();
}

fn bench_monotree_hashmap(
    tree: &mut Monotree<MemoryDB>,
    root: Option<Hash>,
    keys: &mut [Hash],
    leaves: &[Hash],
) {
    keys.sort();
    tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
}

fn bench_monotree_rocksdb(
    tree: &mut Monotree<RocksDB>,
    root: Option<Hash>,
    keys: &mut [Hash],
    leaves: &[Hash],
) {
    keys.sort();
    tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
}

criterion_group!(benches, bench_group);
criterion_main!(benches);
