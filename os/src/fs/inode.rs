use crate::fs::directory_tree::DirectoryTreeNode;
use crate::fs::file_trait::File;
use crate::fs::*;
use crate::mm::UserBuffer;
use crate::syscall::errno::*;
use core::any::Any;
use core::panic;

use crate::fs::fat32::fat_inode::Inode;
use crate::fs::fat32::layout::FATDiskInodeType;
use crate::fs::vfs::VFS;
use alloc::string::ToString;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use dirent::Dirent;
use downcast_rs::*;
use fat32::fat_inode::FileContent;
use fat32::layout::FATShortDirEnt;
use spin::{Mutex, MutexGuard, RwLockReadGuard, RwLockWriteGuard};
#[allow(unused)]
use vfs::VFSDirEnt;

pub struct InodeLock;

pub type FatInode = Inode;

#[allow(unused)]
pub trait InodeTrait: DowncastSync {
    fn read(&self) -> RwLockReadGuard<InodeLock>;
    fn write(&self) -> RwLockWriteGuard<InodeLock>;
    fn get_file_type_lock(&self) -> MutexGuard<DiskInodeType>;
    fn get_file_type(&self) -> DiskInodeType;
    fn get_file_size(&self) -> u32;
    fn get_file_size_rlock(&self, _inode_lock: &RwLockReadGuard<InodeLock>) -> u32;
    fn get_file_size_wlock(&self, _inode_lock: &RwLockWriteGuard<InodeLock>) -> u32;
    fn is_dir(&self) -> bool;
    fn is_file(&self) -> bool;
    fn get_inode_num_lock(&self, lock: &RwLockReadGuard<FileContent>) -> Option<u32>;
    fn get_block_id(&self, lock: &RwLockReadGuard<FileContent>, inner_cache_id: u32)
        -> Option<u32>;
    fn read_at_block_cache_rlock(
        &self,
        _inode_lock: &RwLockReadGuard<InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize;
    fn read_at_block_cache_wlock(
        &self,
        _inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize;
    fn read_at_block_cache(&self, offset: usize, buf: &mut [u8]) -> usize;
    fn write_at_block_cache_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: usize,
        buf: &[u8],
    ) -> usize;
    fn write_at_block_cache(&self, offset: usize, buf: &[u8]) -> usize;
    fn get_single_cache(&self, inner_cache_id: usize) -> Arc<Mutex<PageCache>>;
    fn get_single_cache_lock(
        &self,
        _inode_lock: &RwLockReadGuard<InodeLock>,
        inner_cache_id: usize,
    ) -> Arc<Mutex<PageCache>>;
    fn get_all_cache(&self) -> Vec<Arc<Mutex<PageCache>>>;
    fn get_all_files_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
    ) -> Vec<(String, FATShortDirEnt, u32)>;
    // ) -> Vec<(String, Box<dyn VFSDirEnt>, u32)>;
    fn dirent_info_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        offset: u32,
        length: usize,
    ) -> Result<Vec<(String, usize, u64, FATDiskInodeType)>, ()>;
    fn delete_self_dir_ent(&self) -> Result<(), ()>;
    fn unlink_lock(
        &self,
        _inode_lock: &RwLockWriteGuard<InodeLock>,
        delete: bool,
    ) -> Result<(), isize>;
    fn stat_lock(&self, _inode_lock: &RwLockReadGuard<InodeLock>) -> (i64, i64, i64, i64, u64);
    fn time(&self) -> MutexGuard<InodeTime>;
    fn oom(&self) -> usize;
    fn modify_size_lock(&self, inode_lock: &RwLockWriteGuard<InodeLock>, diff: isize, clear: bool);
    fn is_empty_dir_lock(&self, inode_lock: &RwLockWriteGuard<InodeLock>) -> bool;

    // 从现有的目录项创建新的文件
    fn from_ent(
        &self,
        parent_dir: &Arc<dyn InodeTrait>,
        ent: &FATShortDirEnt,
        offset: u32,
    ) -> Arc<dyn InodeTrait>;
    // where
    //     Self: Sized;
    fn link_par_lock(
        &self,
        inode_lock: &RwLockWriteGuard<InodeLock>,
        parent_dir: &Arc<dyn InodeTrait>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: String,
    ) -> Result<(), ()>;
    fn create_lock(
        &self,
        parent_dir: &Arc<dyn InodeTrait>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: String,
        file_type: DiskInodeType,
    ) -> Result<Arc<dyn InodeTrait>, ()>;
    // where
    // Self: Sized;
    fn gen_short_name_slice(
        parent_dir: &Arc<Self>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: &String,
    ) -> [u8; 11]
    where
        Self: Sized;
    fn gen_name_slice(
        parent_dir: &Arc<Self>,
        parent_inode_lock: &RwLockWriteGuard<InodeLock>,
        name: &String,
    ) -> ([u8; 11], Vec<[u16; 13]>)
    where
        Self: Sized;
    fn gen_long_name_slice(name: &String, long_ent_index: usize) -> [u16; 13]
    where
        Self: Sized;
    fn as_any(&self) -> &dyn Any;
    fn root_inode(efs: &Arc<dyn VFS>) -> Arc<Self>
    where
        Self: Sized;
}
impl_downcast!(sync InodeTrait);

