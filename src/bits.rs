use crate::utils::*;
use crate::Result;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct Bits<'a> {
    pub path: &'a [u8],
    pub range: Range<usize>,
}

impl<'a> Bits<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bits {
            path: bytes,
            range: 0..bytes.len() * 8,
        }
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let b = |x: usize| (x as u16).to_be_bytes();
        Ok([&b(self.range.start), &b(self.range.end), &self.path[..]].concat())
    }

    pub fn first(&self) -> bool {
        bit(&self.path, self.range.start)
    }

    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    pub fn shift(&self, n: usize, tail: bool) -> Self {
        let (q, range) = offsets(&self.range, n, tail);
        match tail {
            false => Self {
                path: &self.path[q..],
                range,
            },
            true => Self {
                path: &self.path[..q],
                range,
            },
        }
    }

    pub fn len_common_bits(a: &Self, b: &Self) -> usize {
        len_lcp(&a.path, &a.range, &b.path, &b.range)
    }
}
