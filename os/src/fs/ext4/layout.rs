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
