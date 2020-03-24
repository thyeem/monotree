#[macro_use]
extern crate criterion;
#[macro_use]
extern crate paste;
extern crate scopeguard;
use criterion::{black_box, Criterion};
use monotree::database::{MemoryDB, RocksDB};
use monotree::hasher::{Blake2b, Blake3};
use monotree::tree::Monotree;
use monotree::utils::*;
use monotree::Hash;
use monotree::*;
use std::fs;

fn prepare(n: usize) -> (Vec<Hash>, Vec<Hash>) {
    (0..n)
        .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
        .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
        .unzip()
}

fn insert<D: Database, H: Hasher>(
    tree: &mut Monotree<D, H>,
    root: Option<Hash>,
    keys: &mut [Hash],
    leaves: &[Hash],
) -> Option<Hash> {
    keys.sort();
    tree.inserts(root.as_ref(), &keys, &leaves).unwrap()
}

fn get<D: Database, H: Hasher>(tree: &mut Monotree<D, H>, root: Option<Hash>, keys: &[Hash]) {
    keys.iter().for_each(|key| {
        tree.get(root.as_ref(), key).unwrap();
    });
}

fn remove<D: Database, H: Hasher>(tree: &mut Monotree<D, H>, root: Option<Hash>, keys: &[Hash]) {
    tree.removes(root.as_ref(), keys).unwrap();
}

macro_rules! impl_bench_group {
    ($n: expr) => {
        item_with_macros! {
            fn [<bench_group _ $n>](c: &mut Criterion) {
                let mut group = c.benchmark_group(format!("EntryNum_{}", stringify!($n)));
                let (keys, leaves) = prepare($n);

                group.bench_function("hashmap_insert", |b| {
                    let mut tree = Monotree::<MemoryDB, Blake2b>::new("hashmap");
                    let root = tree.new_tree();
                    let mut keys = keys.clone();
                    b.iter(|| {
                        insert(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&mut keys),
                            black_box(&leaves),
                        )
                    })
                });

                group.bench_function("rocksdb_insert", |b| {
                    let dbname = hex!(random_bytes(4));
                    let _g = scopeguard::guard((), |_| {
                        if fs::metadata(&dbname).is_ok() {
                            fs::remove_dir_all(&dbname).unwrap()
                        }
                    });
                    let mut tree = Monotree::<RocksDB, Blake3>::new(&dbname);
                    let root = tree.new_tree();
                    let mut keys = keys.clone();
                    b.iter(|| {
                        insert(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&mut keys),
                            black_box(&leaves),
                        )
                    })
                });

                group.bench_function("hashmap_get", |b| {
                    let mut tree = Monotree::<MemoryDB, Blake3>::new("hashmap");
                    let mut root = tree.new_tree();
                    let mut keys = keys.clone();
                    root = insert(&mut tree, root, &mut keys, &leaves);
                    b.iter(|| {
                        get(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&keys),
                        )
                    })
                });

                group.bench_function("rocksdb_get", |b| {
                    let dbname = hex!(random_bytes(4));
                    let _g = scopeguard::guard((), |_| {
                        if fs::metadata(&dbname).is_ok() {
                            fs::remove_dir_all(&dbname).unwrap()
                        }
                    });
                    let mut tree = Monotree::<RocksDB, Blake3>::new(&dbname);
                    let mut root = tree.new_tree();
                    let mut keys = keys.clone();
                    root = insert(&mut tree, root, &mut keys, &leaves);
                    b.iter(|| {
                        get(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&keys),
                        )
                    })
                });

                group.bench_function("hashmap_remove", |b| {
                    let mut tree = Monotree::<MemoryDB, Blake3>::new("hashmap");
                    let mut root = tree.new_tree();
                    let mut keys = keys.clone();
                    root = insert(&mut tree, root, &mut keys, &leaves);
                    keys.sort_unstable_by(|a, b| b.cmp(a));
                    b.iter(|| {
                        remove(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&keys),
                        )
                    })
                });

                group.bench_function("rocksdb_remove", |b| {
                    let dbname = hex!(random_bytes(4));
                    let _g = scopeguard::guard((), |_| {
                        if fs::metadata(&dbname).is_ok() {
                            fs::remove_dir_all(&dbname).unwrap()
                        }
                    });
                    let mut tree = Monotree::<RocksDB, Blake3>::new(&dbname);
                    let mut root = tree.new_tree();
                    let mut keys = keys.clone();
                    root = insert(&mut tree, root, &mut keys, &leaves);
                    keys.sort_unstable_by(|a, b| b.cmp(a));
                    b.iter(|| {
                        remove(
                            black_box(&mut tree),
                            black_box(root),
                            black_box(&keys),
                        )
                    })
                });

                group.finish();
            }
        }
    };
}

impl_bench_group!(10);
impl_bench_group!(100);
impl_bench_group!(1000);
impl_bench_group!(10000);

criterion_group!(
    benches,
    bench_group_10,
    bench_group_100,
    bench_group_1000,
    bench_group_10000
);
criterion_main!(benches);