pub struct InodeTime {
    create_time: u64,
    access_time: u64,
    modify_time: u64,
}
#[allow(unused)]
impl InodeTime {
    pub fn new() -> Self {
        Self {
            create_time: 0,
            access_time: 0,
            modify_time: 0,
        }
    }
    /// 设置inode的创建时间
    pub fn set_create_time(&mut self, create_time: u64) {
        self.create_time = create_time;
    }

    /// 获取inode的创建时间的引用
    pub fn create_time(&self) -> &u64 {
        &self.create_time
    }

    /// 设置inode的访问时间
    pub fn set_access_time(&mut self, access_time: u64) {
        self.access_time = access_time;
    }

    /// 获取inode的访问时间的引用
    pub fn access_time(&self) -> &u64 {
        &self.access_time
    }

    /// 设置inode的修改时间
    pub fn set_modify_time(&mut self, modify_time: u64) {
        self.modify_time = modify_time;
    }

    /// 获取inode的修改时间的引用
    pub fn modify_time(&self) -> &u64 {
        &self.modify_time
    }
}

/// OSInode
/// 对具体文件系统Inode的封装
pub struct OSInode {
    /// 是否可读
    readable: bool,
    /// 是否可写
    writable: bool,
    /// 被进程使用的计数
    special_use: bool,
    /// 是否追加
    append: bool,
    /// 具体的Inode
    inner: Arc<dyn InodeTrait>,
    /// 文件偏移
    offset: Mutex<usize>,
    /// 目录树节点指针
    dirnode_ptr: Arc<Mutex<Weak<DirectoryTreeNode>>>,
}

impl OSInode {
    // 只在获取根目录时使用
    pub fn new(root_inode: Arc<dyn InodeTrait>) -> Arc<dyn File> {
        Arc::new(Self {
            readable: true,
            writable: true,
            special_use: true,
            append: false,
            inner: root_inode,
            offset: Mutex::new(0),
            dirnode_ptr: Arc::new(Mutex::new(Weak::new())),
        })
    }
}

impl Drop for OSInode {
    fn drop(&mut self) {
        if self.special_use {
            let inode = self.get_dirtree_node();
            match inode {
                Some(inode) => inode.sub_special_use(),
                None => {}
            }
        }
    }
}

