#![allow(non_snake_case)]

use crate::{BitsResult, BytesResult, Node, ParseResult, Proof};
use blake2_rfc::blake2b::{blake2b, Blake2bResult};
use std::cmp;
extern crate hex;

#[macro_export]
macro_rules! max {
    ($x: expr) => ($x);
    ($x: expr, $($e: expr),+) => (cmp::max($x, max!($($e),*)));
}

#[macro_export]
macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($e: expr),+) => (cmp::min($x, min!($($e),*)));
}

#[macro_export]
macro_rules! hex {
    ($bytes:expr) => {{
        hex::encode($bytes)
    }};
}

#[macro_export]
macro_rules! bit {
    ($bytes:expr, $i:expr) => {{
        assert!($i > 0);
        let (q, r) = (($i - 1) / 8, ($i - 1) % 8);
        ($bytes[q] >> (7 - r) & 1u8) == 1u8
    }};
}

#[macro_export]
macro_rules! byte {
    ($bits:expr) => {{
        let n = $bits.len();
        assert!(n > 0 && n < 9);
        (0..n).fold(0, |sum, i| sum + (($bits[i] as u8) << (n - 1 - i)))
    }};
}

#[macro_export]
macro_rules! nbit_u8 {
    ($bytes:expr) => {{
        let mut nbit = [0u8; 1];
        nbit.copy_from_slice($bytes);
        u8::from_be_bytes(nbit)
    }};
}

#[macro_export]
macro_rules! nbit_u16 {
    ($bytes:expr) => {{
        let mut nbit = [0u8; 2];
        nbit.copy_from_slice($bytes);
        u16::from_be_bytes(nbit)
    }};
}

#[macro_export]
macro_rules! nbyte {
    ($nbit:expr) => {{
        if $nbit % 8 == 0 {
            ($nbit / 8) as usize
        } else {
            ($nbit / 8 + 1) as usize
        }
    }};
}

#[macro_export]
macro_rules! is_rbit {
    ($bits:expr) => {{
        match $bits.get(0) {
            Some(&v) => v,
            None => false,
        }
    }};
}

pub fn random_bytes(n: usize) -> BytesResult {
    Ok((0..n).map(|_| rand::random::<u8>()).collect())
}

pub fn hash_fn_factory(n: usize) -> Box<dyn Fn(&[u8]) -> Blake2bResult> {
    Box::new(move |x| blake2b(n, &[], x))
}

pub fn bytes_to_bits(bytes: &[u8]) -> BitsResult {
    bytes_to_slicebit(bytes, 1, bytes.len() * 8 + 1)
}

pub fn bytes_to_slicebit(bytes: &[u8], from: usize, to: usize) -> BitsResult {
    Ok((from..to).map(|x| bit!(bytes, x)).collect())
}

pub fn bits_to_bytes(bits: &[bool]) -> BytesResult {
    Ok(bits.rchunks(8).rev().map(|v| byte!(v)).collect())
}

// get length of the Longest Common Prefix bits for a set of two bytes
pub fn len_lcp(a: &[bool], b: &[bool]) -> usize {
    (0..min!(a.len(), b.len()))
        .take_while(|&x| a[x] == b[x])
        .fold(0, |sum, _| sum + 1)
}

pub fn hash(nbyte: usize, bytes: &[u8]) -> BytesResult {
    match bytes.is_empty() {
        true => Ok(vec![]),
        false => Ok(blake2b(nbyte, &[], bytes).as_bytes().to_vec()),
    }
}

pub fn type_from_bytes(bytes: &[u8]) -> Node {
    match bytes.last() {
        Some(&0u8) => Node::Soft,
        Some(&1u8) => Node::Hard,
        _ => unreachable!(),
    }
}

pub fn type_from_parsed(h: &[u8], b: &[bool], H: &[u8], B: &[bool]) -> Node {
    match (h.is_empty(), b.is_empty(), H.is_empty(), B.is_empty()) {
        (_, false, true, true) => Node::Soft,
        (false, false, false, false) => Node::Hard,
        _ => unreachable!(),
    }
}

pub fn encode_node(h: &[u8], bits: &[bool], right: bool) -> BytesResult {
    let nbit = (bits.len() as u16).to_be_bytes();
    let bytes = bits_to_bytes(bits)?;
    match right {
        true => Ok([&nbit[..], &bytes, h].concat()),
        false => Ok([h, &nbit[..], &bytes].concat()),
    }
}

pub fn decode_node(bytes: &[u8], size: usize, right: bool) -> ParseResult {
    let l = bytes.len();
    let n: usize = if right { 0 } else { size };
    let nbit = nbit_u16!(&bytes[n..n + 2]);
    let nbyte = nbyte!(nbit);
    let r = match nbit % 8 {
        0 => 0usize,
        _ => (8 - (nbit % 8)) as usize,
    };
    let bits = bytes_to_slicebit(&bytes[n + 2..n + 2 + nbyte], r + 1, nbyte * 8 + 1)?;
    match right {
        true => Ok((bytes[l - size..l].to_vec(), bits, n + 2 + nbyte)),
        false => Ok((bytes[..size].to_vec(), bits, n + 2 + nbyte)),
    }
}

