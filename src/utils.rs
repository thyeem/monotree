extern crate hex;
use crate::consts::HASH_LEN;
use crate::{Hash, Result};
use blake2_rfc::blake2b::{blake2b, Blake2bResult};
use std::cmp;
use std::ops::Range;

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
/// converts elapsed nano second into appropriate format of time
/// fmtime!(elapsed nano secs as u128) -> &'static str
macro_rules! fmtime {
    ($t:expr) => {{
        match $t as f64 {
            t if t > 1e9 => format!("{:.4} s", 1e-9 * t),
            t if t > 1e6 => format!("{:.4} ms", 1e-6 * t),
            t if t > 1e3 => format!("{:.4} us", 1e-3 * t),
            t if t > 1e0 => format!("{:.4} ns", 1e-0 * t),
            _ => format!("under 1 ns"),
        }
    }};
}

#[macro_export]
/// Super-simple benchmark tool for measurement of runtime.
/// For the given block code, it adds up total time
/// during NUMBER_OF_LOOP-times run and then print and return it.
/// When NUMBER_OF_LOOP is 0, just run once without STDOUT
///
/// perf!(NUMBER_OF_LOOP, LABEL, {CODE BLOCK} ) -> nano secs as u128
/// Example:
///     perf!(NUMBER_OF_LOOP, LABEL, {
///         // HERE-any-code-block-to-measure
///     });
macro_rules! perf {
    ($n:expr, $label:expr, $code:block) => {{
        let label = match $label.trim().len() {
            0 => stringify!($code),
            _ => $label,
        };
        let tick = std::time::Instant::now();
        match $n {
            0 => $code,
            _ => (0..$n).for_each(|_| $code),
        }
        let tock = tick.elapsed().as_nanos();
        match $n {
            0 => tock,
            _ => {
                let elapsed = fmtime!(tock);
                let div = if $n == 0 { 1 } else { $n };
                let mean = fmtime!(tock / div as u128);
                println!("\n{}", label);
                println!("{} loops: {}  ({}, on average)", $n, elapsed, mean);
                tock
            }
        }
    }};
}

pub fn debug<T>(x: &T)
where
    T: std::fmt::Debug,
{
    println!("{:?}", x);
}

pub fn random_bytes(n: usize) -> Vec<u8> {
    (0..n).map(|_| rand::random::<u8>()).collect()
}

pub fn hash_fn_factory(n: usize) -> impl Fn(&[u8]) -> Blake2bResult {
    move |x| blake2b(n, &[], x)
}

/// get length of the Longest Common Prefix bits for a set of two bytes
pub fn len_lcp(a: &[u8], m: &Range<usize>, b: &[u8], n: &Range<usize>) -> usize {
    (0..min!(m.end - m.start, n.end - n.start))
        .take_while(|&i| bit(a, m.start + i) == bit(b, n.start + i))
        .count()
}

/// get ith-index-bit from bytes
/// note that index i starts from 0
pub fn bit(bytes: &[u8], i: usize) -> bool {
    let (q, r) = (i / 8, i % 8);
    (bytes[q] >> (7 - r) & 0x01) == 0x01
}

pub fn nbytes_across(s: usize, e: usize) -> usize {
    let n = (e - (s - s % 8)) / 8;
    match e % 8 {
        0 => n,
        _ => n + 1,
    }
}

pub fn offsets(range: &Range<usize>, n: usize, tail: bool) -> (usize, Range<usize>) {
    let x = range.start + n;
    if tail {
        (nbytes_across(range.start, x), range.start..x)
    } else {
        (x / 8, x % 8..range.end - 8 * (x / 8))
    }
}

/// convert any big-endian bytes into base10 (decimal number)
/// slightly slower than usize::from_be_bytes(),
/// but can go with various length of bytes
pub fn bytes_to_usize(bytes: &[u8]) -> usize {
    let l = bytes.len();
    (0..l).fold(0, |sum, i| {
        sum + (1 << ((l - i - 1) * 8)) * bytes[i] as usize
    })
}

