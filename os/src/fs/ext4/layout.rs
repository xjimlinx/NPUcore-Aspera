#![allow(unused)]
use crate::{
    copy_from_name1, copy_to_name1,
    fs::{
        directory_tree::DirectoryTreeNode, file_trait::File, inode::InodeTrait, vfs::VFS,
        DiskInodeType, OpenFlags,
    },
    lang_items::Bytes,
};
use alloc::{
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::{Mutex, RwLock};

use core::{
    convert::TryInto,
    fmt::Debug,
    mem,
    ptr::{addr_of, addr_of_mut},
};

use super::{
    ext4fs::Ext4FileSystem,
    file::{Ext4FileContent, Ext4FileContentWrapper},
    Ext4Inode,
};

// 可能后续会用到？
pub enum ExtType {
    Ext2,
    Ext3,
    Ext4,
}

// 对Ext4Inode的一层封装，用于构成与OSInode同级别的结构体
pub struct Ext4OSInode {
    /// 是否可读
    readable: bool,
    /// 是否可写
    writable: bool,
    /// 被进程使用的计数
    special_use: bool,
    /// 是否追加
    append: bool,
    /// 具体的Inode
    inode: Arc<Ext4Inode>,
    /// 文件偏移
    offset: Mutex<usize>,
    /// 目录树节点指针
    dirnode_ptr: Arc<Mutex<Weak<DirectoryTreeNode>>>,
    /// ext4fs实例
    ext4fs: Arc<Ext4FileSystem>,
    // 文件
    file_content_wrapper: Arc<Ext4FileContentWrapper>,
}

impl Ext4OSInode {
    pub fn new(
        root_inode: Arc<Ext4Inode>,
        ext4fs: Arc<Ext4FileSystem>,
        file_content_wrapper: Arc<Ext4FileContentWrapper>,
    ) -> Arc<dyn File> {
        Arc::new(Self {
            readable: true,
            writable: true,
            special_use: true,
            append: false,
            inode: root_inode,
            offset: Mutex::new(0),
            dirnode_ptr: Arc::new(Mutex::new(Weak::new())),
            ext4fs,
            file_content_wrapper,
        })
    }
}

impl Ext4OSInode {
    pub fn first_root_inode(ext4fs: &Arc<dyn VFS>) -> Arc<dyn File> {
        let ext4fs_concrete = Arc::downcast::<Ext4FileSystem>(ext4fs.clone()).unwrap();
        // 先获取ROOT_INODE

        // let ext4_root_inode = Ext4OSInode::new(root_inode, ext4fs_concrete, file_content_wrapper);
        todo!()
    }
}

impl Drop for Ext4OSInode {
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
impl File for Ext4OSInode {
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
            inode: self.inode.clone(),
            offset: Mutex::new(*self.offset.lock()),
            dirnode_ptr: self.dirnode_ptr.clone(),
            ext4fs: self.ext4fs.clone(),
            file_content_wrapper: self.file_content_wrapper.clone(),
        })
    }

    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, offset: Option<&mut usize>, buffer: &mut [u8]) -> usize {
        match offset {
            Some(offset) => {
                let len = self.inode.read_at_block_cache(*offset, buffer);
                *offset += len;
                len
            }
            None => {
                let mut offset = self.offset.lock();
                let len = self.inode.read_at_block_cache(*offset, buffer);
                *offset += len;
                len
            }
        }
    }

    fn write(&self, offset: Option<&mut usize>, buf: &[u8]) -> usize {
        todo!()
    }

    fn r_ready(&self) -> bool {
        true
    }

    fn w_ready(&self) -> bool {
        true
    }

    fn read_user(&self, offset: Option<usize>, buf: crate::mm::UserBuffer) -> usize {
        todo!()
    }

    fn write_user(&self, offset: Option<usize>, buf: crate::mm::UserBuffer) -> usize {
        todo!()
    }

    fn get_size(&self) -> usize {
        self.inode.get_file_size() as usize
    }

    fn get_stat(&self) -> crate::fs::Stat {
        todo!()
    }

    fn get_file_type(&self) -> DiskInodeType {
        self.inode.get_file_type()
    }

    fn info_dirtree_node(&self, dirnode_ptr: Weak<DirectoryTreeNode>) {
        *self.dirnode_ptr.lock() = dirnode_ptr;
    }

    fn get_dirtree_node(&self) -> Option<Arc<DirectoryTreeNode>> {
        self.dirnode_ptr.lock().upgrade()
    }

    fn open(&self, flags: OpenFlags, special_use: bool) -> Arc<dyn File> {
        todo!()
    }

    fn open_subfile(&self) -> Result<Vec<(String, Arc<dyn File>)>, isize> {
        todo!()
    }

    fn create(
        &self,
        name: &str,
        file_type: crate::fs::DiskInodeType,
    ) -> Result<Arc<dyn File>, isize> {
        todo!()
    }

    fn link_child(&self, name: &str, child: &Self) -> Result<(), isize>
    where
        Self: Sized,
    {
        todo!()
    }

    fn unlink(&self, delete: bool) -> Result<(), isize> {
        todo!()
    }

    fn get_dirent(&self, count: usize) -> Vec<crate::fs::Dirent> {
        todo!()
    }

    fn lseek(&self, offset: isize, whence: crate::fs::SeekWhence) -> Result<usize, isize> {
        todo!()
    }

    fn modify_size(&self, diff: isize) -> Result<(), isize> {
        todo!()
    }

    fn truncate_size(&self, new_size: usize) -> Result<(), isize> {
        todo!()
    }

    fn set_timestamp(&self, ctime: Option<usize>, atime: Option<usize>, mtime: Option<usize>) {
        let atime = atime.unwrap();
        let ctime = ctime.unwrap();
        let mtime = mtime.unwrap();
        // TODO:
        todo!()
        // self.inode.set_atime(atime as u32);
        // self.inode.set_ctime(ctime as u32);
        // self.inode.set_mtime(mtime as u32);
    }

    fn get_single_cache(&self, offset: usize) -> Result<Arc<Mutex<super::PageCache>>, ()> {
        todo!()
    }

    fn get_all_caches(&self) -> Result<Vec<Arc<Mutex<super::PageCache>>>, ()> {
        todo!()
    }

    /// 这个先不考虑实现
    fn oom(&self) -> usize {
        todo!()
    }

    /// 这个也一样
    fn hang_up(&self) -> bool {
        todo!()
    }

    /// 这个也一样
    fn fcntl(&self, cmd: u32, arg: u32) -> isize {
        todo!()
    }
}

impl Ext4Inode {}
