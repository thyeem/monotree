use crate::consts::HASH_LEN;
use crate::utils::*;
use crate::{Cell, Node, Result, Unit};

impl Node {
    pub fn new(lc: Cell, rc: Cell) -> Self {
        match (&lc, &rc) {
            (&Some(_), &None) => Node::Soft(lc),
            (&Some(_), &Some(_)) => Node::Hard(lc, rc),
            _ => unreachable!(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        match bytes.last() {
            Some(&0x00) => {
                let (cell, _) = Node::parse_bytes(&bytes[..bytes.len() - 1], false)?;
                Ok(Node::Soft(cell))
            }
            Some(&0x01) => {
                let (lc, size) = Node::parse_bytes(&bytes, false)?;
                let (rc, _) = Node::parse_bytes(&bytes[size..bytes.len() - 1], true)?;
                Ok(Node::Hard(lc, rc))
            }
            _ => unreachable!(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bb = |x: usize| (x as u16).to_be_bytes();
        match self {
            Node::Soft(Some(unit)) => Ok([
                &unit.hash[..],
                &bb(unit.range.start),
                &bb(unit.range.end),
                &unit.path,
                &[0x00],
            ]
            .concat()),
            Node::Hard(Some(lu), Some(ru)) => {
                let (lu, ru) = match bit(&ru.path, ru.range.start) {
                    true => (&lu, &ru),
                    false => (&ru, &lu),
                };
                Ok([
                    &lu.hash[..],
                    &bb(lu.range.start),
                    &bb(lu.range.end),
                    &lu.path,
                    &bb(ru.range.start),
                    &bb(ru.range.end),
                    &ru.path,
                    &ru.hash[..],
                    &[0x01],
                ]
                .concat())
            }
            _ => unreachable!(),
        }
    }

    fn parse_bytes(bytes: &[u8], right: bool) -> Result<(Cell, usize)> {
        let l = bytes.len();
        let i = if right { 0usize } else { HASH_LEN };
        let g = if right { l - HASH_LEN..l } else { 0..HASH_LEN };
        let start = bytes_to_usize(&bytes[i..i + 2]);
        let end = bytes_to_usize(&bytes[i + 2..i + 4]);
        let n = nbytes_across(start, end);
        let hash = slice_to_hash(&bytes[g])?;
        let path = bytes[i + 4..i + 4 + n].to_vec();
        let range = start..end;
        Ok((Some(Unit { hash, path, range }), i + 4 + n))
    }
}
