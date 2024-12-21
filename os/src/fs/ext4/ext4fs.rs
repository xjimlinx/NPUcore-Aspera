#![allow(unused)]
use core::arch::asm;
use core::ptr::addr_of;

use super::block_group::{Block, Ext4BlockGroup};
use super::superblock::SUPERBLOCK_OFFSET;
use super::{superblock::Ext4Superblock, BlockCacheManager, BlockDevice, Cache};
use crate::drivers::BLOCK_DEVICE;
use crate::fs::cache::BufferCache;
use crate::fs::inode::InodeTrait;
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
        // 块缓存管理器
        // 读取的数据会被缓存，也就是说放在内存中
        // 这样下次再读取的时候就不用再从磁盘中读取了
        // 速度会快很多
        let ext4_cache_mgr = index_cache_mgr.clone();
        index_cache_mgr
            .lock()
            // 获取第0块的缓存
            .get_block_cache(0, &block_device)
            .lock()
            // 获取超级块
            .read(SUPERBLOCK_OFFSET, |super_block: &SuperBlock| {
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
                // ext4fs.print_block_group(0);
                // 尝试比较超级块内容
                assert!(
                    ext4fs.superblock == Ext4FileSystem::get_superblock_test(BLOCK_DEVICE.clone())
                );
                Arc::new(ext4fs)
            })
    }
    pub fn alloc_blocks(&self, blocks: usize) -> Vec<usize> {
        todo!()
    }
    fn root_inode(&self) -> Arc<dyn InodeTrait> {
        todo!();
    }
}

impl Ext4FileSystem {
    pub fn get_superblock_test(block_device: Arc<dyn BlockDevice>) -> Ext4Superblock {
        let superblock_pre = Block::load_offset(block_device, 0);
        let superblock: Ext4Superblock = superblock_pre.read_offset_as(1024);
        superblock
    }

    pub fn get_superblock(&self) -> Ext4Superblock {
        self.superblock
    }

    pub fn get_block_group(&self, blk_grp_idx: usize) -> Ext4BlockGroup {
        let block_device = self.block_device.clone();
        Ext4BlockGroup::load_new(block_device, &self.superblock, blk_grp_idx)
    }

    pub fn print_block_group(&self, blk_grp_idx: usize) {
        let blk_per_grp = self.superblock.blocks_per_group();
        let blk_per_grp = blk_per_grp as usize;
        self.get_block_group(0)
            .dump_block_group_info(0, blk_per_grp);
    }
    fn test_info(&self) {}
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
    // fn root_inode(&self) -> Arc<dyn InodeTrait> {
    //     todo!();
    // }
}
