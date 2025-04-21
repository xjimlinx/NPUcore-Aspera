#![allow(unused)]
use super::DiskInodeType;
use crate::fs::fat32::dir_iter::*;
use crate::fs::fat32::layout::{FATDirEnt, FATDiskInodeType, FATLongDirEnt, FATShortDirEnt};
use crate::fs::fat32::EasyFileSystem;
use crate::fs::fat32::{BlockCacheManager, Cache, PageCache, PageCacheManager};
use crate::fs::inode::InodeLock;
use crate::fs::inode::InodeTime;
use crate::fs::inode::InodeTrait;
use crate::fs::vfs::VFSFileContent;
use crate::fs::vfs::VFS;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::convert::TryInto;
use core::ops::Mul;
use core::panic;
use downcast_rs::{Downcast, DowncastSync};
use spin::*;

/// 文件内容 FileContent
pub struct FileContent {
    /// 对于FAT32，size 需要从FAT计算
    /// 所以需要遍历FAT32来获取size
    size: u32,
    /// 簇列表
    clus_list: Vec<u32>,
    /// 如果该文件是个目录，那么
    /// hint 会记录最后一个目录项的位置（第一个字节为0x00）
    hint: u32,
}

impl VFSFileContent for FileContent {}

impl FileContent {
    /// 获取文件大小
    /// # 返回值
    /// 文件大小
    #[inline(always)]
    pub fn get_file_size(&self) -> u32 {
        self.size
    }
}
macro_rules! div_ceil {
    ($mult:expr,$deno:expr) => {
        ($mult - 1 + $deno) / $deno
    };
}

/* *ClusLi was DiskInode*
 * Even old New York, was New Amsterdam...
 * Why they changed it I can't say.
 * People just like it better that way.*/
/// The functionality of ClusLi & Inode can be merged.
/// The struct for file information
/// 上面这段描述可能是来自最早的文件系统实现，我也不知道怎么翻译
pub struct FatInode {
    /// inode 锁: for normal operation
    inode_lock: RwLock<InodeLock>,
    /// 文件内容
    file_content: RwLock<FileContent>,
    /// 与该Inode对应的文件缓存管理器
    file_cache_mgr: PageCacheManager,
    /// 文件类型
    file_type: Mutex<DiskInodeType>,
    /// 父目录的inode
    parent_dir: Mutex<Option<(Arc<Self>, u32)>>,
    /// 文件系统实例
    fs: Arc<EasyFileSystem>,
    /// 保存时间的结构体
    time: Mutex<InodeTime>,
    /// Info Inode to delete file content
    deleted: Mutex<bool>,
}

impl Drop for FatInode {
    /// 在删除该inode之前，文件信息需要写回父目录
    fn drop(&mut self) {
        if *self.deleted.lock() {
            // Clear size
            let mut lock = self.file_content.write();
            let length = lock.clus_list.len();
            self.dealloc_clus(&mut lock, length);
        } else {
            if self.parent_dir.lock().is_none() {
                return;
            }
            let par_dir_lock = self.parent_dir.lock();
            let (parent_dir, offset) = par_dir_lock.as_ref().unwrap();

            let par_inode_lock = parent_dir.write();
            let dir_ent = parent_dir.get_dir_ent(&par_inode_lock, *offset).unwrap();
            let mut short_dir_ent = *dir_ent.get_short_ent().unwrap();
            // Modify size
            short_dir_ent.file_size = self.get_file_size();
            // Modify fst cluster
            short_dir_ent.set_fst_clus(
                self.get_first_clus_lock(&self.file_content.read())
                    .unwrap_or(0),
            );
            // Modify time
            // todo!
            log::debug!("[Inode drop]: new_ent: {:?}", short_dir_ent);
            // Write back
            parent_dir
                .set_dir_ent(&par_inode_lock, *offset, dir_ent)
                .unwrap();
        }
    }
}

/// 构造函数
impl FatInode {
    /// Inode 的构造函数
    /// # 参数
    /// + `fst_clus`: 文件的第一个簇
    /// + `file_type`: 文件类型
    /// + `size`: NOTE: the `size` field should be set to `None` for a directory
    /// + `parent_dir`: 父目录
    /// + `fs`: 文件系统实例
    /// # 返回值
    /// 指向inode的指针
    pub fn new(
        fst_clus: u32,
        file_type: DiskInodeType,
        size: Option<u32>,
        parent_dir: Option<(Arc<Self>, u32)>,
        fs: Arc<EasyFileSystem>,
    ) -> Arc<Self> {
        let file_cache_mgr = PageCacheManager::new();
        let clus_list = match fst_clus {
            0 => Vec::new(),
            _ => fs.fat.get_all_clus_num(fst_clus, &fs.block_device),
        };

        let size = size.unwrap_or_else(|| clus_list.len() as u32 * fs.clus_size());
        let hint = 0;

        let file_content = RwLock::new(FileContent {
            size,
            clus_list,
            hint,
        });
        let parent_dir = Mutex::new(parent_dir);
        let time = InodeTime::new();
        let inode = Arc::new(FatInode {
            inode_lock: RwLock::new(InodeLock {}),
            file_content,
            file_cache_mgr,
            file_type: Mutex::new(file_type),
            parent_dir,
            fs,
            time: Mutex::new(time),
            deleted: Mutex::new(false),
        });

        // 初始化 hint
        if file_type == DiskInodeType::Directory {
            inode.set_hint();
        }
        inode
    }
}

/// 基本功能
impl FatInode {
    /// 获取第一个簇
    /// # 参数
    /// + `lock`: 目标文件内容的锁
    /// # 返回值
    /// 如果簇列表非空，会返回第一个簇
    /// 否则返回空
    fn get_first_clus_lock(&self, lock: &RwLockReadGuard<FileContent>) -> Option<u32> {
        // 获取簇列表
        let clus_list = &lock.clus_list;
        // 非空返回第一个簇号
        if !clus_list.is_empty() {
            Some(clus_list[0])
        } else {
            None
        }
    }
    /// 获取根据大小向上取整后所需的簇数
    /// # 返回值
    /// The number representing the number of clusters
    fn total_clus(&self, size: u32) -> u32 {
        //size.div_ceil(self.fs.clus_size())
        let clus_sz = self.fs.clus_size();
        div_ceil!(size, clus_sz)
        //(size - 1 + clus_sz) / clus_sz
    }

