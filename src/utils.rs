//! A module for implementing some helpful functions for `monotree`.
use crate::*;
use num::{NumCast, PrimInt};
use rand::Rng;
use std::cmp;
use std::ops::Range;

#[macro_export]
/// std::cmp::max() extension for use with multiple arguments.
macro_rules! max {
    ($x:expr) => ($x);
    ($x:expr, $($e:expr),+) => (cmp::max($x, max!($($e),+)));
}

#[macro_export]
/// std::cmp::min() extension for use with multiple arguments.
macro_rules! min {
    ($x:expr) => ($x);
    ($x:expr, $($e:expr),+) => (cmp::min($x, min!($($e),+)));
}

#[macro_export]
/// Convert `bytes` slice into `hex` string.
macro_rules! hex {
    ($bytes:expr) => {{
        hex::encode($bytes)
    }};
}

#[macro_export]
/// Convert elapsed time in nano second (`u128`) into appropriate format of time (`String`).
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
/// Simple benchmark tool for runtime measure of a code block.
///
/// For the given block code, it adds up total time during `LOOPS`-time run
/// and then print and return the total time.
/// When `LOOPS` is 0, it only runs once without `STDOUT`.
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

/// Cast from a typed scalar to another based on `num_traits`
pub fn cast<T: NumCast, U: NumCast>(n: T) -> U {
    NumCast::from(n).expect("cast(): Numcast")
}

/// Generate a random byte based on `rand::random`.
pub fn random_byte() -> u8 {
    rand::random::<u8>()
}

/// Generate random bytes of the given length.
pub fn random_bytes(n: usize) -> Vec<u8> {
    (0..n).map(|_| random_byte()).collect()
}

/// Generate a random `Hash`, byte-array of `HASH_LEN` length.
pub fn random_hash() -> Hash {
    slice_to_hash(&random_bytes(HASH_LEN))
}

/// Generate a vector of random `Hash` with the given length.
pub fn random_hashes(n: usize) -> Vec<Hash> {
    (0..n).map(|_| random_hash()).collect()
}

/// Get a fixed lenght byte-array or `Hash` from slice.
pub fn slice_to_hash(slice: &[u8]) -> Hash {
    let mut hash = [0x00; HASH_LEN];
    hash.copy_from_slice(slice);
    hash
}

/// Shuffle a slice using _Fisher-Yates_ algorithm.
pub fn shuffle<T: Clone>(slice: &mut [T]) {
    let mut rng = rand::thread_rng();
    let s = slice.len();
    (0..s).for_each(|i| {
        let q = rng.gen_range(0..s);
        slice.swap(i, q);
    });
}

/// Get sorted indices from unsorted slice.
pub fn get_sorted_indices<T>(slice: &[T], reverse: bool) -> Vec<usize>
where
    T: Clone + cmp::Ord,
{
    let mut t: Vec<_> = slice.iter().enumerate().collect();
    if reverse {
        t.sort_unstable_by(|(_, a), (_, b)| b.cmp(a));
    } else {
        t.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
    }
    t.iter().map(|(i, _)| *i).collect()
}

/// Get length of the longest common prefix bits for the given two slices.
pub fn len_lcp<T>(a: &[u8], m: &Range<T>, b: &[u8], n: &Range<T>) -> T
where
    T: PrimInt + NumCast,
    Range<T>: Iterator<Item = T>,
{
    let count = (cast(0)..min!(m.end - m.start, n.end - n.start))
        .take_while(|&i| bit(a, m.start + i) == bit(b, n.start + i))
        .count();
    cast(count)
}

/// Get `i`-th bit from bytes slice. Index `i` starts from 0.
pub fn bit<T: PrimInt + NumCast>(bytes: &[u8], i: T) -> bool {
    let q = i.to_usize().expect("bit(): usize") / 8;
    let r = i.to_u8().expect("bit(): u8") % 8;
    (bytes[q] >> (7 - r)) & 0x01 == 0x01
}

/// Get the required length of bytes from a `Range`, bits indices across the bytes.
pub fn nbytes_across<T: PrimInt + NumCast>(start: T, end: T) -> T {
    let n = (end - (start - start % cast(8))) / cast(8);
    if end % cast(8) == cast(0) {
        n
    } else {
        n + cast(1)
    }
}

/// Adjust the bytes representation for `Bits` when shifted.
/// Returns a bytes shift, `n` and thereby resulting shifted range, `R`.
pub fn offsets<T: PrimInt + NumCast>(range: &Range<T>, n: T, tail: bool) -> (T, Range<T>) {
    let x = range.start + n;
    let e: T = cast(8);
    if tail {
        (nbytes_across(range.start, x), range.start..x)
    } else {
        (x / e, x % e..range.end - e * (x / e))
    }
}

/// Convert big-endian bytes into base10 or decimal number.
pub fn bytes_to_int<T: PrimInt + NumCast>(bytes: &[u8]) -> T {
    let l = bytes.len();
    let sum = (0..l).fold(0, |sum, i| {
        sum + (1 << ((l - i - 1) * 8)) * bytes[i] as usize
    });
    cast(sum)
}

/// Get a compressed bytes (leading-zero-truncated big-endian bytes) from a `u64`.
pub fn int_to_bytes(number: u64) -> Vec<u8> {
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

/// Convert a Vec slice of bit or `bool` into a number as `usize`.
pub fn bits_to_usize(bits: &[bool]) -> usize {
    let l = bits.len();
    (0..l).fold(0, |sum, i| sum + ((bits[i] as usize) << (l - 1 - i)))
}

/// Convert a bytes slice into a Vec of bit.
pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    bytes_to_slicebit(bytes, &(0..bytes.len() * 8))
}

/// Convert (bytes slice + Range) representation into bits in forms of `Vec<bool>`.
pub fn bytes_to_slicebit<T>(bytes: &[u8], range: &Range<T>) -> Vec<bool>
where
    T: PrimInt + NumCast,
    Range<T>: Iterator<Item = T>,
{
    range.clone().map(|x| bit(bytes, x)).collect()
}

/// Convert bits, Vec slice of `bool` into bytes, `Vec<u8>`.
pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.rchunks(8)
        .rev()
        .map(|v| bits_to_usize(v) as u8)
        .collect()
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
    fn test_bytes_to_int() {
        let number: usize = bytes_to_int(&[0x73, 0x6f, 0x66, 0x69, 0x61]);
        assert_eq!(number, 495790221665usize);
    }

    #[test]
    fn test_usize_to_bytes() {
        assert_eq!(
            int_to_bytes(495790221665u64),
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
