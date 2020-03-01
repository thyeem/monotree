extern crate const_utils;
pub const HASH_LEN: usize = 32;
pub const UNIT_BIT: usize = 4;
pub const NL: usize = 1 << UNIT_BIT;
pub const NBYTE: usize = const_utils::max(NL / 8, 1);