    /// 获取由给定缓存索引表示的块ID(实际存入起始块号)列表
    /// # 参数
    /// + `clus_list`: 簇列表
    /// + `inner_cache_id`: Index of T's file caches (每个cache有4096字节)
    /// 即cache的索引
    /// # 返回值
    /// 块号列表
    /// # 补充说明
    /// 第一次读这个函数实现（以及调用他的相关函数栈），感觉有点抽象，
    /// 因为BLOCK_SIZE固定大小为2048B
    /// 每扇区字节数也固定为2048B，
    /// 然后一簇扇区数量就为1！！！
    /// 然后感觉有些数据就没必要处理
    /// 总之先这样吧，说不定适用于fat扇区数不为2048的情况
    /// 然后还有另一个问题，
    /// 就是每次会存两个块（扇区）
    /// 包含对应的第一个块（扇区）
    /// 以及第二个块（扇区），即第一个块（扇区）的下一个块（扇区）
    fn get_neighboring_sec(&self, clus_list: &Vec<u32>, inner_cache_id: usize) -> Vec<usize> {
        // 获取每簇包含扇区数量(实际为1)
        let sec_per_clus = self.fs.sec_per_clus as usize;
        // 获取每扇区字节数(实际为2048)
        let byts_per_sec = self.fs.byts_per_sec as usize;
        // 获取每个缓存页面扇区数（实际为2，也就是说每个页面包含两个簇）
        let sec_per_cache = PageCacheManager::CACHE_SZ / byts_per_sec;
        // 计算当前缓存页面应该读取的第一个扇区编号
        let mut sec_id = inner_cache_id * sec_per_cache;
        // 初始化用于存储缓存页面需要加载的扇区块号集合
        let mut block_ids = Vec::with_capacity(sec_per_cache);
        // 遍历页面内每个扇区
        for _ in 0..sec_per_cache {
            // 计算簇号
            // 公式为：
            // 扇区编号 / 每簇扇区数
            let cluster_id = sec_id / sec_per_clus;
            // 若cluster_id大于等于簇列表长度，则跳出循环
            if cluster_id >= clus_list.len() {
                break;
            }
            // 计算簇内偏移量
            // 公式为：
            // 扇区编号 % 每簇扇区数
            let offset = sec_id % sec_per_clus;
            // 获取当前簇的起始（第一个）扇区号
            let start_block_id = self.fs.first_sector_of_cluster(clus_list[cluster_id]) as usize;
            // 存入块号
            block_ids.push(start_block_id + offset);
            // 更新扇区id号，进入下一次循环
            sec_id += 1;
        }
        // 返回块号列表
        block_ids
    }

    /// 打开根目录
    /// # 参数
    /// + `efs`: 指向文件系统实例的指针
    /// # 返回值
    /// 指向Inode的指针
    pub fn root_inode(efs: &Arc<dyn VFS>) -> Arc<Self> {
        let efs_concrete = Arc::downcast::<EasyFileSystem>(efs.clone()).unwrap();
        let rt_clus = efs_concrete.root_clus;
        Self::new(
            rt_clus,
            DiskInodeType::Directory,
            None,
            None,
            Arc::clone(&efs_concrete),
        )
    }
}

/// File Content Operation
/// 文件内容操作相关方法
impl FatInode {
    /// 分配需要的簇
    /// 需要尽可能多的分配簇，然后追加到`lock`中的`clus_list`中
    /// # 参数
    /// + `lock`: 目标文件内容（锁）
    /// + `alloc_num`: 需要分配的簇数
    fn alloc_clus(&self, lock: &mut RwLockWriteGuard<FileContent>, alloc_num: usize) {
        let clus_list = &mut lock.clus_list;
        let mut new_clus_list = self.fs.fat.alloc(
            &self.fs.block_device,
            alloc_num,
            clus_list.last().map(|clus| *clus),
        );
        clus_list.append(&mut new_clus_list);
    }
    /// 从lock中的clus_list释放一定数量的簇
    /// 当要释放的数量超过可用数量时，`clus_list` 会被清空
    /// # 参数
    /// + `lock`: 目标文件内容（锁）
    /// + `dealloc_num`: 需要释放的簇数
    fn dealloc_clus(&self, lock: &mut RwLockWriteGuard<FileContent>, dealloc_num: usize) {
        let clus_list = &mut lock.clus_list;
        let dealloc_num = dealloc_num.min(clus_list.len());
        let mut dealloc_list = Vec::<u32>::with_capacity(dealloc_num);
        for _ in 0..dealloc_num {
            dealloc_list.push(clus_list.pop().unwrap());
        }
        self.fs.fat.free(
            &self.fs.block_device,
            dealloc_list,
            clus_list.last().map(|x| *x),
        );
    }
    fn clear_at_block_cache_lock(
        &self,
        _inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: usize,
        length: usize,
    ) -> usize {
        let mut start = offset;
        let end = offset + length;

        let mut start_cache = start / PageCacheManager::CACHE_SZ;
        let mut write_size = 0;
        loop {
            // calculate end of current block
            let mut end_current_block =
                (start / PageCacheManager::CACHE_SZ + 1) * PageCacheManager::CACHE_SZ;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let lock = self.file_content.read();
            let block_write_size = end_current_block - start;
            self.file_cache_mgr
                .get_cache(
                    start_cache,
                    || -> Vec<usize> { self.get_neighboring_sec(&lock.clus_list, start_cache) },
                    &self.fs.block_device,
                )
                .lock()
                // I know hardcoding 4096 in is bad, but I can't get around Rust's syntax checking...
                .modify(0, |data_block: &mut [u8; 4096]| {
                    let dst = &mut data_block[start % PageCacheManager::CACHE_SZ
                        ..start % PageCacheManager::CACHE_SZ + block_write_size];
                    dst.fill(0);
                });
            drop(lock);
            write_size += block_write_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_cache += 1;
            start = end_current_block;
        }
        write_size
    }
}

