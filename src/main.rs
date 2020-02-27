#[macro_use]
extern crate timeit;
use binradix::utils::*;
use binradix::{database, tree, BytesTuple, VoidResult};
use starling::hash_tree::HashTree;

fn main() -> VoidResult {
    const HASH_BYTE: usize = 32;
    let kv_pair: Vec<BytesTuple> = (0..10000)
        .map(|_| {
            (
                random_bytes(HASH_BYTE).unwrap(),
                random_bytes(HASH_BYTE).unwrap(),
            )
        })
        .collect();
    println!("random (k, v) pairs:  {:?}", &kv_pair.len());
    test_binradix(&kv_pair)?;
    test_merklebit(&kv_pair)?;
    Ok(())
}

#[allow(dead_code)]
fn test_binradix(kv_pair: &[BytesTuple]) -> VoidResult {
    let mut t = tree::BinaryRadix::<database::MemoryDB>::new(32, "MEM");
    let mut r = t.new_tree()?;
    timeit!({
        for (k, v) in kv_pair {
            r = t.update(&r, &k, &v)?;
            // assert_eq!(t.get_leaf(&r, &k).unwrap(), *v);
            // let pf = t.get_merkle_proof(&r, &k)?;
            // assert_eq!(verify_proof(32, &r, &v, &pf), true);
        }
    });
    Ok(())
}

#[allow(dead_code)]
fn test_merklebit(kv_pair: &[BytesTuple]) -> VoidResult {
    let mut tree = HashTree::<[u8; 32], Vec<u8>>::new(256).unwrap();
    let mut root: Option<&[u8; 32]> = None;
    let mut root_value: [u8; 32];
    timeit!({
        for (k, v) in kv_pair {
            let mut key = [0u8; 32];
            let mut value = vec![0u8; 32];
            key.copy_from_slice(&k[0..32]);
            value.copy_from_slice(&v[0..32]);
            root_value = tree.insert(root, &mut [key], &[value.clone()]).unwrap();
            root = Some(&root_value);
            // let leaf = tree.get(&root_value, &mut [key]).unwrap();
            // assert_eq!(leaf.get(&key).unwrap(), &Some(value));
        }
    });
    Ok(())
}
