#![allow(unused)]
use core::arch::asm;
use core::ptr::addr_of;

use super::block_group::{Block, Ext4BlockGroup};
use super::direntry::{Ext4DirEntry, Ext4DirSearchResult};
use super::path::path_check;
use super::superblock::SUPERBLOCK_OFFSET;
use super::*;
use super::{superblock::Ext4Superblock, BlockCacheManager, BlockDevice, Cache};
use crate::arch::BLOCK_SZ;
use crate::drivers::BLOCK_DEVICE;
use crate::fs::cache::BufferCache;
use crate::fs::ext4::error::{Errno, Ext4Error};
use crate::fs::file_trait::File;
use crate::fs::inode::InodeTrait;
use crate::fs::vfs::VFS;
use crate::{arch, fs::filesystem::FS_Type};
use alloc::{sync::Arc, vec::Vec};
use layout::Ext4OSInode;
use spin::Mutex;
type SuperBlock = Ext4Superblock;

/// Ext4文件系统对象实例
pub struct Ext4FileSystem {
    /// 块设备
    pub block_device: Arc<dyn BlockDevice>,
    /// 超级块信息
    pub superblock: SuperBlock,
    // /// 块大小
    // pub block_size: usize,
    // /// 每组块的数量
    // pub block_group_count: u32,
    // /// Inode表的起始块号
    // pub inode_table_start_block: u32,
    /// 缓存管理器
    pub cache_mgr: Arc<Mutex<BlockCacheManager>>,
}

