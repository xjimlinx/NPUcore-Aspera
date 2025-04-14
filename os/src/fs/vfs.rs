use crate::fs::cache::BlockCacheManager;
use crate::fs::BlockDevice;
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast_rs::{impl_downcast, DowncastSync};

// 根目录项
use super::directory_tree::ROOT;
use super::ext4::ext4fs::Ext4FileSystem;
use super::ext4::layout::Ext4OSInode;
use super::ext4::ROOT_INODE;
use super::fat32::EasyFileSystem;
use super::file_trait::File;
use super::filesystem::{pre_mount, FS_Type};
use super::inode::{FatInode, OSInode};

// VFS trait, 实现了该trait的文件系统都应该可以直接
// 被 NPUcore 支持
pub trait VFS: DowncastSync {
    // 关闭文件
    fn close(&self) -> () {
        todo!();
    }

    // 读取文件
    fn read(&self) -> Vec<u8> {
        todo!();
    }

    // 写入文件
    fn write(&self, _data: Vec<u8>) -> usize {
        todo!();
    }

    // fn get_super_block(&self) -> SuperBlock {
    //     todo!();
    // }

    fn get_direcotry(&self) -> ROOT {
        todo!();
    }

    fn alloc_blocks(&self, blocks: usize) -> Vec<usize>;

    fn get_filesystem_type(&self) -> FS_Type;
}
impl_downcast!(sync VFS);

impl VFS {
    pub fn open_fs(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<Self> {
        let fs_type = pre_mount();
        match fs_type {
            FS_Type::Fat32 => EasyFileSystem::open(block_device, index_cache_mgr),
            // FS_Type::Ext4 => Ext4FileSystem::open(block_device, index_cache_mgr),
            FS_Type::Ext4 => Arc::new(Ext4FileSystem::open_ext4rs(block_device, index_cache_mgr)),
            FS_Type::Null => panic!("no filesystem found"),
        }
    }
    pub fn root_osinode(vfs: &Arc<dyn VFS>) -> Arc<dyn File> {
        match vfs.get_filesystem_type() {
            FS_Type::Fat32 => OSInode::new(FatInode::root_inode(vfs)),
            FS_Type::Ext4 => {
                let vfs_concrete = Arc::downcast::<Ext4FileSystem>(vfs.clone()).unwrap();
                let root_inode = vfs_concrete.get_inode_ref(ROOT_INODE);
                Ext4OSInode::new(root_inode, vfs_concrete)
            }
            FS_Type::Null => panic!("Null filesystem type does not have a root inode"),
        }
    }
}

// 对不同类型文件系统文件的封装
pub trait VFSFileContent {}

// 对不同类型文件系统目录的封装
pub trait VFSDirEnt {}