#[allow(unused)]
impl File for OSInode {
    fn deep_clone(&self) -> Arc<dyn File> {
        if self.special_use {
            let inode = self.get_dirtree_node();
            match inode {
                Some(inode) => inode.add_special_use(),
                None => {}
            }
        }
        Arc::new(Self {
            readable: self.readable,
            writable: self.writable,
            special_use: self.special_use,
            append: self.append,
            inner: self.inner.clone(),
            offset: Mutex::new(*self.offset.lock()),
            dirnode_ptr: self.dirnode_ptr.clone(),
        })
    }
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    /// 如果 offset 不是 `None`，`kread()` 会从偏移位置开始读取文件
    /// `*offset`会被调整以反映写入缓冲区的字节数
    /// 并且文件偏移不会被修改
    /// 否则 `kread()` 会开始从偏移位置开始读取文件，
    /// 文件偏移会被调整以反映写入缓冲区的字节数
    /// # 警告
    /// + Buffer 必须在内核态
    fn read(&self, offset: Option<&mut usize>, buffer: &mut [u8]) -> usize {
        match offset {
            Some(offset) => {
                let len = self.inner.read_at_block_cache(*offset, buffer);
                *offset += len;
                len
            }
            None => {
                let mut offset = self.offset.lock();
                let len = self.inner.read_at_block_cache(*offset, buffer);
                *offset += len;
                len
            }
        }
    }
    /// If offset is not `None`, `kwrite()` will start writing file from `*offset`,
    /// the `*offset` is adjusted to reflect the number of bytes read from the buffer,
    /// and the file offset won't be modified.
    /// Otherwise `kwrite()` will start writing file from file offset,
    /// the file offset is adjusted to reflect the number of bytes read from the buffer.
    /// # Warning
    /// Buffer must be in kernel space
    fn write(&self, offset: Option<&mut usize>, buffer: &[u8]) -> usize {
        match offset {
            Some(offset) => {
                let len = self.inner.write_at_block_cache(*offset, buffer);
                *offset += len;
                len
            }
            None => {
                let mut offset = self.offset.lock();
                let inode_lock = self.inner.write();
                if self.append {
                    *offset = self.inner.get_file_size_wlock(&inode_lock) as usize;
                }
                let len = self
                    .inner
                    .write_at_block_cache_lock(&inode_lock, *offset, buffer);
                *offset += len;
                len
            }
        }
    }
    fn r_ready(&self) -> bool {
        true
    }
    fn w_ready(&self) -> bool {
        true
    }
    fn read_user(&self, offset: Option<usize>, mut buf: UserBuffer) -> usize {
        let mut total_read_size = 0usize;

        let inode_lock = self.inner.read();
        match offset {
            Some(mut offset) => {
                let mut offset = &mut offset;
                for slice in buf.buffers.iter_mut() {
                    let read_size =
                        self.inner
                            .read_at_block_cache_rlock(&inode_lock, *offset, *slice);
                    if read_size == 0 {
                        break;
                    }
                    *offset += read_size;
                    total_read_size += read_size;
                }
            }
            None => {
                let mut offset = self.offset.lock();
                for slice in buf.buffers.iter_mut() {
                    let read_size =
                        self.inner
                            .read_at_block_cache_rlock(&inode_lock, *offset, *slice);
                    if read_size == 0 {
                        break;
                    }
                    *offset += read_size;
                    total_read_size += read_size;
                }
            }
        }
        total_read_size
    }

    fn write_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        let mut total_write_size = 0usize;