pub fn verify_proof(nbyte: usize, root: &[u8], leaf: &[u8], proof: &Proof) -> bool {
    let mut h: Vec<u8> = leaf.to_vec();
    for (prefix, cut) in proof.iter().rev() {
        if prefix == &[0u8] {
            let o = [h.as_slice(), cut.as_slice()].concat();
            h = hash(nbyte, &o).unwrap();
        } else if prefix == &[1u8] {
            let l = cut.len();
            let o = [&cut[..l - 1], h.as_slice(), &cut[l - 1..]].concat();
            h = hash(nbyte, &o).unwrap();
        } else {
            return false;
        }
    }
    root.to_vec() == h
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_macro_byte() {
        let bits = vec![false, false, true, true, false, false, true, true];
        assert_eq!(byte!(&bits), 51);
        assert_eq!(byte!(&bits[..4]), 3);
        assert_eq!(byte!(&bits[..6]), 12);
        assert_eq!(byte!(&bits[..7]), 25);
    }
    #[test]
    fn test_hash() {
        let hash_sofimarie = [
            251, 130, 151, 239, 42, 158, 87, 241, 137, 148, 66, 113, 96, 195, 10, 104, 93, 80, 72,
            178, 87, 51, 209, 15, 22, 199, 243, 142, 92, 183, 55, 196,
        ];
        let sofimarie = hash(32, b"sofimarie").unwrap();
        let hash_nil: Vec<u8> = vec![];
        let nil = hash(32, &[]).unwrap();
        assert_eq!(hash_sofimarie.to_vec(), sofimarie);
        assert_eq!(hash_nil, nil);
    }
    #[test]
    fn test_len_lcp() {
        let a = vec![false, false, false, true, false];
        let b = vec![false, false, true, false];
        let c = vec![true, false, false];
        assert_eq!(len_lcp(&a, &a), 5);
        assert_eq!(len_lcp(&a, &b), 2);
        assert_eq!(len_lcp(&b, &c), 0);
    }
    #[test]
    fn test_bytes_to_bits() {
        let bytes = vec![100u8, 200];
        let bits = vec![
            false, true, true, false, false, true, false, false, true, true, false, false, true,
            false, false, false,
        ];
        assert_eq!(bytes_to_bits(&bytes).unwrap(), bits);
    }
    #[test]
    fn test_bits_to_bytes() {
        let bits = vec![
            false, false, true, true, false, false, true, true, false, false, true, true, false,
            false, true, true,
        ];
        assert_eq!(bits_to_bytes(&bits).unwrap(), vec![51, 51]);
        assert_eq!(bits_to_bytes(&bits[3..14]).unwrap(), vec![4, 204]);
    }
    #[test]
    fn test_encode_node() {
        let bits = vec![
            false, false, true, true, false, false, true, true, false, false, true, true, false,
            false, true, true,
        ];
        let hash = vec![33u8, 77];
        assert_eq!(
            encode_node(&hash, &bits, true).unwrap(),
            vec![0u8, 16, 51, 51, 33, 77]
        );
        assert_eq!(
            encode_node(&hash, &bits, false).unwrap(),
            vec![33u8, 77, 0, 16, 51, 51]
        );
        assert_eq!(
            encode_node(&hash, &bits[3..14], true).unwrap(),
            vec![0u8, 11, 4, 204, 33, 77]
        );
        assert_eq!(
            encode_node(&hash, &bits[3..14], false).unwrap(),
            vec![33u8, 77, 0, 11, 4, 204]
        );
    }
    #[test]
    fn test_decode_node() {
        let bits = vec![
            false, false, true, true, false, false, true, true, false, false, true, true, false,
            false, true, true,
        ];
        assert_eq!(
            decode_node(&[0u8, 16, 51, 51, 33, 77], 2, true).unwrap(),
            (vec![33u8, 77], bits.as_slice().to_vec(), 4)
        );
        assert_eq!(
            decode_node(&[33u8, 77, 0, 16, 51, 51], 2, false).unwrap(),
            (vec![33u8, 77], bits.as_slice().to_vec(), 6)
        );
        assert_eq!(
            decode_node(&[0u8, 11, 4, 204, 33, 77], 2, true).unwrap(),
            (vec![33u8, 77], bits[3..14].to_vec(), 4)
        );
        assert_eq!(
            decode_node(&[33u8, 77, 0, 11, 4, 204], 2, false).unwrap(),
            (vec![33u8, 77], bits[3..14].to_vec(), 6)
        );
    }
}
