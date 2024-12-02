mod cache;
mod dev;
pub mod directory_tree;
mod fat32;
mod ext4;
pub mod file_trait;
mod filesystem;
mod layout;
pub mod poll;
#[cfg(feature = "swap")]
pub mod swap;
// Xein add this
pub mod inode;
pub mod file_descriptor;
mod vfs;
mod dirent;

pub use file_descriptor::{FileDescriptor, FdTable};
pub use self::dev::{hwclock::*, null::*, pipe::*, socket::*, tty::*, zero::*};

pub use self::dirent::*;
pub use self::layout::*;
pub use crate::drivers::block::BlockDevice;
pub use self::fat32::DiskInodeType;
// TODO: ext4 support
use self::cache::PageCache;
use alloc::{
    string::String,
    sync::Arc,
};
use lazy_static::*;

lazy_static! {
    pub static ref ROOT_FD: Arc<FileDescriptor> = Arc::new(FileDescriptor::new(
        false,
        false,
        self::directory_tree::ROOT
            .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
            .unwrap()
    ));
}