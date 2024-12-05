pub(crate) mod dir_iter;
mod bitmap;
mod efs;
mod ext4_inode;
mod block_group;
mod superblock;
mod block;
pub mod layout;
pub use ext4_inode::*;

pub use super::cache::{BlockCacheManager, BufferCache, Cache, PageCache, PageCacheManager};
pub use crate::drivers::block::BlockDevice;
use bitmap::Ext4;
pub use efs::EasyFileSystem;
// pub use layout::DiskInodeType;
pub use crate::fs::fat32::fat_inode::Inode;
pub use ext4_rs::fuse_interface::*;

pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;
pub const EXT4_INODE_MODE_PERM_MASK: u16 = 0x0FFF;
pub const EXT4_INODE_BLOCK_SIZE: usize = 512;
pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;
pub const EXT4_INODE_FLAG_EXTENTS: usize = 0x00080000; /* Inode uses extents */
/// BLock group descriptor flags.
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;