/// Directory Operation
impl FatInode {
    /// A Constructor for `DirIter`(See `dir_iter.rs/DirIter` for details).
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `offset`: The start offset of iterator
    /// + `mode`: The mode of iterator
    /// + `forward`: The direction of the iterator iteration
    /// # Return Value
    /// Pointer to iterator
    fn dir_iter<'a, 'b>(
        &'a self,
        inode_lock: &'a RwLockWriteGuard<'b, InodeLock>,
        offset: Option<u32>,
        mode: DirIterMode,
        forward: bool,
    ) -> DirIter<'a, 'b> {
        debug_assert!(self.is_dir(), "this isn't a directory");
        DirIter::new(inode_lock, offset, mode, forward, self)
    }
    /// Set the offset of the last entry in the directory file(first byte is 0x00) to hint
    fn set_hint(&self) {
        let inode_lock = self.write();
        let mut iter = self.dir_iter(&inode_lock, None, DirIterMode::Enum, FORWARD);
        loop {
            let dir_ent = iter.next();
            if dir_ent.is_none() {
                // Means iter reachs the end of file
                let mut lock = self.file_content.write();
                lock.hint = lock.size;
                return;
            }
            let dir_ent = dir_ent.unwrap();
            if dir_ent.last_and_unused() {
                let mut lock = self.file_content.write();
                lock.hint = iter.get_offset().unwrap();
                return;
            }
        }
    }
    /// Check if current file is an empty directory
    /// If a file contains only "." and "..", we consider it to be an empty directory
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// # Return Value
    /// Bool result
    /// Expand directory file's size(a cluster)
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// # Return Value
    /// Default is Ok
    fn expand_dir_size(&self, inode_lock: &RwLockWriteGuard<InodeLock>) -> Result<(), ()> {
        let diff_size = self.fs.clus_size();
        self.modify_size_lock(inode_lock, diff_size as isize, false);
        Ok(())
    }
    /// Shrink directory file's size to fit `hint`.
    /// For directory files, it has at least one cluster, which should be noted.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// # Return Value
    /// Default is Ok
    fn shrink_dir_size(&self, inode_lock: &RwLockWriteGuard<InodeLock>) -> Result<(), ()> {
        let lock = self.file_content.read();
        let new_size = div_ceil!(lock.hint, self.fs.clus_size())
            .mul(self.fs.clus_size())
            .max(self.fs.clus_size());
        /*lock
        .hint
        .div_ceil(self.fs.clus_size())
        .mul(self.fs.clus_size())
        // For directory file, it has at least one cluster
        .max(self.fs.clus_size());*/
        let diff_size = new_size as isize - lock.size as isize;
        drop(lock);
        self.modify_size_lock(inode_lock, diff_size as isize, false);
        Ok(())
    }
    /// Allocate directory entries required for new file.
    /// The allocated directory entries is a contiguous segment.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `alloc_num`: Required number of directory entries
    /// # Return Value
    /// It will return lock anyway.
    /// If successful, it will also return the offset of the last allocated entry.
    fn alloc_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        alloc_num: usize,
    ) -> Result<u32, ()> {
        let offset = self.file_content.read().hint;
        let mut iter = self.dir_iter(inode_lock, None, DirIterMode::Enum, FORWARD);
        iter.set_iter_offset(offset);
        let mut found_free_dir_ent = 0;
        loop {
            let dir_ent = iter.next();
            if dir_ent.is_none() {
                if self.expand_dir_size(&mut iter.inode_lock).is_err() {
                    log::error!("[alloc_dir_ent]expand directory size error");
                    return Err(());
                }
                continue;
            }
            // We assume that all entries after `hint` are valid
            // That's why we use `hint`. It can reduce the cost of iterating over used entries
            found_free_dir_ent += 1;
            if found_free_dir_ent >= alloc_num {
                let offset = iter.get_offset().unwrap();
                // Set hint
                // Set next entry to last_and_unused
                if iter.next().is_some() {
                    iter.write_to_current_ent(&FATDirEnt::unused_and_last_entry());
                    let mut lock = self.file_content.write();
                    lock.hint = iter.get_offset().unwrap();
                } else {
                    // Means iter reachs the end of file
                    let mut lock = self.file_content.write();
                    lock.hint = lock.size;
                }
                return Ok(offset);
            }
        }
    }
    /// Get a directory entries.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `offset`: The offset of entry
    /// # Return Value
    /// If successful, it will return a `FATDirEnt`(See `layout.rs/FATDirEnt` for details)
    /// Otherwise, it will return Error
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    fn get_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
    ) -> Result<FATDirEnt, ()> {
        let mut dir_ent = FATDirEnt::empty();
        if self.read_at_block_cache_wlock(inode_lock, offset as usize, dir_ent.as_bytes_mut())
            != dir_ent.as_bytes().len()
        {
            return Err(());
        }
        Ok(dir_ent)
    }
    /// Write the directory entry back to the file contents.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `offset`: The offset of file to write
    /// + `dir_ent`: The buffer needs to write back
    /// # Return Value
    /// If successful, it will return Ok.
    /// Otherwise, it will return Error.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    fn set_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
        dir_ent: FATDirEnt,
    ) -> Result<(), ()> {
        if self.write_at_block_cache_lock(inode_lock, offset as usize, dir_ent.as_bytes())
            != dir_ent.as_bytes().len()
        {
            return Err(());
        }
        Ok(())
    }
    /// Get directory entries, including short and long entries
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `offset`: The offset of short entry
    /// # Return Value
    /// If successful, it returns a pair of a short directory entry and a long directory entry list.
    /// Otherwise, it will return Error.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    fn get_all_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
    ) -> Result<(FATShortDirEnt, Vec<FATLongDirEnt>), ()> {
        debug_assert!(self.is_dir());
        let short_ent: FATShortDirEnt;
        let mut long_ents = Vec::<FATLongDirEnt>::with_capacity(5);

        let mut iter = self.dir_iter(inode_lock, Some(offset), DirIterMode::Enum, BACKWARD);

        short_ent = *iter.current_clone().unwrap().get_short_ent().unwrap();

        // Check if this directory entry is only a short directory entry
        {
            let dir_ent = iter.next();
            // First directory entry
            if dir_ent.is_none() {
                return Ok((short_ent, long_ents));
            }
            let dir_ent = dir_ent.unwrap();
            // Short directory entry
            if !dir_ent.is_long() {
                return Ok((short_ent, long_ents));
            }
        }

        // Get long dir_ents
        loop {
            let dir_ent = iter.current_clone();
            if dir_ent.is_none() {
                return Err(());
            }
            let dir_ent = dir_ent.unwrap();
            if dir_ent.get_long_ent().is_none() {
                return Err(());
            }
            long_ents.push(*dir_ent.get_long_ent().unwrap());
            if dir_ent.is_last_long_dir_ent() {
                break;
            }
        }
        Ok((short_ent, long_ents))
    }
    /// Delete derectory entries, including short and long entries.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `offset`: The offset of short entry
    /// # Return Value
    /// If successful, it will return Ok.
    /// Otherwise, it will return Error.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock.
    fn delete_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
    ) -> Result<(), ()> {
        debug_assert!(self.is_dir());
        let mut iter = self.dir_iter(inode_lock, Some(offset), DirIterMode::Used, BACKWARD);

        iter.write_to_current_ent(&FATDirEnt::unused_not_last_entry());
        // Check if this directory entry is only a short directory entry
        {
            let dir_ent = iter.next();
            // First directory entry
            if dir_ent.is_none() {
                return Ok(());
            }
            let dir_ent = dir_ent.unwrap();
            // Short directory entry
            if !dir_ent.is_long() {
                return Ok(());
            }
        }
        // Remove long dir_ents
        loop {
            let dir_ent = iter.current_clone();
            if dir_ent.is_none() {
                return Err(());
            }
            let dir_ent = dir_ent.unwrap();
            if !dir_ent.is_long() {
                return Err(());
            }
            iter.write_to_current_ent(&FATDirEnt::unused_not_last_entry());
            iter.next();
            if dir_ent.is_last_long_dir_ent() {
                break;
            }
        }
        // Modify hint
        // We use new iterate mode
        let mut iter = self.dir_iter(
            inode_lock,
            Some(self.file_content.read().hint),
            DirIterMode::Enum,
            BACKWARD,
        );
        loop {
            let dir_ent = iter.next();
            if dir_ent.is_none() {
                // Indicates that the file is empty
                self.file_content.write().hint = 0;
                break;
            }
            let dir_ent = dir_ent.unwrap();
            if dir_ent.unused() {
                self.file_content.write().hint = iter.get_offset().unwrap();
                iter.write_to_current_ent(&FATDirEnt::unused_and_last_entry());
            } else {
                // Represents `iter` pointer to a used entry
                break;
            }
        }
        // Modify file size
        self.shrink_dir_size(inode_lock)
    }
    /// Create new disk space for derectory entries, including short and long entries.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `short_ent`: short entry
    /// + `long_ents`: list of long entries
    /// # Return Value
    /// If successful, it will return Ok.
    /// Otherwise, it will return Error.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock.
    fn create_dir_ent(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        short_ent: FATShortDirEnt,
        long_ents: Vec<FATLongDirEnt>,
    ) -> Result<u32, ()> {
        debug_assert!(self.is_dir());
        let short_ent_offset = match self.alloc_dir_ent(inode_lock, 1 + long_ents.len()) {
            Ok(offset) => offset,
            Err(_) => return Err(()),
        };
        // We have graranteed we have alloc enough entries
        // So we use Enum mode
        let mut iter = self.dir_iter(
            inode_lock,
            Some(short_ent_offset),
            DirIterMode::Enum,
            BACKWARD,
        );

        iter.write_to_current_ent(&FATDirEnt {
            short_entry: short_ent,
        });
        for long_ent in long_ents {
            iter.next();
            iter.write_to_current_ent(&FATDirEnt {
                long_entry: long_ent,
            });
        }
        Ok(short_ent_offset)
    }
    /// Modify current directory file's ".." directory entry
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `parent_dir_clus_num`: The first cluster number of the parent directory
    /// # Return Value
    /// If successful, it will return Ok.
    /// Otherwise, it will return Error.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    fn modify_parent_dir_entry(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        parent_dir_clus_num: u32,
    ) -> Result<(), ()> {
        debug_assert!(self.is_dir());
        let mut iter = self.dir_iter(inode_lock, None, DirIterMode::Used, FORWARD);
        loop {
            let dir_ent = iter.next();
            if dir_ent.is_none() {
                break;
            }
            let mut dir_ent = dir_ent.unwrap();
            if dir_ent.get_name() == ".." {
                dir_ent.set_fst_clus(parent_dir_clus_num);
                iter.write_to_current_ent(&dir_ent);
                return Ok(());
            }
        }
        Err(())
    }
}

