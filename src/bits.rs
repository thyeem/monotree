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
        Ok([
            &(self.range.start as u16).to_be_bytes(),
            &(self.range.end as u16).to_be_bytes(),
            &self.path[..],
        ]
        .concat())
    }

    pub fn first(&self) -> bool {
        bit(&self.path, self.range.start)
    }

    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0 || self.path.len() == 0
    }

    pub fn shift(&self, n: usize, tail: bool) -> Self {
        let (q, range) = offsets(&self.range, n, tail);
        if tail {
            Self {
                path: &self.path[..q],
                range,
            }
        } else {
            Self {
                path: &self.path[q..],
                range,
            }
        }
    }

    pub fn len_common_bits(a: &Self, b: &Self) -> usize {
        len_lcp(&a.path, &a.range, &b.path, &b.range)
    }
}