/// usize::to_be_bytes()'s enough, but need to compress bytes
/// cutting down big-endian bytes leading zero
pub fn usize_to_bytes(number: usize) -> Vec<u8> {
    match number {
        0 => vec![0x00],
        _ => number
            .to_be_bytes()
            .iter()
            .skip_while(|&x| *x == 0x00)
            .copied()
            .collect(),
    }
}

pub fn bits_to_usize(bits: &[bool]) -> usize {
    let l = bits.len();
    (0..l).fold(0, |sum, i| sum + ((bits[i] as usize) << (l - 1 - i)))
}

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    bytes_to_slicebit(bytes, &(0..bytes.len() * 8))
}

pub fn bytes_to_slicebit(bytes: &[u8], range: &Range<usize>) -> Vec<bool> {
    range.clone().map(|x| bit(bytes, x)).collect()
}

pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.rchunks(8)
        .rev()
        .map(|v| bits_to_usize(v) as u8)
        .collect()
}

pub fn slice_to_hash(slice: &[u8]) -> Result<Hash> {
    let mut hash = [0x00; HASH_LEN];
    hash.copy_from_slice(slice);
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bit() {
        let bytes = [0x73, 0x6f, 0x66, 0x69, 0x61];
        assert_eq!(bit(&bytes, 10), true);
        assert_eq!(bit(&bytes, 20), false);
        assert_eq!(bit(&bytes, 30), false);
    }

    #[test]
    fn test_nbyte_across() {
        assert_eq!(nbytes_across(0, 8), 1);
        assert_eq!(nbytes_across(1, 7), 1);
        assert_eq!(nbytes_across(5, 9), 2);
        assert_eq!(nbytes_across(9, 16), 1);
        assert_eq!(nbytes_across(7, 19), 3);
    }

    #[test]
    fn test_offsets() {
        assert_eq!(offsets(&(0..8), 1, false), (0, 1..8));
        assert_eq!(offsets(&(0..8), 1, true), (1, 0..1));
        assert_eq!(offsets(&(3..20), 10, false), (1, 5..12));
        assert_eq!(offsets(&(3..20), 10, true), (2, 3..13));
        assert_eq!(offsets(&(9..16), 5, false), (1, 6..8));
        assert_eq!(offsets(&(9..16), 5, true), (1, 9..14));
    }

    #[test]
    fn test_bytes_to_usize() {
        assert_eq!(
            bytes_to_usize(&[0x73, 0x6f, 0x66, 0x69, 0x61]),
            495790221665usize
        );
    }

    #[test]
    fn test_usize_to_bytes() {
        assert_eq!(
            usize_to_bytes(495790221665usize),
            [0x73, 0x6f, 0x66, 0x69, 0x61]
        );
    }

    #[test]
    fn test_bytes_to_bits() {
        assert_eq!(
            bytes_to_bits(&[0x33, 0x33]),
            [
                false, false, true, true, false, false, true, true, false, false, true, true,
                false, false, true, true,
            ]
        );
    }

    #[test]
    fn test_bits_to_bytes() {
        let bits = [
            false, false, true, true, false, false, true, true, false, false, true, true, false,
            false, true, true,
        ];
        assert_eq!(bits_to_bytes(&bits), [0x33, 0x33]);
    }

    #[test]
    fn test_bits_to_usize() {
        assert_eq!(
            bits_to_usize(&[
                false, false, true, true, false, false, true, true, false, false, true, true,
                false, false, true, true,
            ]),
            13107usize
        );
    }

    #[test]
    fn test_len_lcp() {
        let sofia = [0x73, 0x6f, 0x66, 0x69, 0x61];
        let maria = [0x6d, 0x61, 0x72, 0x69, 0x61];
        assert_eq!(len_lcp(&sofia, &(0..3), &maria, &(0..3)), 3);
        assert_eq!(len_lcp(&sofia, &(0..3), &maria, &(5..9)), 0);
        assert_eq!(len_lcp(&sofia, &(2..9), &maria, &(18..30)), 5);
        assert_eq!(len_lcp(&sofia, &(20..30), &maria, &(3..15)), 4);
    }
}