/// Create
impl FatInode {
    /// Construct short and long entries
    /// # Arguments
    /// + `parent_dir`: The pointer to parent directory
    /// + `parent_inode_lock`: the lock of parent's inode
    /// + `name`: File name
    /// + `fst_clus`: The first cluster of constructing file
    /// + `file_type`: The file type of constructing file
    /// # Return Value
    /// A pair of a short directory entry and a list of long name entries
    /// # Warning
    /// This function will lock the `file_content` of the parent directory, may cause deadlock
    fn gen_dir_ent(
        parent_dir: &Arc<Self>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: &String,
        fst_clus: u32,
        file_type: DiskInodeType,
    ) -> (FATShortDirEnt, Vec<FATLongDirEnt>) {
        // Generate name slices
        let (short_name_slice, long_name_slices) =
            Self::gen_name_slice(parent_dir, parent_inode_lock, &name);
        // Generate short entry
        let short_ent = FATShortDirEnt::from_name(short_name_slice, fst_clus, file_type);
        // Generate long entries
        let long_ent_num = long_name_slices.len();
        let long_ents = long_name_slices
            .iter()
            .enumerate()
            .map(|(i, slice)| FATLongDirEnt::from_name_slice(i + 1 == long_ent_num, i + 1, *slice))
            .collect();
        (short_ent, long_ents)
    }

    /// 从一个目录项创建文件.
    /// # 参数
    /// + `parent_dir`: the parent directory inode pointer
    /// + `ent`: the short entry as the source of information
    /// + `offset`: the offset of the short directory entry in the `parent_dir`
    /// # 返回值
    /// 指向Inode的指针
    pub fn from_fat_ent(parent_dir: &Arc<Self>, ent: &FATShortDirEnt, offset: u32) -> Arc<Self> {
        Self::new(
            ent.get_first_clus(),
            if ent.is_dir() {
                DiskInodeType::Directory
            } else {
                DiskInodeType::File
            },
            if ent.is_file() {
                Some(ent.file_size)
            } else {
                None
            },
            Some((parent_dir.clone(), offset)),
            parent_dir.fs.clone(),
        )
    }

    /// Fill out an empty directory with only the '.' & '..' entries.
    /// # Arguments
    /// + `parent_dir`: the pointer of parent directory inode
    /// + `current_dir`: the pointer of new directory inode
    /// + `fst_clus`: the first cluster number of current file
    fn fill_empty_dir(parent_dir: &Arc<Self>, current_dir: &Arc<Self>, fst_clus: u32) {
        let current_inode_lock = current_dir.write();
        let mut iter = current_dir.dir_iter(&current_inode_lock, None, DirIterMode::Enum, FORWARD);
        let mut short_name: [u8; 11] = [' ' as u8; 11];
        //.
        iter.next();
        short_name[0] = '.' as u8;
        iter.write_to_current_ent(&FATDirEnt {
            short_entry: FATShortDirEnt::from_name(
                short_name,
                fst_clus as u32,
                DiskInodeType::Directory,
            ),
        });
        //..
        iter.next();
        short_name[1] = '.' as u8;
        iter.write_to_current_ent(&FATDirEnt {
            short_entry: FATShortDirEnt::from_name(
                short_name,
                parent_dir
                    .get_first_clus_lock(&parent_dir.file_content.read())
                    .unwrap(),
                DiskInodeType::Directory,
            ),
        });
        //add "unused and last" sign
        iter.next();
        iter.write_to_current_ent(&FATDirEnt::unused_and_last_entry());
    }
}

// ls and find local
impl FatInode {
    /// ls - General Purose file filterer
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// # WARNING
    /// The definition of OFFSET is CHANGED for this item.
    /// It should point to the NEXT USED entry whether it as a long entry whenever possible or a short entry if no long ones exist.
    /// # Return value
    /// On success, the function returns `Ok(_)`. On failure, multiple chances exist: either the Vec is empty, or the Result is `Err(())`.
    /// # Implementation Information
    /// The iterator stops at the last available item when it reaches the end,
    /// returning `None` from then on,
    /// so relying on the offset of the last item to decide whether it has reached an end is not recommended.
    #[inline(always)]
    pub fn ls_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
    ) -> Result<Vec<(String, FATShortDirEnt)>, ()> {
        if !self.is_dir() {
            return Err(());
        }
        Ok(self
            .dir_iter(inode_lock, None, DirIterMode::Used, FORWARD)
            .walk()
            .collect())
    }
    /// find `req_name` in current directory file
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `req_name`: required file name
    /// # Return value
    /// On success, the function returns `Ok(_)`. On failure, multiple chances exist: either the Vec is empty, or the Result is `Err(())`.
    pub fn find_local_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        req_name: String,
    ) -> Result<Option<(String, FATShortDirEnt, u32)>, ()> {
        if !self.is_dir() {
            return Err(());
        }
        log::debug!("[find_local] name: {:?}", req_name);
        let mut walker = self
            .dir_iter(inode_lock, None, DirIterMode::Used, FORWARD)
            .walk();
        match walker.find(|(name, _)| {
            name.len() == req_name.len() && name.as_str().eq_ignore_ascii_case(req_name.as_str())
        }) {
            Some((name, short_ent)) => {
                log::trace!("[find_local] Query name: {} found", req_name);
                Ok(Some((name, short_ent, walker.iter.get_offset().unwrap())))
            }
            None => {
                log::trace!("[find_local] Query name: {} not found", req_name);
                Ok(None)
            }
        }
    }
}