impl Ext4FileSystem {
    // 获取根 Inode
    pub fn get_root_inode(&self) -> Arc<dyn File> {
        let root_inode_ref = self.get_inode_ref(ROOT_INODE);
        todo!()
    }
    // Opens and loads an Ext4 from the `block_device`.
    // 针对ext4rs原有的方法的方法，可能需要修改
    pub fn open_ext4rs(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<Mutex<BlockCacheManager>>,
    ) -> Self {
        // 读取超级块
        let block = Block::load_offset(block_device.clone(), 0);
        let superblock: Ext4Superblock = block.read_offset_as(SUPERBLOCK_OFFSET);
        let cache_mgr = index_cache_mgr.clone();
        let ext4fs = Ext4FileSystem {
            block_device,
            superblock,
            cache_mgr,
        };
        ext4fs.test_info();
        ext4fs
    }
    /// with dir result search path offset
    /// # 参数
    /// + path: 路径
    /// + parent_inode_num: 父目录Inode节点号
    /// + create: 是否创建目标文件
    /// + ftype: 文件类型
    /// + name_off: 路径中当前处理部分的偏移量,用来记录已经处理的路径部分的偏移量
    /// # 返回值
    /// + 目标文件的Inode节点号
    pub fn generic_open(
        &self,
        path: &str,
        parent_inode_num: &mut u32,
        create: bool,
        ftype: u16,
        name_off: &mut u32,
    ) -> Result<u32, isize> {
        let mut is_goal = false;

        let mut parent = parent_inode_num;

        let mut search_path = path;

        let mut dir_search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        loop {
            // 路径可能包含多个斜杠
            // 每遇到一个就跳过一个，并将偏移量 name_off 加 1
            while search_path.starts_with('/') {
                *name_off += 1; // Skip the slash
                search_path = &search_path[1..];
            }
            // 使用 path_check 检查当前路径，并返回当前部分的长度 len
            let len = path_check(search_path, &mut is_goal);

            // 路径中的当前部分
            // 比如usr
            // 或者lib
            // 亦或者1.txt之类的
            let current_path = &search_path[..len];

            // 路径长度若为 0 或者路径为空
            // 退出
            if len == 0 || search_path.is_empty() {
                break;
            }

            search_path = &search_path[len..];

            // 使用dir_find_entry查找当前父目录下是否存在current_path对应的文件或者目录
            let r = self.dir_find_entry(*parent, current_path, &mut dir_search_result);
            match r {
                Ok(_) => {
                    println!(
                        "[kernel generic_open] Find in parent {:x?} r {:?} name {:?}",
                        parent, r, current_path
                    );
                }
                Err(errno) => {
                    println!("[failed in ext4fs generic_open function!] {:?}", errno)
                }
            }

            // 查找失败
            if let Err(e) = r {
                if e.error() != Errno::ENOENT || !create {
                    println!("[kernel generic_open] No such file or directory");
                }

                // 创建新 inode
                let mut inode_mode = 0;
                if is_goal {
                    inode_mode = ftype;
                } else {
                    inode_mode = InodeFileType::S_IFDIR.bits();
                }

                let new_inode_ref = self.create(*parent, current_path, inode_mode)?;

                // Update parent the new inode
                *parent = new_inode_ref.inode_num;

                // Update dir_search_result to reflect the new inode
                dir_search_result.dentry.inode = new_inode_ref.inode_num;

                continue;
            }

            if is_goal {
                break;
            } else {
                // 更新父目录Inode节点号
                *parent = dir_search_result.dentry.inode;
            }
            *name_off += len as u32;
        }

        // 下面的两行好像一模一样？？？？
        // 目标文件已找到时退出
        // 返回找到的inode号
        if is_goal {
            return Ok(dir_search_result.dentry.inode);
        }

        Ok(dir_search_result.dentry.inode)
    }
    pub fn open_old(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<Mutex<BlockCacheManager>>,
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
                    block_device,
                    /// 超级块信息
                    superblock: super_block.clone(),
                    // /// 块大小
                    // block_size: super_block.block_size() as usize,
                    // /// 每组块的数量
                    // block_group_count: super_block.blocks_per_group(),
                    // /// Inode表的起始块号
                    // inode_table_start_block: super_block.get_inode_table_start(),
                    /// 缓存管理器
                    cache_mgr: ext4_cache_mgr,
                };
                ext4fs.test_info();
                Arc::new(ext4fs)
            })
    }
    pub fn alloc_blocks(&self, blocks: usize) -> Vec<usize> {
        todo!()
    }
    fn root_inode(&self) -> Arc<dyn InodeTrait> {
        todo!();
    }
    #[allow(unused)]
    pub fn dir_mk(&self, path: &str) -> Result<usize, isize> {
        let mut nameoff = 0;

        let filetype = InodeFileType::S_IFDIR;

        // todo get this path's parent

        // start from root
        let mut parent = ROOT_INODE;

        let r = self.generic_open(path, &mut parent, true, filetype.bits(), &mut nameoff);
        Ok(EOK)
    }
    pub fn unlink(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
    ) -> Result<usize, isize> {
        self.dir_remove_entry(parent, name)?;

        let is_dir = child.inode.is_dir();

        self.ialloc_free_inode(child.inode_num, is_dir);

        Ok(EOK)
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
        // inode表长
        let inode_size = self.superblock.inode_size();
        let inodes_per_grp = self.superblock.inodes_per_group;
        let ino_table_len = (inodes_per_grp as usize) * (inode_size as usize) / BLOCK_SIZE;
        self.get_block_group(blk_grp_idx).dump_block_group_info(
            blk_grp_idx,
            blk_per_grp,
            ino_table_len,
        );
    }
    fn test_info(&self) {
        self.superblock.dump_info();
        self.print_block_group(0);
        self.print_block_group(1);
        self.print_block_group(2);
        self.print_block_group(3);
        // 尝试比较超级块内容
        assert!(self.superblock == Ext4FileSystem::get_superblock_test(BLOCK_DEVICE.clone()));
        // self.test_get_file("remove.lua");
        // self.test_get_file("/remove.lua");
        // self.test_get_file("/busybox_cmd.txt");
        // println!("Finish the test");
    }
}

impl VFS for Ext4FileSystem {
    fn alloc_blocks(&self, blocks: usize) -> Vec<usize> {
        self.alloc_blocks(blocks)
    }
    fn get_filesystem_type(&self) -> FS_Type {
        FS_Type::Ext4
    }
}