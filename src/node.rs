//! A module for defining `Node` used in `monotree`.
use crate::utils::*;
use crate::*;

/// A type for describing components of `Node`: a real element `Unit` or a virtual element `None`.
pub type Cell<'a> = Option<Unit<'a>>;

#[derive(Clone, Debug, PartialEq)]
/// An component of `Node` consisting of `Hash` and `Bits`, which represents a joint of subtrees it has.
pub struct Unit<'a> {
    pub hash: &'a [u8],
    pub bits: Bits<'a>,
}

#[derive(Clone, Debug)]
/// The only component of `monotree`. In a big picture, `monotree` simply consists of structured `Node`s.
///
/// # Schematic
/// There are two types of `Node` -- ___Soft node___ and ___Hard node___.   
/// * ___Hard___: a node that has two real cells as components. two links to child nodes.
/// * ___Soft___: a node that has only one real cell and it has only one link going out to child node.
/// ```
/// //              Root
/// //             /    \
/// //          NodeA   NodeB
/// //         /     \      \
/// //      NodeC   LeafB  LeafC
/// //       /
/// //     LeafA
/// ```
/// where NodeA is a _Hard node_, NodeB and NodeC are _Soft nodes_.
///
/// # Byte-Serialized View
/// Numbers in parentheses refer to byte length.
/// By default `HashLen = 32`, `BitsLen = 2`.
///
/// _SoftNode_ = `Cell` + `0x00`(1), where    
/// `Cell` = `hash`(`HASH_LEN`) + `path`(`< HASH_LEN`) + `range_start`(`BitsLen`) + `range_end`(`BitsLen`).   
/// `0x00` is an indicator for soft node.  
///
/// _HardNode_ = `Cell_L` + `Cell_R` + `0x01`(1), where    
/// `Cell_L` = `hash_L`(`HASH_LEN`) + `path_L`(`< HASH_LEN`) + `range_L_start`(`BitsLen`) + `range_L_end`(`BitsLen`)   
/// `Cell_R` = `path_R`(`< HASH_LEN`) _ `range_R_start`(`BitsLen`) + `range_R_end`(`BitsLen`) + `hash_R`(`HASH_LEN`).   
/// `0x01` is an indicator for hard node.
///
/// To make ***Merkle proof*** easier, we purposely placed the _hashes_ on outskirts of the serialized form.
/// With only 1-bit information of left or right, provers can easily guess
/// which side the hash he holds should be appended for the next step.
/// Refer to `verify_proof()` implementation regarding on this discussion.
pub enum Node<'a> {
    Soft(Cell<'a>),
    Hard(Cell<'a>, Cell<'a>),
}

impl<'a> Node<'a> {
    pub fn new(lc: Cell<'a>, rc: Cell<'a>) -> Self {
        match (&lc, &rc) {
            (&Some(_), &None) => Node::Soft(lc),
            (&None, &Some(_)) => Node::Soft(rc),
            (&Some(_), &Some(_)) => Node::Hard(lc, rc),
            _ => unreachable!("Node::new()"),
        }
    }

    /// Construct `Cell`s by deserializing bytes slice.
    pub fn cells_from_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, Cell<'a>)> {
        match Node::from_bytes(&bytes)? {
            Node::Soft(cell) => Ok((cell, None)),
            Node::Hard(lc, rc) => {
                if right {
                    Ok((rc, lc))
                } else {
                    Ok((lc, rc))
                }
            }
        }
    }

    fn parse_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, usize)> {
        let len_bytes = bytes.len();
        let len_bits = std::mem::size_of::<BitsLen>();
        let offset_hash = if right { 0usize } else { HASH_LEN };
        let range_hash = if right {
            len_bytes - HASH_LEN..len_bytes
        } else {
            0..HASH_LEN
        };
        let start: BitsLen = bytes_to_int(&bytes[offset_hash..offset_hash + len_bits]);
        let end: BitsLen = bytes_to_int(&bytes[offset_hash + len_bits..offset_hash + 2 * len_bits]);
        let offset_bits = nbytes_across(start, end) as usize;
        Ok((
            Some(Unit {
                hash: &bytes[range_hash],
                bits: Bits {
                    path: &bytes
                        [offset_hash + 2 * len_bits..offset_hash + 2 * len_bits + offset_bits],
                    range: start..end,
                },
            }),
            offset_hash + 2 * len_bits + offset_bits,
        ))
    }

    /// Construct `Node` by deserializing bytes slice.
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
            _ => unreachable!("Node::from_bytes()"),
        }
    }

    /// Serialize `Node` into bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Node::Soft(Some(unit)) => {
                Ok([&unit.hash[..], &unit.bits.to_bytes()?, &[0x00]].concat())
            }
            Node::Hard(Some(lu), Some(ru)) => {
                let (lu, ru) = if ru.bits.first() { (lu, ru) } else { (ru, lu) };
                Ok([
                    &lu.hash[..],
                    &lu.bits.to_bytes()?,
                    &ru.bits.to_bytes()?,
                    &ru.hash[..],
                    &[0x01],
                ]
                .concat())
            }
            _ => unreachable!("node.to_bytes()"),
        }
    }
}
