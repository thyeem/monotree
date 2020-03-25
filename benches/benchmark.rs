use criterion::{black_box, criterion_group, criterion_main, Criterion};
use monotree::database::*;
use monotree::hasher::*;
use monotree::utils::*;
use monotree::*;
use std::fs;

fn insert<D: Database, H: Hasher>(
    tree: &mut Monotree<D, H>,
    root: Option<Hash>,
    keys: &mut [Hash],
    leaves: &[Hash],
) -> Option<Hash> {
    tree.inserts(root.as_ref(), keys, &leaves).expect("insert")
}

fn get<D: Database, H: Hasher>(
    tree: &mut Monotree<D, H>,
    root: Option<Hash>,
    keys: &mut [Hash],
    _leaves: &[Hash],
) {
    keys.iter().for_each(|key| {
        tree.get(root.as_ref(), key).expect("get");
    });
}

fn remove<D: Database, H: Hasher>(
    tree: &mut Monotree<D, H>,
    root: Option<Hash>,
    keys: &mut [Hash],
    _leaves: &[Hash],
) {
    tree.removes(root.as_ref(), keys).expect("remove");
}

macro_rules! impl_bench_group {
    ($n:expr) => {
        paste::item_with_macros! {
            fn [<bench_group_ $n>](c: &mut Criterion) {
                let mut group = c.benchmark_group(format!("entry_num_{}", stringify!($n)));
                let mut keys = random_hashes($n);
                let leaves = random_hashes($n);

                impl_params_bench!(
                    {group, &mut keys, &leaves},
                    [("hashmap", MemoryDB), ("rocksdb", RocksDB), ("sled", Sled)],
                    [
                        ("blake3", Blake3),
                        ("blake2s", Blake2s),
                        ("blake2b", Blake2b),
                        ("sha2", Sha2),
                        ("sha3", Sha3)
                    ],
                    [insert, get, remove]
                );
                group.finish();
            }
        }
    };
}

macro_rules! impl_params_bench {
    ({$($g:tt)+}, [$($db:tt)+], [$($hasher:tt)+], [$f:tt, $($fn:tt),*]) => {
        impl_params_bench!({$($g)+}, [$($db)+], [$($hasher)+], [$f]);
        impl_params_bench!({$($g)+}, [$($db)+], [$($hasher)+], [$($fn),*]);
    };

    ({$($g:tt)+}, [$($db:tt)+], [($($h:tt)+), $($hasher:tt),*], [$f:tt]) => {
        impl_params_bench!({$($g)+}, [$($db)+], [($($h)+)], [$f]);
        impl_params_bench!({$($g)+}, [$($db)+], [$($hasher),*], [$f]);
    };

    ({$($g:tt)+}, [($($d:tt)+), $($db:tt),*], [($($h:tt)+)], [$f:tt]) => {
        impl_bench_fn!({$($g)+}, ($($d)+), ($($h)+), $f);
        impl_params_bench!({$($g)+}, [$($db),*], [($($h)+)], [$f]);
    };

    ({$($g:tt)+}, [($($d:tt)+)], [($($h:tt)+)], [$f:tt]) => {
        impl_bench_fn!({$($g)+}, ($($d)+), ($($h)+), $f);
    };

    ($($other:tt)*) => {};
}

macro_rules! impl_bench_fn {
    ({$g:ident, $k:expr, $v: expr}, ($d:expr, $db:ident), ($h:expr, $hasher:ident), $fn:ident) => {
        $g.bench_function(format!("{}_{}_{}", stringify!($fn), $d, $h), |b| {
            let dbname = format!(".tmp/{}", hex!(random_bytes(4)));
            let _g = scopeguard::guard((), |_| {
                if fs::metadata(&dbname).is_ok() {
                    fs::remove_dir_all(&dbname).unwrap()
                }
            });
            let mut tree = Monotree::<$db, $hasher>::new(&dbname);
            let mut root: Option<Hash> = None;
            let mut keys = $k.clone();
            root = match stringify!($fn) {
                "get" | "remove" => insert(
                    black_box(&mut tree),
                    black_box(root),
                    black_box(&mut keys),
                    black_box($v),
                ),
                _ => root,
            };
            b.iter(|| {
                $fn(
                    black_box(&mut tree),
                    black_box(root),
                    black_box(&mut keys),
                    black_box($v),
                )
            })
        });
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
