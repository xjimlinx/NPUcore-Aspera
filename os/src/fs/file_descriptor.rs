use super::{
    cache::PageCache, directory_tree::DirectoryTreeNode, dirent::Dirent, file_trait::File, Statx,
};
use crate::{
    config::SYSTEM_FD_LIMIT,
    mm::{Frame, UserBuffer},
    syscall::errno::*,
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::slice::{Iter, IterMut};
use spin::Mutex;

use super::layout::{OpenFlags, SeekWhence, Stat};

#[derive(Clone)]
pub struct FileDescriptor {
    cloexec: bool,
    nonblock: bool,
    pub file: Arc<dyn File>,
}

#[allow(unused)]
impl FileDescriptor {
    pub fn new(cloexec: bool, nonblock: bool, file: Arc<dyn File>) -> Self {
        Self {
            cloexec,
            nonblock,
            file,
        }
    }
    pub fn set_cloexec(&mut self, flag: bool) {
        self.cloexec = flag;
    }
    pub fn get_cloexec(&self) -> bool {
        self.cloexec
    }

    pub fn get_nonblock(&self) -> bool {
        self.nonblock
    }

    pub fn get_cwd(&self) -> Option<String> {
        let inode = self.file.get_dirtree_node();
        let inode = match inode {
            Some(inode) => inode,
            None => return None,
        };
        Some(inode.get_cwd())
    }
    /// Just used for cwd
    pub fn cd(&self, path: &str) -> Result<Arc<Self>, isize> {
        match self.open(path, OpenFlags::O_DIRECTORY | OpenFlags::O_RDONLY, true) {
            Ok(fd) => Ok(Arc::new(fd)),
            Err(errno) => Err(errno),
        }
    }
    pub fn readable(&self) -> bool {
        self.file.readable()
    }
    pub fn writable(&self) -> bool {
        self.file.writable()
    }
    pub fn read(&self, offset: Option<&mut usize>, buf: &mut [u8]) -> usize {
        self.file.read(offset, buf)
    }
    pub fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
        self.file.write(offset, buf)
    }
    pub fn r_ready(&self) -> bool {
        self.file.r_ready()
    }
    pub fn w_ready(&self) -> bool {
        self.file.w_ready()
    }
    pub fn read_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        self.file.read_user(offset, buf)
    }
    pub fn write_user(&self, offset: Option<usize>, buf: UserBuffer) -> usize {
        self.file.write_user(offset, buf)
    }
    pub fn get_stat(&self) -> Stat {
        self.file.get_stat()
    }
    pub fn get_statx(&self, mask: u32) -> Statx {
        let stat = self.file.get_stat();
        Statx::new(
            mask,
            stat.get_nlink(),
            stat.get_mode() as u16,
            stat.get_ino() as u64,
            stat.get_size() as u64,
            stat.get_atime() as i64,
            stat.get_ctime() as i64,
            stat.get_mtime() as i64,
            (stat.get_rdev() & 0xffff_00) >> 8 as u32,
            (stat.get_rdev() & 0xff) as u32,
            (stat.get_dev() & 0xffff_00) >> 8 as u32,
            (stat.get_dev() & 0xff) as u32,
        )
    }
    pub fn open(&self, path: &str, flags: OpenFlags, special_use: bool) -> Result<Self, isize> {
        if path == "" {
            return Ok(self.clone());
        }
        if self.file.is_file() && !path.starts_with('/') {
            return Err(ENOTDIR);
        }
        let inode = self.file.get_dirtree_node();
        let inode = match inode {
            Some(inode) => inode,
            None => return Err(ENOENT),
        };
        let file = match inode.open(path, flags, special_use) {
            Ok(file) => file,
            Err(errno) => return Err(errno),
        };
        let cloexec = flags.contains(OpenFlags::O_CLOEXEC);
        Ok(Self::new(cloexec, false, file))
    }
    pub fn mkdir(&self, path: &str) -> Result<(), isize> {
        if self.file.is_file() && !path.starts_with('/') {
            return Err(ENOTDIR);
        }
        let inode = self.file.get_dirtree_node();
        let inode = match inode {
            Some(inode) => inode,
            None => return Err(ENOENT),
        };
        inode.mkdir(path)
    }
    pub fn delete(&self, path: &str, delete_directory: bool) -> Result<(), isize> {
        if self.file.is_file() && !path.starts_with('/') {
            return Err(ENOTDIR);
        }
        let inode = self.file.get_dirtree_node();
        let inode = match inode {
            Some(inode) => inode,
            None => return Err(ENOENT),
        };
        inode.delete(path, delete_directory)
    }
    pub fn rename(
        old_fd: &Self,
        old_path: &str,
        new_fd: &Self,
        new_path: &str,
    ) -> Result<(), isize> {
        if old_fd.file.is_file() && !old_path.starts_with('/') {
            return Err(ENOTDIR);
        }
        if new_fd.file.is_file() && !new_path.starts_with('/') {
            return Err(ENOTDIR);
        }
        let old_inode = old_fd.file.get_dirtree_node();
        let old_inode = match old_inode {
            Some(inode) => inode,
            None => return Err(ENOENT),
        };
        let new_inode = new_fd.file.get_dirtree_node();
        let new_inode = match new_inode {
            Some(inode) => inode,
            None => return Err(ENOENT),
        };

        let old_abs_path = [old_inode.get_cwd(), old_path.to_string()].join("/");
        let new_abs_path = [new_inode.get_cwd(), new_path.to_string()].join("/");
        DirectoryTreeNode::rename(&old_abs_path, &new_abs_path)
    }

    /// 获取目录项数组
    /// # 参数
    /// + count：要获取的目录项数量
    pub fn get_dirent(&self, count: usize) -> Result<Vec<Dirent>, isize> {
        // 非目录，没有目录项，直接返回
        if !self.file.is_dir() {
            return Err(ENOTDIR);
        }
        // 调用file的get_dirent方法
        Ok(self.file.get_dirent(count))
    }
    pub fn get_offset(&self) -> usize {
        self.lseek(0, SeekWhence::SEEK_CUR).unwrap()
    }
    pub fn lseek(&self, offset: isize, whence: SeekWhence) -> Result<usize, isize> {
        self.file.lseek(offset, whence)
    }
    pub fn get_size(&self) -> usize {
        self.file.get_size()
    }
    pub fn modify_size(&self, diff: isize) -> Result<(), isize> {
        self.file.modify_size(diff)
    }
    pub fn truncate_size(&self, new_size: isize) -> Result<(), isize> {
        if new_size < 0 || !self.writable() {
            return Err(EINVAL);
        }
        // todo: support ETXTBSY
        self.file.truncate_size(new_size as usize)
    }
    pub fn set_timestamp(
        &self,
        ctime: Option<usize>,
        atime: Option<usize>,
        mtime: Option<usize>,
    ) -> Result<(), isize> {
        self.file.set_timestamp(ctime, atime, mtime);
        Ok(())
    }
    pub fn get_single_cache(&self, offset: usize) -> Result<Arc<Mutex<PageCache>>, ()> {
        self.file.get_single_cache(offset)
    }
    /// 获取文件的所有缓存
    pub fn get_all_caches(&self) -> Result<Vec<Arc<Mutex<PageCache>>>, ()> {
        self.file.get_all_caches()
    }
    pub fn ioctl(&self, cmd: u32, argp: usize) -> isize {
        self.file.ioctl(cmd, argp)
    }
    // for execve
    /// 映射到内核空间
    /// # 参数
    /// + addr: 地址
    /// # 返回值
    /// 返回一个指向字节数组的静态引用，
    /// 该数组在整个程序生命周期有效，
    /// 返回的切片允许访问映射到内存中的数据
    pub fn map_to_kernel_space(&self, addr: usize) -> &'static [u8] {
        // 获取缓存数据
        let caches = self.get_all_caches().unwrap();
        // 获取内存帧
        let frames = caches
            .iter()
            .map(|cache| Frame::InMemory(cache.try_lock().unwrap().get_tracker()))
            .collect();

        // 映射到内核空间
        crate::mm::KERNEL_SPACE
            .lock()
            .insert_program_area(
                addr.into(),
                crate::mm::MapPermission::R | crate::mm::MapPermission::W,
                frames,
            )
            .unwrap();
        unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, self.get_size()) }
    }
}
/// ### 文件描述符表
#[derive(Clone)]
pub struct FdTable {
    // 文件描述符 数组
    inner: Vec<Option<FileDescriptor>>,
    // 已回收的文件描述符
    recycled: Vec<u8>,
    soft_limit: usize,
    hard_limit: usize,
}

