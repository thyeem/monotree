use crate::bits::Bits;
use crate::consts::HASH_LEN;
use crate::utils::*;
use crate::Result;

pub type Proof = Vec<(bool, Vec<u8>)>;
pub type Cell<'a> = Option<Unit<'a>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Node<'a> {
    Soft(Cell<'a>),
    Hard(Cell<'a>, Cell<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit<'a> {
    pub hash: &'a [u8],
    pub bits: Bits<'a>,
}

impl<'a> Node<'a> {
    pub fn new(lc: Cell<'a>, rc: Cell<'a>) -> Self {
        match (&lc, &rc) {
            (&Some(_), &None) => Node::Soft(lc),
            (&Some(_), &Some(_)) => Node::Hard(lc, rc),
            _ => unreachable!(),
        }
    }

    pub fn cells_from_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, Cell<'a>)> {
        match Node::from_bytes(&bytes)? {
            Node::Soft(cell) => Ok((cell, None)),
            Node::Hard(lc, rc) => match right {
                true => Ok((rc, lc)),
                false => Ok((lc, rc)),
            },
        }
    }

    fn parse_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, usize)> {
        let l = bytes.len();
        let i = if right { 0usize } else { HASH_LEN };
        let g = if right { l - HASH_LEN..l } else { 0..HASH_LEN };
        let start = bytes_to_usize(&bytes[i..i + 2]);
        let end = bytes_to_usize(&bytes[i + 2..i + 4]);
        let n = nbytes_across(start, end);
        Ok((
            Some(Unit {
                hash: &bytes[g],
                bits: Bits {
                    path: &bytes[i + 4..i + 4 + n],
                    range: start..end,
                },
            }),
            i + 4 + n,
        ))
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
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
        match self {
            Node::Soft(Some(unit)) => {
                Ok([&unit.hash[..], &unit.bits.to_bytes()?, &[0x00]].concat())
            }
            Node::Hard(Some(lu), Some(ru)) => {
                let (lu, ru) = match ru.bits.first() {
                    true => (&lu, &ru),
                    false => (&ru, &lu),
                };
                Ok([
                    &lu.hash[..],
                    &lu.bits.to_bytes()?,
                    &ru.bits.to_bytes()?,
                    &ru.hash[..],
                    &[0x01],
                ]
                .concat())
            }
            _ => unreachable!(),
        }
    }
}
