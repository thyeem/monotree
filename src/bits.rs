//! A module for representing `BitVec` in terms of bytes slice.
use crate::utils::*;
use crate::*;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
/// `BitVec` implementation based on bytes slice.
pub struct Bits<'a> {
    pub path: &'a [u8],
    pub range: Range<BitsLen>,
}

impl<'a> Bits<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bits {
            path: bytes,
            range: 0..(bytes.len() as BitsLen * 8),
        }
    }

    /// Construct `Bits` instance by deserializing bytes slice.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        let u = std::mem::size_of::<BitsLen>();
        let start: BitsLen = bytes_to_int(&bytes[..u]);
        let end: BitsLen = bytes_to_int(&bytes[u..2 * u]);
        Self {
            path: &bytes[2 * u..],
            range: start..end,
        }
    }

    /// Serialize `Bits` into bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let start = (self.range.start / 8) as usize;
        let end = self.range.end.div_ceil(8) as usize;
        let mut path = self.path[start..end].to_owned();
        let r = (self.range.start % 8) as u8;
        if r != 0 {
            let mask = 0xffu8 >> r;
            path[0] &= mask;
        }
        let r = (self.range.end % 8) as u8;
        if r != 0 {
            let mask = 0xffu8 << (8 - r);
            let last = path.len() - 1;
            path[last] &= mask;
        }
        Ok([
            &self.range.start.to_be_bytes(),
            &self.range.end.to_be_bytes(),
            &path[..],
        ]
        .concat())
    }

    /// Get the very first bit.
    pub fn first(&self) -> bool {
        bit(self.path, self.range.start)
    }

    pub fn len(&self) -> BitsLen {
        self.range.end - self.range.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0 || self.path.len() == 0
    }

    /// Get the first `n` bits.
    pub fn take(&self, n: BitsLen) -> Self {
        let x = self.range.start + n;
        let q = nbytes_across(self.range.start, x);
        let range = self.range.start..x;
        Self {
            path: &self.path[..q as usize],
            range,
        }
    }

    /// Skip the first `n` bits.
    pub fn drop(&self, n: BitsLen) -> Self {
        let x = self.range.start + n;
        let q = x / 8;
        let range = x % 8..self.range.end - 8 * (x / 8);
        Self {
            path: &self.path[q as usize..],
            range,
        }
    }

    /// Get length of the longest common prefix bits for the given two `Bits`.
    pub fn len_common_bits(a: &Self, b: &Self) -> BitsLen {
        len_lcp(a.path, &a.range, b.path, &b.range)
    }
}
