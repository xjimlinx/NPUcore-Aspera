#![allow(unused)]
use core::arch::asm;
use core::ptr::addr_of;

use super::{superblock::Ext4Superblock, BlockCacheManager, BlockDevice, Cache};
use crate::fs::cache::BufferCache;
use crate::fs::vfs::VFS;
use crate::{arch, fs::filesystem::FS_Type};
use alloc::{sync::Arc, vec::Vec};
type SuperBlock = Ext4Superblock;

/// Ext4文件系统对象实例
pub struct Ext4FileSystem {
    /// 块设备
    pub block_device: Arc<dyn BlockDevice>,
    /// 超级块信息
    pub superblock: SuperBlock,
    /// 块大小
    pub block_size: usize,
    /// 每组块的数量
    pub block_group_count: u32,
    /// Inode表的起始块号
    pub inode_table_start_block: u32,
    /// 缓存管理器
    pub cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
}

impl Ext4FileSystem {
    pub fn open(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<Self> {
        let ext4_cache_mgr = index_cache_mgr.clone();
        index_cache_mgr
            .lock()
            // 获取第0块的缓存
            .get_block_cache(0, &block_device)
            .lock()
            // 获取超级块
            .read(1024, |super_block: &SuperBlock| {
                // 创建ext4实例
                let ext4fs = Self {
                    block_device: block_device,
                    /// 超级块信息
                    superblock: super_block.clone(),
                    /// 块大小
                    block_size: super_block.block_size() as usize,
                    /// 每组块的数量
                    block_group_count: super_block.blocks_per_group(),
                    /// Inode表的起始块号
                    inode_table_start_block: super_block.get_inode_table_start(),
                    /// 缓存管理器
                    cache_mgr: ext4_cache_mgr,
                };
                ext4fs.superblock.dump_info();
                Arc::new(ext4fs)
            })
    }
    pub fn alloc_blocks(&self, blocks: usize) -> Vec<usize> {
        todo!()
    }
}

impl VFS for Ext4FileSystem {
    fn alloc_blocks(&self, blocks: usize) -> Vec<usize> {
        self.alloc_blocks(blocks)
    }
    fn open(
        &self,
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<Self>
    where
        Self: Sized,
    {
        self.open(block_device, index_cache_mgr)
    }
    fn get_filesystem_type(&self) -> FS_Type {
        FS_Type::Ext4
    }
}