impl InodeTrait for FatInode {
    /// Get self's file content lock
    /// # Return Value
    /// a lock of file content
    #[inline(always)]
    fn read(&self) -> RwLockReadGuard<InodeLock> {
        self.inode_lock.read()
    }
    #[inline(always)]
    fn write(&self) -> RwLockWriteGuard<InodeLock> {
        self.inode_lock.write()
    }
    fn get_file_type_lock(&self) -> MutexGuard<DiskInodeType> {
        self.file_type.lock()
    }
    /// Get file type
    fn get_file_type(&self) -> DiskInodeType {
        *self.file_type.lock()
    }
    #[inline(always)]
    fn get_file_size_rlock(&self, _inode_lock: &RwLockReadGuard<InodeLock>) -> u32 {
        self.get_file_size()
    }
    fn get_file_size_wlock(&self, _inode_lock: &RwLockWriteGuard<InodeLock>) -> u32 {
        self.get_file_size()
    }
    #[inline(always)]
    fn get_file_size(&self) -> u32 {
        self.file_content.read().get_file_size()
    }
    /// Check if file type is directory
    /// # Return Value
    /// Bool result
    #[inline(always)]
    fn is_dir(&self) -> bool {
        self.get_file_type() == DiskInodeType::Directory
    }
    /// Check if file type is file
    /// # Return Value
    /// Bool result
    #[inline(always)]
    fn is_file(&self) -> bool {
        self.get_file_type() == DiskInodeType::File
    }
    /// 获取Inode号
    /// 方便起见，将第一个扇区号作为inode号
    /// # 参数
    /// + `lock`: The lock of target file content
    /// # 返回值
    /// If cluster list isn't empty, it will return the first sector number.
    /// Otherwise it will return None.
    #[inline(always)]
    fn get_inode_num_lock(&self, lock: &RwLockReadGuard<FileContent>) -> Option<u32> {
        self.get_first_clus_lock(lock)
            .map(|clus| self.fs.first_sector_of_cluster(clus))
    }
    /// Get first block id corresponding to the inner cache index
    /// # Arguments
    /// + `lock`: The lock of target file content
    /// + `inner_cache_id`: The index of inner cache
    /// # Return Value
    /// If `inner_cache_id` is valid, it will return the first block id
    /// Otherwise it will return None
    #[inline(always)]
    fn get_block_id(
        &self,
        lock: &RwLockReadGuard<FileContent>,
        inner_cache_id: u32,
    ) -> Option<u32> {
        let idx = inner_cache_id as usize / self.fs.sec_per_clus as usize;
        let clus_list = &lock.clus_list;
        if idx >= clus_list.len() {
            return None;
        }
        let base = self.fs.first_sector_of_cluster(clus_list[idx]);
        let offset = inner_cache_id % self.fs.sec_per_clus as u32;
        Some(base + offset)
    }
    /// 将文件内容读取到buffer中
    /// # 说明
    /// 会一直读取，直到文件的末尾或缓冲区不能再读取为止
    ///
    /// 这个操作会在
    /// start >= end
    /// 的时候被忽略
    /// # 参数
    /// + `inode_lock`: inode 锁
    /// + `offset`: 文件开始读取的起始位置
    /// + `buf`: 接受数据的buffer
    /// # 返回值
    /// + 读取到的字节数
    fn read_at_block_cache_rlock(
        &self,
        _inode_lock: &RwLockReadGuard<InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize {
        let mut start = offset;
        let size = self.file_content.read().size as usize;
        let end = (offset + buf.len()).min(size);
        if start >= end {
            return 0;
        }
        let mut start_cache = start / PageCacheManager::CACHE_SZ;
        let mut read_size = 0;
        loop {
            // calculate end of current block
            // 计算当前块的结束位置
            let mut end_current_block =
                (start / PageCacheManager::CACHE_SZ + 1) * PageCacheManager::CACHE_SZ;
            end_current_block = end_current_block.min(end);
            // 读取并更新读取长度
            let lock = self.file_content.read();
            let block_read_size = end_current_block - start;
            self.file_cache_mgr
                .get_cache(
                    start_cache,
                    || -> Vec<usize> { self.get_neighboring_sec(&lock.clus_list, start_cache) },
                    &self.fs.block_device,
                )
                .lock()
                // I know hardcoding 4096 in is bad, but I can't get around Rust's syntax checking...
                .read(0, |data_block: &[u8; 4096]| {
                    let dst = &mut buf[read_size..read_size + block_read_size];
                    let src = &data_block[start % PageCacheManager::CACHE_SZ
                        ..start % PageCacheManager::CACHE_SZ + block_read_size];
                    dst.copy_from_slice(src);
                });
            drop(lock);
            read_size += block_read_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_cache += 1;
            start = end_current_block;
        }
        read_size
    }
    /// do same thing but params different
    fn read_at_block_cache_wlock(
        &self,
        _inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize {
        let mut start = offset;
        let size = self.file_content.read().size as usize;
        let end = (offset + buf.len()).min(size);
        if start >= end {
            return 0;
        }
        let mut start_cache = start / PageCacheManager::CACHE_SZ;
        let mut read_size = 0;
        loop {
            // calculate end of current block
            let mut end_current_block =
                (start / PageCacheManager::CACHE_SZ + 1) * PageCacheManager::CACHE_SZ;
            end_current_block = end_current_block.min(end);
            // read and update read size
            let lock = self.file_content.read();
            let block_read_size = end_current_block - start;
            self.file_cache_mgr
                .get_cache(
                    start_cache,
                    || -> Vec<usize> { self.get_neighboring_sec(&lock.clus_list, start_cache) },
                    &self.fs.block_device,
                )
                .lock()
                // I know hardcoding 4096 in is bad, but I can't get around Rust's syntax checking...
                .read(0, |data_block: &[u8; 4096]| {
                    let dst = &mut buf[read_size..read_size + block_read_size];
                    let src = &data_block[start % PageCacheManager::CACHE_SZ
                        ..start % PageCacheManager::CACHE_SZ + block_read_size];
                    dst.copy_from_slice(src);
                });
            drop(lock);
            read_size += block_read_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_cache += 1;
            start = end_current_block;
        }
        read_size
    }
    /// Read file content into buffer.
    /// It will read from `offset` until the end of the file or buffer can't read more
    /// This operation is ignored if start is greater than or equal to end.
    /// # Arguments
    /// + `offset`: The start offset in file
    /// + `buf`: The buffer to receive data
    /// # Return Value
    /// The number of number of bytes read.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    #[inline(always)]
    fn read_at_block_cache(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.read_at_block_cache_rlock(&self.read(), offset, buf)
    }

    /// Write buffer into file content.
    /// It will start to write from `offset` until the buffer is written,
    /// and when the write exceeds the end of file, it will modify file's size.
    /// If hard disk space id low, it will try to write as much data as possible.
    /// # 参数
    /// + `inode_lock`: inode锁
    /// + `offset`: The start offset in file
    /// + `buf`: The buffer to write data
    /// # 返回值
    /// The number of number of bytes write.
    fn write_at_block_cache_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: usize,
        buf: &[u8],
    ) -> usize {
        let mut start = offset;
        let old_size = self.get_file_size() as usize;
        let diff_len = buf.len() as isize + offset as isize - old_size as isize;
        if diff_len > 0 as isize {
            // allocate as many clusters as possible.
            self.modify_size_lock(inode_lock, diff_len, false);
        }
        let end = (offset + buf.len()).min(self.get_file_size() as usize);

        debug_assert!(start <= end);

        let mut start_cache = start / PageCacheManager::CACHE_SZ;
        let mut write_size = 0;
        loop {
            // calculate end of current block
            let mut end_current_block =
                (start / PageCacheManager::CACHE_SZ + 1) * PageCacheManager::CACHE_SZ;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let lock = self.file_content.read();
            let block_write_size = end_current_block - start;
            self.file_cache_mgr
                .get_cache(
                    start_cache,
                    || -> Vec<usize> { self.get_neighboring_sec(&lock.clus_list, start_cache) },
                    &self.fs.block_device,
                )
                .lock()
                // I know hardcoding 4096 in is bad, but I can't get around Rust's syntax checking...
                .modify(0, |data_block: &mut [u8; 4096]| {
                    let src = &buf[write_size..write_size + block_write_size];
                    let dst = &mut data_block[start % PageCacheManager::CACHE_SZ
                        ..start % PageCacheManager::CACHE_SZ + block_write_size];
                    dst.copy_from_slice(src);
                });
            drop(lock);
            write_size += block_write_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_cache += 1;
            start = end_current_block;
        }
        write_size
    }

    /// Write buffer into file content.
    /// It will start to write from `offset` until the buffer is written,
    /// and when the write exceeds the end of file, it will modify file's size.
    /// If hard disk space id low, it will try to write as much data as possible.
    /// # Arguments
    /// + `offset`: The start offset in file
    /// + `buf`: The buffer to write data
    /// # Return Value
    /// The number of number of bytes write.
    /// # Warning
    /// This function will lock self's `file_content`, may cause deadlock
    #[inline(always)]
    fn write_at_block_cache(&self, offset: usize, buf: &[u8]) -> usize {
        self.write_at_block_cache_lock(&self.write(), offset, buf)
    }

    /// Get a page cache corresponding to `inner_cache_id`.
    /// 获取一个与inner_cache_id对应的pagecache,
    /// # 参数
    /// + `inner_cache_id`: 内部cache的id号
    /// # 返回值
    /// 指向PageCache的指针
    /// # 警告
    /// 这个函数会将file_content上锁，可能会导致死锁
    fn get_single_cache(&self, inner_cache_id: usize) -> Arc<Mutex<PageCache>> {
        self.get_single_cache_lock(&self.read(), inner_cache_id)
    }

    /// 获取与 `inner_cache_id` 对应的页面缓存。
    /// # 参数
    /// + `inode_lock`: inode 的锁
    /// + `inner_cache_id`: 内部缓存的索引
    /// # 返回值
    /// 指向页面缓存的指针
    fn get_single_cache_lock(
        &self,
        _inode_lock: &RwLockReadGuard<InodeLock>,
        inner_cache_id: usize,
    ) -> Arc<Mutex<PageCache>> {
        // 上锁，共享只读访问
        let lock = self.file_content.read();
        // 获取邻近的扇区
        self.file_cache_mgr.get_cache(
            inner_cache_id,
            || -> Vec<usize> { self.get_neighboring_sec(&lock.clus_list, inner_cache_id) },
            &self.fs.block_device,
        )
    }

    /// 获取所有文件对应的页缓存
    /// # 返回值
    /// 指向页缓存的指针列表
    fn get_all_cache(&self) -> Vec<Arc<Mutex<PageCache>>> {
        // 上锁，共享只读访问
        let inode_lock = self.read();
        // 上锁，共享只读访问，lock为file_content
        let lock = self.file_content.read();
        // 获取cache_num，缓存页数
        // 计算公式为
        // (文件大小 + 4096(页大小) - 1) / 4096(页大小)
        // 确保文件内容不是CACHE_SZ整数倍时也可以多分配一个页面缓存
        let cache_num =
            (lock.size as usize + PageCacheManager::CACHE_SZ - 1) / PageCacheManager::CACHE_SZ;
        // 初始化缓存列表，预先分配空间，避免多次重新分配内存
        let mut cache_list = Vec::<Arc<Mutex<PageCache>>>::with_capacity(cache_num);
        // 遍历所有缓存页，加入到缓存列表中
        for inner_cache_id in 0..cache_num {
            cache_list.push(self.get_single_cache_lock(&inode_lock, inner_cache_id));
        }
        cache_list
    }

    /// Delete the short and the long entry of `self` from `parent_dir`
    /// # 返回值
    /// 执行成功返回Ok
    /// 否则返回Err
    /// # 警告
    /// 这个函数会给parent_dir上锁，可能会导致死锁
    fn delete_self_dir_ent(&self) -> Result<(), ()> {
        if let Some((par_inode, offset)) = &*self.parent_dir.lock() {
            return par_inode.delete_dir_ent(&par_inode.write(), *offset);
        }
        Err(())
    }

    /// Delete the file from the disk,
    /// This file doesn't be removed immediately(dropped)
    /// deallocating both the directory entries (whether long or short),
    /// and the occupied clusters.
    /// # Arguments
    /// + `inode_lock`: The lock of inode
    /// + `delete`: Signal of deleting the file content when inode is dropped
    /// # Return Value
    /// If successful, it will return Ok.
    /// Otherwise, it will return Error with error number.
    /// # Warning
    /// This function will lock trash's `file_content`, may cause deadlock
    /// Make sure Arc has a strong count of 1.
    /// Make sure all its caches are not held by anyone else.
    /// Make sure target directory file is empty.
    fn unlink_lock(
        &self,
        _inode_lock: &RwLockWriteGuard<InodeLock>,
        delete: bool,
    ) -> Result<(), isize> {
        log::debug!(
            "[delete_from_disk] inode: {:?}, type: {:?}",
            self.get_inode_num_lock(&self.file_content.read()),
            self.file_type
        );
        // Remove directory entries
        if self.parent_dir.lock().is_none() {
            return Ok(());
        }
        if self.delete_self_dir_ent().is_err() {
            panic!()
        }
        if delete {
            *self.deleted.lock() = true;
        }
        *self.parent_dir.lock() = None;
        Ok(())
    }

    /// Get a dirent information from the `self` at `offset`
    /// 获取`self`目录中的`offset`位置的目录项信息
    /// 当 self 不是目录时返回 None
    /// # 参数
    /// + `inode_lock`: inode 锁
    /// + `offset` 目录项的起始偏移量（目录项从哪个位置开始读取）
    /// + `length` 需要读取的目录项长度
    /// # 返回值
    /// On success, the function returns `Ok(file name, file size, first cluster, file type)`.
    /// On failure, multiple chances exist: either the Vec is empty, or the Result is `Err(())`.
    fn dirent_info_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
        length: usize,
    ) -> Result<Vec<(String, usize, u64, FATDiskInodeType)>, ()> {
        // 如果文件不是目录，返回错误
        if !self.is_dir() {
            return Err(());
        }
        // 获取文件大小
        let size = self.get_file_size();
        // 初始化迭代器
        let mut walker = self
            .dir_iter(inode_lock, None, DirIterMode::Used, FORWARD)
            .walk();
        // 设置迭代器起始偏移量
        walker.iter.set_iter_offset(offset);
        // 初始化存储目录项的向量
        let mut v = Vec::with_capacity(length);

        // 读取第一个目录项
        let (mut last_name, mut last_short_ent) = match walker.next() {
            Some(tuple) => tuple,
            // 若目录为空直接返回空向量
            None => return Ok(v),
        };
        // 遍历目录项并插入到向量中
        for _ in 0..length {
            // 计算下一个目录项的偏移量
            let next_dirent_offset =
                walker.iter.get_offset().unwrap() as usize + core::mem::size_of::<FATDirEnt>();
            // 获取下一个目录项
            let (name, short_ent) = match walker.next() {
                Some(tuple) => tuple,
                None => {
                    // 插入上一个结果，然后直接返回
                    v.push((
                        last_name,
                        size as usize,
                        last_short_ent.get_first_clus() as u64,
                        last_short_ent.attr,
                    ));
                    return Ok(v);
                }
            };
            // 插入上一个结果
            v.push((
                last_name,
                next_dirent_offset,
                last_short_ent.get_first_clus() as u64,
                last_short_ent.attr,
            ));
            // 更新新的目录项
            last_name = name;
            last_short_ent = short_ent;
        }
        Ok(v)
    }

