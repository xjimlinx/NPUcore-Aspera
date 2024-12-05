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
pub struct FSInfo {
    /// Value 0x41615252. This lead signature is used to validate that this is in fact an FSInfo sector.
    lead_sig: u32,
    /// The reserved area should be empty.
    reserved1: [u8; 480],
    /// Value 0x61417272. Another signature that is more localized in the sector to the location of the fields that are used.
    struc_sig: u32,
    /// Contains the last known free cluster count on the volume. If the
    /// value is 0xFFFFFFFF, then the free count is unknown and must be
    /// computed. Any other value can be used, but is not necessarily
    /// correct. It should be range checked at least to make sure it is <=
    /// volume cluster count.
    free_count: u32,
    /// This is a hint for the FAT driver. It indicates the cluster number at
    /// which the driver should start looking for free clusters. Because a
    /// FAT32 FAT is large, it can be rather time consuming if there are a
    /// lot of allocated clusters at the start of the FAT and the driver starts
    /// looking for a free cluster starting at cluster 2. Typically this value is
    /// set to the last cluster number that the driver allocated. If the value is
    /// 0xFFFFFFFF, then there is no hint and the driver should start
    /// looking at cluster 2. Any other value can be used, but should be
    /// checked first to make sure it is a valid cluster number for the
    /// volume.
    nxt_free: u32,
    reserved2: [u8; 12],
    /// Value 0xAA550000.
    /// This trail signature is used to validate that this is in fact an FSInfo sector.
    /// Note that the high 2 bytes of this value which go into the bytes at offsets 510 and 511
    /// match the signature bytes used at the same offsets in sector 0.
    trail_sig: u32,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DiskInodeType {
    File,
    Directory,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Ext4DiskInodeType {
}

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
