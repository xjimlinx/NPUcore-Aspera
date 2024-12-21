mod block_group;
mod crc;
mod direntry;
mod error;
mod ext4_inode;
pub mod ext4fs;
mod extent;
mod file;
pub mod layout;
mod superblock;
#[allow(unused)]
pub use super::cache::{BlockCacheManager, BufferCache, Cache, PageCache, PageCacheManager};
pub use crate::drivers::block::BlockDevice;
#[allow(unused)]
pub use crate::fs::fat32::fat_inode::Inode;
pub use ext4_inode::*;

pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;
pub const EXT4_INODE_MODE_PERM_MASK: u16 = 0x0FFF;
pub const EXT4_INODE_BLOCK_SIZE: usize = 512;
pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;
pub const EXT4_INODE_FLAG_EXTENTS: usize = 0x00080000; /* Inode uses extents */
/// BLock group descriptor flags.
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;

pub const BLOCK_SIZE: usize = 2048;
pub const EXT4_SUPERBLOCK_OFFSET_ON_WHEN_BLOCK_SIZE_2048: usize = 1024;