/// ### 文件描述符表的方法
#[allow(unused)]
impl FdTable {
    pub const DEFAULT_FD_LIMIT: usize = 128;
    pub const SYSTEM_FD_LIMIT: usize = SYSTEM_FD_LIMIT;
    pub fn new(inner: Vec<Option<FileDescriptor>>) -> Self {
        Self {
            inner,
            recycled: Vec::new(),
            soft_limit: FdTable::DEFAULT_FD_LIMIT,
            hard_limit: FdTable::SYSTEM_FD_LIMIT,
        }
    }
    pub fn get_soft_limit(&self) -> usize {
        self.soft_limit
    }
    pub fn set_soft_limit(&mut self, limit: usize) {
        if limit < self.inner.len() {
            log::warn!(
                "[FdTable::set_soft_limit] new limit: {} is smaller than current table length: {}",
                self.inner.len(),
                self.soft_limit
            );
            self.inner.truncate(limit);
            self.recycled.retain(|&fd| (fd as usize) < limit);
        }
        self.soft_limit = limit;
    }
    pub fn get_hard_limit(&self) -> usize {
        self.hard_limit
    }
    pub fn set_hard_limit(&mut self, limit: usize) {
        if limit < self.inner.len() {
            log::warn!(
                "[FdTable::set_hard_limit] new limit: {} is smaller than current table length: {}",
                self.inner.len(),
                self.soft_limit
            );
            self.inner.truncate(limit);
            self.recycled.retain(|&fd| (fd as usize) < limit);
        }
        self.hard_limit = limit;
    }
    #[inline]
    pub fn get_ref(&self, fd: usize) -> Result<&FileDescriptor, isize> {
        if fd >= self.inner.len() {
            return Err(EBADF);
        }
        match &self.inner[fd] {
            Some(file_descriptor) => Ok(file_descriptor),
            None => Err(EBADF),
        }
    }
    #[inline]
    pub fn get_refmut(&mut self, fd: usize) -> Result<&mut FileDescriptor, isize> {
        if fd >= self.inner.len() {
            return Err(EBADF);
        }
        match &mut self.inner[fd] {
            Some(file_descriptor) => Ok(file_descriptor),
            None => Err(EBADF),
        }
    }
    #[inline]
    pub fn remove(&mut self, fd: usize) -> Result<FileDescriptor, isize> {
        if fd >= self.inner.len() {
            return Err(EBADF);
        }
        match self.inner[fd].take() {
            Some(file_descriptor) => {
                self.recycled.push(fd as u8);
                Ok(file_descriptor)
            }
            None => Err(EBADF),
        }
    }
    #[inline(always)]
    pub fn iter(&self) -> Iter<Option<FileDescriptor>> {
        self.inner.iter()
    }
    #[inline(always)]
    pub fn iter_mut(&mut self) -> IterMut<Option<FileDescriptor>> {
        self.inner.iter_mut()
    }
    /// check if `fd` is valid
    #[inline]
    pub fn check(&self, fd: usize) -> Result<(), isize> {
        if fd >= self.inner.len() || self.inner[fd].is_none() {
            return Err(EBADF);
        }
        Ok(())
    }
    pub fn find_min(&mut self) -> Option<u8> {
        if let Some(&min_value) = self.recycled.iter().min() {
            if let Some(index) = self.recycled.iter().position(|&x| x == min_value) {
                self.recycled.remove(index);
                Some(min_value)
            } else {
                None
            }
        } else {
            None
        }
    }
    #[inline]
    pub fn insert(&mut self, file_descriptor: FileDescriptor) -> Result<usize, isize> {
        // 直接pop fd省事，但是初赛openat测例要求新的fd>旧的，改为find_min，每次取最小的fd
        // let fd = match self.recycled.pop() {
        let fd = match self.find_min(){
            Some(fd) => {
                self.inner[fd as usize] = Some(file_descriptor);
                fd as usize
            }
            None => {
                let current = self.inner.len();
                if current == self.soft_limit {
                    return Err(EMFILE);
                } else {
                    self.inner.push(Some(file_descriptor));
                    current
                }
            }
        };
        Ok(fd)
    }