    /// 获取状态stat结构体
    /// # 参数
    /// + `inode_lock`: inode锁
    /// # 返回值
    /// (file size, access time, modify time, create time, inode number)
    fn stat_lock(&self, _inode_lock: &RwLockReadGuard<InodeLock>) -> (i64, i64, i64, i64, u64) {
        let time = self.time.lock();
        (
            self.get_file_size() as i64,
            time.access_time().clone() as i64,
            time.modify_time().clone() as i64,
            time.create_time().clone() as i64,
            self.get_inode_num_lock(&self.file_content.read())
                .unwrap_or(0) as u64,
        )
    }

    /// 获取 time 字段
    fn time(&self) -> MutexGuard<InodeTime> {
        self.time.lock()
    }

    /// 当内存不足的时候，调用该函数来释放其缓存
    /// it just tries to lock it's file contents to free memory
    /// # 返回值
    /// oom函数释放掉的页的数量
    fn oom(&self) -> usize {
        let neighbor = |inner_cache_id| {
            self.get_neighboring_sec(&self.file_content.read().clus_list, inner_cache_id)
        };
        self.file_cache_mgr.oom(neighbor, &self.fs.block_device)
    }

    /// 改变当前文件的大小
    /// This operation is ignored if the result size is negative
    /// # 参数
    /// + `inode_lock`: inode锁
    /// + `diff`: file 大小的改变量
    /// # 警告
    /// This function will not modify its parent directory (since we changed the size of the current file),
    /// we will modify it when it is deleted.
    fn modify_size_lock(&self, inode_lock: &RwLockWriteGuard<InodeLock>, diff: isize, clear: bool) {
        let mut lock = self.file_content.write();

        debug_assert!(diff.saturating_add(lock.size as isize) >= 0);

        let old_size = lock.size;
        let new_size = (lock.size as isize + diff) as u32;

        let old_clus_num = self.total_clus(old_size) as usize;
        let new_clus_num = self.total_clus(new_size) as usize;

        if diff > 0 {
            self.alloc_clus(&mut lock, new_clus_num - old_clus_num);
        } else {
            self.dealloc_clus(&mut lock, old_clus_num - new_clus_num);
        }

        lock.size = new_size;
        drop(lock);

        if diff > 0 {
            if clear {
                self.clear_at_block_cache_lock(
                    inode_lock,
                    old_size as usize,
                    (new_size - old_size) as usize,
                );
            }
        } else {
            self.file_cache_mgr.notify_new_size(new_size as usize)
        }
    }

