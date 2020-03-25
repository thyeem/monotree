use monotree::database::*;
use monotree::hasher::*;
use monotree::utils::*;
use monotree::*;
use std::fs;

macro_rules! call_perf_mixed {
    ([$($db:tt)+], [$($hasher:tt)+], [$n:tt, $($ns:tt),*]) => {
        call_perf_mixed!([$($db)+], [$($hasher)+], [$n]);
        call_perf_mixed!([$($db)+], [$($hasher)+], [$($ns),*]);
    };

    ([$($db:tt)+], [($($h:tt)+), $($hasher:tt),*], [$n:tt]) => {
        call_perf_mixed!([$($db)+], [($($h)+)], [$n]);
        call_perf_mixed!([$($db)+], [$($hasher),*], [$n]);
    };

    ([($($d:tt)+), $($db:tt),*], [($($h:tt)+)], [$n:tt]) => {
        impl_perf!($n, ($($d)+), ($($h)+));
        call_perf_mixed!([$($db),*], [($($h)+)], [$n]);
    };

    ([($($d:tt)+)], [($($h:tt)+)], [$n:tt]) => {
        impl_perf!($n, ($($d)+), ($($h)+));
    };

    ($($other:tt)*) => {};
}

macro_rules! impl_perf {
    ($n:expr, ($d:expr, $db:ident), ($h:expr, $hasher:ident)) => {{
        let keys = random_hashes($n);
        let leaves = random_hashes($n);
        let dbname = format!(".tmp/{}", hex!(random_bytes(4)));
        let _g = scopeguard::guard((), |_| {
            if fs::metadata(&dbname).is_ok() {
                fs::remove_dir_all(&dbname).unwrap()
            }
        });
        let mut tree = Monotree::<$db, $hasher>::new(&dbname);
        let mut root: Option<Hash> = None;
        let msg = format!("{}-{}  {}", $d, $h, $n);
        perf!(1, &msg, {
            root = tree.inserts(root.as_ref(), &keys, &leaves).unwrap();
        });
        assert_ne!(root, None);
    }};
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    call_perf_mixed!(
        [("hashmap", MemoryDB), ("rocksdb", RocksDB), ("sled", Sled)],
        [
            ("blake3", Blake3),
            ("blake2s", Blake2s),
            ("blake2b", Blake2b),
            ("sha2", Sha2),
            ("sha3", Sha3)
        ],
        [100_000, 500_000, 1_000_000]
    );
}
