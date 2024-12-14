#![allow(unused)]
use crate::{copy_from_name1, copy_to_name1, lang_items::Bytes};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use core::{
    convert::TryInto,
    fmt::Debug,
    mem,
    ptr::{addr_of, addr_of_mut},
};

// 可能后续会用到？
pub enum ExtType {
    Ext2,
    Ext3,
    Ext4,
}

#[derive(Debug, Clone)]
#[repr(C)]
/// *On-disk* data structure.
/// The direct creation/storage of this struct should avoided since the size of reserved area is rather big.
pub struct FSInfo {}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DiskInodeType {
    File,
    Directory,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Ext4DiskInodeType {}

#[repr(align(8))]
pub union EXT4DirEnt {
    pub entry: [u8; 8],
    pub empty: [u8; 32],
}

impl Debug for EXT4DirEnt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe {
            let entry = &self.entry;
            let empty = &self.empty;
            let mut s = String::new();
            s.push_str("EXT4DirEnt {\n");
            s.push_str(&format!("  entry: {:?},\n", entry));
            s.push_str(&format!("  empty: {:?},\n", empty));
            s.push_str("}");
            write!(f, "{}", s)
        }
    }
}

impl EXT4DirEnt {
    // TODO:
}
