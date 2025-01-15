mod balloc;
mod bitmap;
mod block_group;
mod crc;
mod direntry;
mod error;
mod ext4_inode;
pub mod ext4fs;
mod extent;
mod file;
mod ialloc;
pub mod layout;
mod path;
mod superblock;
mod test;
#[allow(unused)]
pub use super::cache::{BlockCacheManager, BufferCache, Cache, PageCache, PageCacheManager};
pub use crate::drivers::block::BlockDevice;
#[allow(unused)]
pub use crate::fs::fat32::fat_inode::Inode;
pub use ext4_inode::*;

/// Inode相关的常量
/// 根目录的inode号
pub const ROOT_INODE: u32 = 2;
/// 日志inode号
pub const EXT4_JOURNAL_INODE: u32 = 8;
/// 未删除目录的inode号
pub const UNDEL_DIR_INODE: u32 = 6;
/// lost+found目录的inode号
pub const LOST_AND_FOUND_INODE: u32 = 11;
/// 常规文件
pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
/// 提取类型的掩码
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;
/// 提取权限的掩码
pub const EXT4_INODE_MODE_PERM_MASK: u16 = 0x0FFF;
/// 需要修改为动态获取? TODO: check
/// Inode块大小?
pub const EXT4_INODE_BLOCK_SIZE: usize = 512;
/// 经典Inode大小
pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;
/// Inode扩展标志
pub const EXT4_INODE_FLAG_EXTENTS: usize = 0x00080000; /* Inode uses extents */
/// BLock group descriptor flags.
/// 最小块组描述符大小
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
/// 最大块组描述符大小
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;

/// 块大小
pub const BLOCK_SIZE: usize = 2048;

/// 超级块偏移量（当块大小为2048时，实际上大于1024的话都是这个值）
pub const EXT4_SUPERBLOCK_OFFSET_ON_WHEN_BLOCK_SIZE_2048: usize = 1024;
pub const SUPERBLOCK_OFFSET: usize = 1024;

/// 逻辑块号
pub type Ext4Lblk = u32;
/// 物理块号
pub type Ext4Fsblk = u64;

// Extent相关的常量
/// 初始化extent时，允许的最大长度
pub const EXT_INIT_MAX_LEN: u16 = 32768;
/// 未写入的extent的最大长度
pub const EXT_UNWRITTEN_MAX_LEN: u16 = 65535;
/// extent可以涵盖的最大块数
pub const EXT_MAX_BLOCKS: Ext4Lblk = u32::MAX;
/// 表示extent结构体的魔数
pub const EXT4_EXTENT_MAGIC: u16 = 0xF30A;
/// 操作成功
pub const EOK: usize = 0;
