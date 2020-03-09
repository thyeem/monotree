# MonoTree
Rust implementation of an optimized Sparse Merkle Tree.   
This is kind of a binary-radix tree based on bitwise branching.   

- Simple and ___very easy to understand___, but ___fast___ and robust.  
- Fully featured Sparse Merkle Tree (SMT) as a storage including _inclusion proof_ , _non-inclusion proof_, and _its verification_.  
- Not verbose neither mouthful at all.  

Currently, no nibbles or lumps of bit (nor 4-bit nibbles neither bytewise). The branching unit is just a single bit.  

## Dependancies
This library mostly relies on the _Rust standard library only_ except for hash function (`Blake2`) and database API (`Rocksdb`).

## Usage
How to generate and manipulate a new MonoTree instance

### usage::outline
```rust
    // using MemoryDB (HashMap) or RocksDB
    // DBTYPE := MemoryDB | RocksDB
    let mut tree = tree::MonoTree::<DBTYPE>::new("");
    let mut root = tree.new_tree();
    // ... do something with `tree` and `root`
```

### usage::example
 ```rust
    use monotree::consts::HASH_LEN;
    use monotree::database::{MemoryDB, RocksDB};
    use monotree::tree::MonoTree;
    use monotree::utils::*;
    use monotree::*;

    fn main() {
        /// gen random 10000 key-value pair
        let pairs: Vec<(Hash, Hash)> = (0..10000)
            .map(|_| (random_bytes(HASH_LEN), random_bytes(HASH_LEN)))
            .map(|x| (slice_to_hash(&x.0).unwrap(), slice_to_hash(&x.1).unwrap()))
            .collect();

        /// init tree using either MemoryDB or RocksDB
        let mut tree = MonoTree::<MemoryDB>::new("memdb");
        // let mut tree = MonoTree::<RocksDB>::new("testdb");
        let mut root = tree.new_tree();

        // insert random key-value pairs into the tree
        pairs.iter().for_each(|(key, value)| {
            // insert a key
            root = tree.insert(root.as_ref(), key, value);

            // immediately check if the key-value pair was correctly inserted
            let leaf = tree.get(root.as_ref(), key).unwrap();
            assert_eq!(leaf, *value);
        });

        // More strictly, all the assertions below must be satisfied 
        // from the last obtained root.
        pairs.iter().for_each(|(key, value)| {
            let leaf = tree.get(root.as_ref(), key).unwrap();
            assert_eq!(leaf, *value);
        });
    }
}
 ```
## Merkle proof example: verifying inclusion of data
### proof::outline
```rust
    // gen proof simply,
    let proof = tree.get_merkle_proof(ROOT_REF, KEY_REF).unwrap();

    // verify proof
    tree::verify_proof(ROOT_REF, VALUE, PROOF_REF)
```
### proof::example
```rust
    // suppose pairs of (key: Hash, value: Hash) were alreay inserted.
    // test all of those with the last root obtained.
    pairs.iter().for_each(|(key, value)| {
        // gen merkle proof on the key
        let proof = tree.get_merkle_proof(root.as_ref(), key).unwrap();

        // verify the proof
        assert_eq!(tree::verify_proof(root.as_ref(), value, &proof), true);
    });
```

## Performance

_TBD_

## Further improvements
_This MonoTree already seems to outperform some of the well-known trees in Rust crates_.
However, this is a special case of the PoT (Power Of Two) binary tree. If it were generalized with the PoT Tree (2^n nibbles of unit), there would have been room for further performance improvement. This generalization would be future-work.


## Benchmark and tests
performs a micro-benchmark based on _Criterion [https://crates.io/crates/criterion]_
```bash
    $ cargo bench
```
and a macroscopic time-scale benchmark was also prepared in _main.rs_ (with broad error-bar).
```bash
    $ cargo run --release
```
and, as always, some unit tests from _utils.rs_. (will be more covered up)
```bash
    $ cargo test
```