    fn is_empty_dir_lock(&self, inode_lock: &RwLockWriteGuard<InodeLock>) -> bool {
        if !self.is_dir() {
            return false;
        }
        let iter = self
            .dir_iter(inode_lock, None, DirIterMode::Used, FORWARD)
            .walk();
        for (name, _) in iter {
            if [".", ".."].contains(&name.as_str()) == false {
                return false;
            }
        }
        true
    }

    /// 获取所有子文件
    /// # 参数
    /// + inode_lock：inode锁
    /// # 返回值
    /// + 子文件信息向量
    fn get_all_files_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
    ) -> Vec<(String, FATShortDirEnt, u32)> {
        // 存放子文件的向量
        // 容量预设为8
        let mut vec = Vec::with_capacity(8);
        // 创建目录迭代器
        let mut walker = self
            .dir_iter(inode_lock, None, DirIterMode::Used, FORWARD)
            .walk();
        loop {
            // 遍历目录项
            let ele = walker.next();
            match ele {
                Some((name, short_ent)) => {
                    // 跳过特殊目录项
                    if name == "." || name == ".." {
                        continue;
                    }
                    // 存放目录项
                    vec.push((name, short_ent, walker.iter.get_offset().unwrap()))
                }
                None => break,
            }
        }
        vec
    }

    fn from_ent(
        &self,
        parent_dir: &Arc<dyn InodeTrait>,
        ent: &FATShortDirEnt,
        offset: u32,
    ) -> Arc<dyn InodeTrait> {
        let parent_dir_specific = Arc::downcast::<FatInode>(parent_dir.clone()).unwrap();
        // let shit = parent_dir.clone();
        let inode = Self::from_fat_ent(&parent_dir_specific, ent, offset);
        inode
    }

    fn link_par_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        parent_dir: &Arc<dyn InodeTrait>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: String,
    ) -> Result<(), ()>
