mod cache;
mod dev;
pub mod directory_tree;
mod ext4;
mod fat32;
pub mod file_trait;
mod filesystem;
mod layout;
pub mod poll;
#[cfg(feature = "swap")]
pub mod swap;
// Xein add this
mod dirent;
mod file_descriptor;
mod inode;
mod vfs;

pub use self::dev::{hwclock::*, pipe::*, socket::*};
pub use file_descriptor::{FdTable, FileDescriptor};
use filesystem::FS_Type;

pub use self::dirent::*;
pub use self::fat32::DiskInodeType;
pub use self::layout::*;
pub use crate::drivers::block::BlockDevice;
// TODO: ext4 support
use self::cache::PageCache;
use alloc::{string::String, sync::Arc};
use lazy_static::*;

// 当前的文件系统类型
pub const CURR_FS_TYPE: FS_Type = FS_Type::Fat32;

lazy_static! {
    // 根目录的 FileDescriptor
    pub static ref ROOT_FD: Arc<FileDescriptor> = Arc::new(FileDescriptor::new(
        false,
        false,
        self::directory_tree::ROOT
            .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
            .unwrap()
    ));
}
