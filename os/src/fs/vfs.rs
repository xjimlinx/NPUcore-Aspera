use crate::fs::cache::{BlockCacheManager, Cache};
use crate::fs::BlockDevice;
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast_rs::{impl_downcast, DowncastSync};

// 根目录项
use super::directory_tree::ROOT;
use super::ext4::{Ext4Inode, Inode};
use super::filesystem::FS_Type;
use super::inode::InodeTrait;

// VFS trait, 实现了该trait的文件系统都应该可以直接
// 被 NPUcore 支持
pub trait VFS: DowncastSync {
    // 打开文件
    // 网上找到的资料是通过dentry
    // 找到对应的inode
    // 然后获取文件操作集
    // 调用具体文件系统的open操作来完成
    fn open(
        &self,
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<Self>
    where
        Self: Sized;

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

    // fn root_inode(&self) -> Arc<dyn InodeTrait>;
}
impl_downcast!(sync VFS);

impl VFS {
    pub fn root_inode(efs: &Arc<dyn VFS>) -> Arc<dyn InodeTrait> {
        match efs.get_filesystem_type() {
            FS_Type::Ext4 => Ext4Inode::root_inode(efs),
            FS_Type::Fat32 => Inode::root_inode(efs),
            FS_Type::Null => panic!("Null filesystem type does not have a root inode"),
        }
    }
}

// 对不同类型文件系统文件的封装
pub trait VFSFileContent {}

// pub struct SuperBlock {
//     // 文件系统魔数
//     s_magic: u32,
//     // 指向super_operations结构体的指针
//     s_op: Option<u32>,
//     // 指向与特定文件系统相关的私有数据结构的指针
//     s_fs_info: Option<u32>,
//     // 根目录 dentry
//     s_root: ROOT,
//     // 指向 文件系统类型结构体的指针
//     s_type: Option<u32>,

// }

// 对不同类型文件系统目录的封装
pub trait VFSDirEnt {}