    /// insert at `pos`, if there is an existing fd, it will be replaced.
    #[inline]
    pub fn insert_at(
        &mut self,
        file_descriptor: FileDescriptor,
        pos: usize,
    ) -> Result<usize, isize> {
        let current = self.inner.len();
        if pos < current {
            if self.inner[pos].is_none() {
                self.recycled.retain(|&fd| fd as usize != pos);
            }
            self.inner[pos] = Some(file_descriptor);
            Ok(pos)
        } else {
            if pos >= self.soft_limit {
                return Err(EMFILE);
            } else {
                (current..pos)
                    .rev()
                    .for_each(|fd| self.recycled.push(fd as u8));
                self.inner.resize(pos, None);
                self.inner.push(Some(file_descriptor));
                Ok(pos)
            }
        }
    }

    /// try to insert at the lowest-numbered available fd greater than or equal to `hint`(no replace)
    #[inline]
    pub fn try_insert_at(
        &mut self,
        file_descriptor: FileDescriptor,
        hint: usize,
    ) -> Result<usize, isize> {
        if hint >= self.soft_limit {
            return Err(EMFILE);
        }
        let current = self.inner.len();
        if hint < current {
            match self.inner[hint] {
                Some(_) => match self.recycled.iter().copied().find(|&fd| fd as usize > hint) {
                    Some(fd) => {
                        self.inner[fd as usize] = Some(file_descriptor);
                        Ok(fd as usize)
                    }
                    None => {
                        if current == self.soft_limit {
                            return Err(EMFILE);
                        } else {
                            self.inner.push(Some(file_descriptor));
                            Ok(current)
                        }
                    }
                },
                None => {
                    self.recycled.retain(|&fd| fd as usize != hint);
                    self.inner[hint] = Some(file_descriptor);
                    Ok(hint)
                }
            }
        } else {
            if hint >= self.soft_limit {
                return Err(EMFILE);
            } else {
                (current..hint).for_each(|fd| self.recycled.push(fd as u8));
                self.inner.resize(hint, None);
                self.inner.push(Some(file_descriptor));
                Ok(hint)
            }
        }
    }
    /// Take the ownership of the given fd
    pub fn take(&mut self, fd: usize) -> Option<FileDescriptor> {
        if fd >= self.inner.len() {
            None
        } else {
            self.inner[fd].take()
        }
    }
}