// where
        // Self: Sized,
    {
        // let parent_dir_specific = parent_dir.as_any().downcast_ref::<Arc<Self>>().ok_or(())?;
        let parent_dir_specific = Arc::downcast::<FatInode>(parent_dir.clone()).unwrap();
        // Genrate directory entries
        let (short_ent, long_ents) = Self::gen_dir_ent(
            // parent_dir,
            &parent_dir_specific,
            parent_inode_lock,
            &name,
            self.get_first_clus_lock(&self.file_content.read())
                .unwrap_or(0),
            *self.file_type.lock(),
        );
        // Allocate new directory entry
        let short_ent_offset =
            // match parent_dir.create_dir_ent(parent_inode_lock, short_ent, long_ents) {
            match parent_dir_specific.create_dir_ent(parent_inode_lock, short_ent, long_ents) {
                Ok(offset) => offset,
                Err(_) => return Err(()),
            };
        // If this is a directory, modify ".."
        if self.is_dir()
            && self
                .modify_parent_dir_entry(
                    inode_lock,
                    // parent_dir
                    parent_dir_specific
                        // .get_first_clus_lock(&parent_dir.file_content.read())
                        .get_first_clus_lock(&parent_dir_specific.file_content.read())
                        .unwrap(),
                )
                .is_err()
        {
            return Err(());
        }
        // Modify parent directory
        // *self.parent_dir.lock() = Some((parent_dir.clone(), short_ent_offset));
        *self.parent_dir.lock() = Some((parent_dir_specific.clone(), short_ent_offset));
        Ok(())
    }

    /// 从父目录创建一个文件或目录
    /// 父目录将写入新的文件目录项。
    /// # 参数
    /// + `parent_dir`: 指向父目录的指针
    /// + `parent_inode_lock`: 父目录的锁
    /// + `name`: 新文件的名字
    /// + `file_type`: 新文件的文件类型
    /// # 返回值
    /// 如果成功，会返回新文件的inode
    /// 否则，返回错误
    /// # 警告
    /// 这个函数会将父目录的`file_content`锁住，可能会导致死锁
    /// 名字的长度应该小于256(ascii)，否则文件系统无法存储。
    /// 确保父目录中没有重复的名字
    fn create_lock(
        &self,
        parent_dir: &Arc<dyn InodeTrait>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: String,
        file_type: DiskInodeType,
    ) -> Result<Arc<dyn InodeTrait>, ()>
    where
        Self: Sized,
    {
        // 将父Inode下转为具体的Inode类型
        let parent_dir_specific = Arc::downcast::<FatInode>(parent_dir.clone()).unwrap();
        // 如果父Inode是普通文件或者名称长度大于256，返回错误
        if parent_dir.is_file() || name.len() >= 256 {
            Err(())
        } else {
            log::debug!(
                "[create] par_inode: {:?}, name: {:?}, file_type: {:?}",
                parent_dir_specific.get_inode_num_lock(&parent_dir_specific.file_content.read()),
                &name,
                file_type
            );
            // 如果文件类型是目录，分配第一个簇
            let fst_clus = if file_type == DiskInodeType::Directory {
                let fst_clus =
                    parent_dir_specific
                        .fs
                        .fat
                        .alloc(&parent_dir_specific.fs.block_device, 1, None);
                // 如果分配到的第一个簇是空值，返回错误
                if fst_clus.is_empty() {
                    return Err(());
                }
                fst_clus[0]
            } else {
                // 常规文件，fst_clus = 0
                0
            };
            // 生成目录项
            let (short_ent, long_ents) =
                // Self::gen_dir_ent(parent_dir, parent_inode_lock, &name, fst_clus, file_type);
                Self::gen_dir_ent(&parent_dir_specific, parent_inode_lock, &name, fst_clus, file_type);
            // Create directory entry
            let short_ent_offset =
                // match parent_dir.create_dir_ent(parent_inode_lock, short_ent, long_ents) {
                match parent_dir_specific.create_dir_ent(parent_inode_lock, short_ent, long_ents) {
                    Ok(offset) => offset,
                    Err(_) => return Err(()),
                };
            // Generate current file
            // let current_file = Self::from_fat_ent(&parent_dir, &short_ent, short_ent_offset);
            let current_file =
                Self::from_fat_ent(&parent_dir_specific, &short_ent, short_ent_offset);
            // If file_type is Directory, set first 3 directory entry
            if file_type == DiskInodeType::Directory {
                // Set hint
                current_file.file_content.write().hint =
                    2 * core::mem::size_of::<FATDirEnt>() as u32;
                // Fill content
                // Self::fill_empty_dir(&parent_dir, &current_file, fst_clus);
                Self::fill_empty_dir(&parent_dir_specific, &current_file, fst_clus);
            }
            Ok(current_file)
        }
    }
    /// Construct a \[u8,11\] corresponding to the short directory entry name
    /// # Arguments
    /// + `parent_dir`: The pointer to parent directory
    /// + `parent_inode_lock`: the lock of parent's inode
    /// + `name`: File name
    /// # Return Value
    /// A short name slice
    /// # Warning
    /// This function will lock the `file_content` of the parent directory, may cause deadlock
    fn gen_short_name_slice(
        parent_dir: &Arc<Self>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: &String,
    ) -> [u8; 11] {
        let short_name = FATDirEnt::gen_short_name_prefix(name.clone());
        if short_name.len() == 0 || short_name.find(' ').unwrap_or(8) == 0 {
            panic!("illegal short name");
        }

        let mut short_name_slice = [0u8; 11];
        short_name_slice.copy_from_slice(&short_name.as_bytes()[0..11]);

        let iter = parent_dir.dir_iter(parent_inode_lock, None, DirIterMode::Short, FORWARD);
        FATDirEnt::gen_short_name_numtail(iter.collect(), &mut short_name_slice);
        short_name_slice
    }
    /// Construct short and long entries name slices
    /// # Arguments
    /// + `parent_dir`: The pointer to parent directory
    /// + `parent_inode_lock`: the lock of parent's inode
    /// + `name`: File name
    /// # Return Value
    /// A pair of a short name slice and a list of long name slices
    /// # Warning
    /// This function will lock the `file_content` of the parent directory, may cause deadlock
    fn gen_name_slice(
        parent_dir: &Arc<Self>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: &String,
    ) -> ([u8; 11], Vec<[u16; 13]>) {
        let short_name_slice = Self::gen_short_name_slice(parent_dir, parent_inode_lock, name);

        let long_ent_num = div_ceil!(name.len(), 13);
        //name.len().div_ceil(13);
        let mut long_name_slices = Vec::<[u16; 13]>::with_capacity(long_ent_num);
        for i in 0..long_ent_num {
            long_name_slices.push(Self::gen_long_name_slice(name, i));
        }

        (short_name_slice, long_name_slices)
    }
    /// Construct a \[u16,13\] corresponding to the `long_ent_num`'th 13-u16 or shorter name slice
    /// _NOTE_: the first entry is of number 0 for `long_ent_num`
    /// # Arguments
    /// + `name`: File name
    /// + `long_ent_index`: The index of long entry(start from 0)
    /// # Return Value
    /// A long name slice
    fn gen_long_name_slice(name: &String, long_ent_index: usize) -> [u16; 13] {
        let mut v: Vec<u16> = name.encode_utf16().collect();
        debug_assert!(long_ent_index * 13 < v.len());
        while v.len() < (long_ent_index + 1) * 13 {
            v.push(0);
        }
        let start = long_ent_index * 13;
        let end = (long_ent_index + 1) * 13;
        v[start..end].try_into().expect("should be able to cast")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn root_inode(efs: &Arc<dyn VFS>) -> Arc<Self> {
        FatInode::root_inode(efs)
    }
}
