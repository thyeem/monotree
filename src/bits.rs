use crate::utils::*;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub type BitsRange = Range<u16>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits<'a> {
    pub path: &'a [u8],
    pub range: BitsRange,
}

impl<'a> Bits<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bits {
            path: bytes,
            range: 0u16..(bytes.len() as u16 * 8u16),
        }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        let start: u16 = bytes_to_int(&bytes[..2]);
        let end: u16 = bytes_to_int(&bytes[2..4]);
        Self {
            path: &bytes[4..],
            range: start..end,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok([
            &self.range.start.to_be_bytes(),
            &self.range.end.to_be_bytes(),
            &self.path[..],
        ]
        .concat())
    }

    pub fn first(&self) -> bool {
        bit(&self.path, self.range.start)
    }

    pub fn len(&self) -> usize {
        (self.range.end - self.range.start) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0 || self.path.len() == 0
    }

    pub fn shift(&self, n: usize, tail: bool) -> Self {
        let (q, range) = offsets(&self.range, n as u16, tail);
        if tail {
            Self {
                path: &self.path[..q as usize],
                range,
            }
        } else {
            Self {
                path: &self.path[q as usize..],
                range,
            }
        }
    }

    pub fn len_common_bits(a: &Self, b: &Self) -> usize {
        len_lcp(&a.path, &a.range, &b.path, &b.range) as usize
    }
}