        let inode_lock = self.inner.write();
        match offset {
            Some(mut offset) => {
                let mut offset = &mut offset;
                for slice in buf.buffers.iter() {
                    let write_size =
                        self.inner
                            .write_at_block_cache_lock(&inode_lock, *offset, *slice);
                    assert_eq!(write_size, slice.len());
                    *offset += write_size;
                    total_write_size += write_size;
                }
            }
            None => {
                let mut offset = self.offset.lock();
                if self.append {
                    *offset = self.inner.get_file_size_wlock(&inode_lock) as usize;
                }
                for slice in buf.buffers.iter() {
                    let write_size =
                        self.inner
                            .write_at_block_cache_lock(&inode_lock, *offset, *slice);
                    assert_eq!(write_size, slice.len());
                    *offset += write_size;
                    total_write_size += write_size;
                }
            }
        }
        total_write_size
    }
    fn get_size(&self) -> usize {
        self.inner.get_file_size() as usize
    }
    fn get_stat(&self) -> Stat {
        let (size, atime, mtime, ctime, ino) = self.inner.stat_lock(&self.inner.read());
        let st_mod: u32 = {
            if self.inner.is_dir() {
                (StatMode::S_IFDIR | StatMode::S_IRWXU | StatMode::S_IRWXG | StatMode::S_IRWXO)
                    .bits()
            } else {
                (StatMode::S_IFREG | StatMode::S_IRWXU | StatMode::S_IRWXG | StatMode::S_IRWXO)
                    .bits()
            }
        };
        Stat::new(
            crate::makedev!(8, 0),
            ino,
            st_mod,
            1,
            0,
            size,
            atime,
            mtime,
            ctime,
        )
    }
    fn get_file_type(&self) -> DiskInodeType {
        self.inner.get_file_type()
    }

    fn info_dirtree_node(&self, dirnode_ptr: Weak<DirectoryTreeNode>) {
        *self.dirnode_ptr.lock() = dirnode_ptr;
    }

    fn get_dirtree_node(&self) -> Option<Arc<DirectoryTreeNode>> {
        self.dirnode_ptr.lock().upgrade()
    }

    /// 打开文件
    /// # 参数
    /// + flags: 标志
    /// + special_user: 打开这个文件的进程数
    /// # 返回值
    /// + 本身
    fn open(&self, flags: OpenFlags, special_use: bool) -> Arc<dyn File> {
        Arc::new(Self {
            readable: flags.contains(OpenFlags::O_RDONLY) || flags.contains(OpenFlags::O_RDWR),
            writable: flags.contains(OpenFlags::O_WRONLY) || flags.contains(OpenFlags::O_RDWR),
            special_use,
            append: flags.contains(OpenFlags::O_APPEND),
            inner: self.inner.clone(),
            offset: Mutex::new(0),
            dirnode_ptr: self.dirnode_ptr.clone(),
        })
    }
    /// 打开子文件
    /// # 返回值
    /// + 子文件数组
    fn open_subfile(&self) -> Result<Vec<(String, Arc<dyn File>)>, isize> {
        // 获取实际Inode
        let inode_lock = self.inner.write();

        // 子文件构造闭包
        // 根据目录项 short_ent
        // 和偏移量 offset
        // 返回值
        // + 一个File对象（OSInode）
        let get_dyn_file = |short_ent, offset| -> Arc<dyn File> {
            Arc::new(Self {
                readable: true,
                writable: true,
                special_use: false,
                append: false,
                inner: self.inner.from_ent(&self.inner, short_ent, offset),
                offset: Mutex::new(0),
                dirnode_ptr: Arc::new(Mutex::new(Weak::new())),
            })
        };
        Ok(self
            .inner
            .get_all_files_lock(&inode_lock)
            .iter()
            .map(|(name, short_ent, offset)| (name.clone(), get_dyn_file(short_ent, *offset)))
            .collect())
    }

    /// 创建文件
    /// # 参数
    /// + name：文件名
    /// + file_type: 文件类型
    /// # 返回值
    /// + 文件对象
    fn create(&self, name: &str, file_type: DiskInodeType) -> Result<Arc<dyn File>, isize> {
        // 加锁
        let inode_lock = self.inner.write();
        // 创建新文件
        let new_file =
            self.inner
                .create_lock(&self.inner, &inode_lock, name.to_string(), file_type);
        // 返回新的文件对象
        if let Ok(inner) = new_file {
            Ok(Arc::new(Self {
                readable: true,
                writable: true,
                special_use: false,
                append: false,
                inner,
                offset: Mutex::new(0),
                dirnode_ptr: Arc::new(Mutex::new(Weak::new())),
            }))
        } else {
            panic!()
        }
    }
    fn link_child(&self, name: &str, child: &Self) -> Result<(), isize>
    where
        Self: Sized,
    {
        let par_inode_lock = self.inner.write();
        let child_inode_lock = child.inner.write();
        if child
            .inner
            .link_par_lock(
                &child_inode_lock,
                &self.inner,
                &par_inode_lock,
                name.to_string(),
            )
            .is_err()
        {
            panic!();
        }
        Ok(())
    }
    fn unlink(&self, delete: bool) -> Result<(), isize> {
        let inode_lock = self.inner.write();
        if self.inner.is_dir() && !self.inner.is_empty_dir_lock(&inode_lock) {
            return Err(ENOTEMPTY);
        }
        match self.inner.unlink_lock(&inode_lock, delete) {
            Ok(_) => Ok(()),
            Err(errno) => Err(errno),
        }
    }

    /// 获取目录项
    /// # 参数
    /// + count：要获取的目录项数量
    /// # 返回值
    /// + 获取到的目录项数组/向量
    fn get_dirent(&self, count: usize) -> Vec<Dirent> {
        // 定义三个Dirent常量
        const DT_UNKNOWN: u8 = 0;
        const DT_DIR: u8 = 4;
        const DT_REG: u8 = 8;

        assert!(self.inner.is_dir());
        // 获取当前偏移量
        let mut offset = self.offset.lock();
        // 获取inode锁
        let inode_lock = self.inner.write();
        log::debug!(
            "[get_dirent] tot size: {}, offset: {}, count: {}",
            self.inner.get_file_size_wlock(&inode_lock),
            offset,
            count
        );
        // 通过调用dirent_info_lock获取元组项
        let vec = self
            .inner
            .dirent_info_lock(
                &inode_lock,
                *offset as u32,
                count / core::mem::size_of::<Dirent>(),
            )
            .unwrap();
        // 获取最后一个目录项距离下一个目录项的偏移量
        // fat32下并不是一次性获取完，而是分很多次
        if let Some((_, next_offset, _, _)) = vec.last() {
            *offset = *next_offset;
        }
        // 迭代vec来获取需要的目录项
        vec.iter()
            .map(|(name, offset, first_clus, type_)| {
                let d_type = match type_ {
                    FATDiskInodeType::AttrDirectory | FATDiskInodeType::AttrVolumeID => DT_DIR,
                    FATDiskInodeType::AttrArchive => DT_REG,
                    _ => DT_UNKNOWN,
                };
                Dirent::new(
                    *first_clus as usize,
                    *offset as isize,
                    d_type,
                    name.as_str(),
                )
            })
            .collect()
    }
    fn lseek(&self, offset: isize, whence: SeekWhence) -> Result<usize, isize> {
        let inode_lock = self.inner.write();
        let new_offset = match whence {
            SeekWhence::SEEK_SET => offset,
            SeekWhence::SEEK_CUR => *self.offset.lock() as isize + offset,
            SeekWhence::SEEK_END => self.inner.get_file_size_wlock(&inode_lock) as isize + offset,
            // whence is duplicated
            _ => return Err(EINVAL),
        };
        let new_offset = match new_offset < 0 {
            true => return Err(EINVAL),
            false => new_offset as usize,
        };
        *self.offset.lock() = new_offset;
        Ok(new_offset)
    }
    fn modify_size(&self, diff: isize) -> Result<(), isize> {
        let inode_lock = self.inner.write();
        self.inner.modify_size_lock(&inode_lock, diff, true);
        Ok(())
    }
    fn truncate_size(&self, new_size: usize) -> Result<(), isize> {
        let inode_lock = self.inner.write();
        let old_size = self.inner.get_file_size_wlock(&inode_lock);
        self.inner
            .modify_size_lock(&inode_lock, new_size as isize - old_size as isize, true);
        Ok(())
    }
    fn set_timestamp(&self, ctime: Option<usize>, atime: Option<usize>, mtime: Option<usize>) {
        let mut inode_time = self.inner.time();
        if let Some(ctime) = ctime {
            inode_time.set_create_time(ctime as u64);
        }
        if let Some(atime) = atime {
            inode_time.set_access_time(atime as u64);
        }
        if let Some(mtime) = mtime {
            inode_time.set_modify_time(mtime as u64);
        }
    }
    fn get_single_cache(&self, offset: usize) -> Result<Arc<Mutex<PageCache>>, ()> {
        // 确保偏移量4KB对齐
        if offset & 0xfff != 0 {
            return Err(());
        }
        let inode_lock = self.inner.read();
        let inner_cache_id = offset >> 12;
        Ok(self
            .inner
            .get_single_cache_lock(&inode_lock, inner_cache_id))
    }
    /// 获取所有缓存
    /// # 返回值
    /// + 所有缓存
    fn get_all_caches(&self) -> Result<Vec<Arc<Mutex<PageCache>>>, ()> {
        Ok(self.inner.get_all_cache())
    }
    fn oom(&self) -> usize {
        self.inner.oom()
    }

    fn hang_up(&self) -> bool {
        todo!()
    }

    fn fcntl(&self, cmd: u32, arg: u32) -> isize {
        todo!()
    }
}

// 文件或者目录
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DiskInodeType {
    File,
    Directory,
    FIFO,
    Character,
    Block,
    Socket,
    Link,
}
