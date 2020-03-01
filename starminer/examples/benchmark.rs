use crate::dynamic_smt::dynamic_sparse_main;
use crate::hashtree::starling_main;
use crate::merkletrie::{patricia_main, patricia_order};
use crate::smt::{sparse_main, sparse_order};
use bitvec::prelude::*;

#[allow(dead_code)]
fn test_order() -> Result<(), failure::Error> {
    sparse_order()?;
    patricia_order()?;
    Ok(())
}

#[allow(dead_code)]
fn benchmark() -> Result<(), failure::Error> {
    sparse_main()?;
    patricia_main()?;
    starling_main()?;
    Ok(())
}

#[allow(dead_code)]
fn binary_test() -> Result<(), failure::Error> {
    //big endian
    let mut bv = bitvec![Msb0, u8; 0,0,0, 1, 0, 1];
    for _i in 0..8 {
        bv.push(true);
    }

    let m = bincode::serialize(&bv).unwrap();
    let bv2: BitVec<Msb0, u8> = bincode::deserialize(&m).unwrap();
    let m2 = bincode::serialize(&bv2).unwrap();
    assert!(m == m2);

    println!("{:?}", bv);
    println!("{:?}", &bv[0..3]);
    println!("encoded:{} bytes", m.len());
    Ok(())
}
pub fn benchmark_main() -> Result<(), failure::Error> {
    sparse_main()?;
    patricia_main()?;
    starling_main()?;
    dynamic_sparse_main()?;
    Ok(())
}
