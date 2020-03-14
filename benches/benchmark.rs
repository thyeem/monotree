#[macro_use]
extern crate criterion;
use criterion::{black_box, Criterion};

use monotree::consts::HASH_LEN;
use monotree::database::MemoryDB;
use monotree::tree::MonoTree;
use monotree::utils::*;
use monotree::Hash;
use starling::hash_tree::HashTree;

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

    let mut tree = MonoTree::<MemoryDB>::new("memdb");
    let root = tree.new_tree();
    let monotree = (&mut tree, root);

    let mut tree = HashTree::<Hash, Vec<u8>>::new(256).unwrap();
    let root: Option<Hash> = None;
    let merklebit = (&mut tree, root);

    group.bench_function("merklebit", |b| {
        b.iter(|| {
            bench_merklebit(
                black_box(merklebit.0),
                black_box(merklebit.1),
                black_box(&pairs),
            )
        })
    });
    group.bench_function("monotree", |b| {
        b.iter(|| {
            bench_monotree(
                black_box(monotree.0),
                black_box(monotree.1),
                black_box(&pairs),
            )
        })
    });
    group.finish();
}

fn bench_monotree(
    tree: &mut MonoTree<MemoryDB>,
    mut root: Option<Hash>,
    pairs: &Vec<(Hash, Hash)>,
) {
    pairs.iter().for_each(|(key, value)| {
        root = tree.insert(root.as_ref(), key, value).unwrap();
    });
}

fn bench_merklebit(tree: &mut HashTree<Hash>, mut root: Option<Hash>, pairs: &Vec<(Hash, Hash)>) {
    pairs.iter().for_each(|(key, value)| {
        root = Some(
            tree.insert(root.as_ref(), &mut [*key], &[value.to_vec()])
                .unwrap(),
        );
    });
}

criterion_group!(benches, bench_group);
criterion_main!(benches);
