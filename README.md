# Binary Radix Tree
Rust implementation of an optimized-sparse-merkle-tree

## Usage
How to generate a new binary radix tree
```rust
    let mut tree = tree::BinaryRadix::_DBTYPE_::new(_HASH_BYTE_, "");
    let mut root = tree.new_tree().unwrap();
```

## Example
 ```rust
    use binradix::utils::*;
    use binradix::{database, tree, BytesTuple, VoidResult};

    fn main() -> VoidResult {
        /// declare the size of hash in byte
        const HASH_BYTE: usize = 32;

        /// gen random key-value pair
        let kv_pair: Vec<BytesTuple> = (0..10000)
            .map(|_| {
                (
                    random_bytes(HASH_BYTE).unwrap(),
                    random_bytes(HASH_BYTE).unwrap(),
                )
            })
            .collect();
        Ok(())

        /// init tree using either MemDB or RocksDB
        let mut tree = tree::BinaryRadix::<database::MemoryDB>::new(HASH_BYTE, "");
        // let mut tree = tree::BinaryRadix::<database::RocksdbDB>::new(HASH_BYTE, "");
        let mut root = tree.new_tree()?;

        /// Simple, but rubust basic operation test.  
        /// Uncomment the last part when testing inclusion-merkle-proof as well.
        for (k, v) in kv_pair {
            root = tree.update(&root, &key, &value)?;
            assert_eq!(t.get_leaf(&r, &k).unwrap(), *v);
            // let pf = t.get_merkle_proof(&r, &k)?;
            // assert_eq!(verify_proof(32, &r, &v, &pf), true);
        }
    }
}



 ```
