use core::{convert::TryFrom, fmt::Debug, intrinsics::size_of};

use super::block_group::Block;
use super::ext4fs::Ext4FileSystem;
use super::*;
use super::{crc::*, superblock::Ext4Superblock};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use error::{Errno, Ext4Error};

bitflags! {
    // #[derive(PartialEq, Eq)]
    pub struct DirEntryType: u8 {
        const EXT4_DE_UNKNOWN = 0;
        const EXT4_DE_REG_FILE = 1;
        const EXT4_DE_DIR = 2;
        const EXT4_DE_CHRDEV = 3;
        const EXT4_DE_BLKDEV = 4;
        const EXT4_DE_FIFO = 5;
        const EXT4_DE_SOCK = 6;
        const EXT4_DE_SYMLINK = 7;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// 目录项结构体
pub struct Ext4DirEntry {
    pub inode: u32,               // 该目录项指向的inode的编号
    pub entry_len: u16,           // 到下一个目录项的距离
    pub name_len: u8,             // 低8位的文件名长度
    pub inner: Ext4DirEnInternal, // Union体成员
    pub name: [u8; 255],          // 文件名
}

/// Internal directory entry structure.
#[repr(C)]
#[derive(Clone, Copy)]
pub union Ext4DirEnInternal {
    pub name_length_high: u8, // 高8位的文件名长度
    pub inode_type: u8,       // 引用的inode的类型（在rev >= 0.5中）
}

/// Fake directory entry structure. Used for directory entry iteration.
#[repr(C)]
pub struct Ext4FakeDirEntry {
    inode: u32,
    entry_length: u16,
    name_length: u8,
    inode_type: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntryTail {
    pub reserved_zero1: u32,
    pub rec_len: u16,
    pub reserved_zero2: u8,
    pub reserved_ft: u8,
    pub checksum: u32, // crc32c(uuid+inum+dirblock)
}

pub struct Ext4DirSearchResult {
    pub dentry: Ext4DirEntry,
    pub pblock_id: usize,   // disk block id
    pub offset: usize,      // offset in block
    pub prev_offset: usize, //prev direntry offset
}

impl Ext4DirSearchResult {
    pub fn new(dentry: Ext4DirEntry) -> Self {
        Self {
            dentry,
            pblock_id: 0,
            offset: 0,
            prev_offset: 0,
        }
    }
}

impl Debug for Ext4DirEnInternal {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unsafe {
            write!(
                f,
                "Ext4DirEnInternal {{ name_length_high: {:?} }}",
                self.name_length_high
            )
        }
    }
}

impl Default for Ext4DirEnInternal {
    fn default() -> Self {
        Self {
            name_length_high: 0,
        }
    }
}

impl Default for Ext4DirEntry {
    fn default() -> Self {
        Self {
            inode: 0,
            entry_len: 0,
            name_len: 0,
            inner: Ext4DirEnInternal::default(),
            name: [0; 255],
        }
    }
}

impl<T> TryFrom<&[T]> for Ext4DirEntry {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

#[allow(unused)]
/// Directory entry implementation.
impl Ext4DirEntry {
    /// Check if the directory entry is unused.
    pub fn unused(&self) -> bool {
        self.inode == 0
    }

    /// Set the directory entry as unused.
    pub fn set_unused(&mut self) {
        self.inode = 0
    }

    /// Check name
    pub fn compare_name(&self, name: &str) -> bool {
        if self.name_len as usize == name.len() {
            return &self.name[..name.len()] == name.as_bytes();
        }
        false
    }

    /// Entry length
    pub fn entry_len(&self) -> u16 {
        self.entry_len
    }

    /// Dir type
    pub fn get_de_type(&self) -> u8 {
        unsafe { self.inner.inode_type }
    }

    /// Get name to string
    pub fn get_name(&self) -> String {
        let name_len = self.name_len as usize;
        let name = &self.name[..name_len];
        let name = core::str::from_utf8(name).unwrap();
        name.to_string()
    }

    /// Get name len
    pub fn get_name_len(&self) -> usize {
        self.name_len as usize
    }

    /// 计算目录项的实际使用长度（不包括填充字节）
    pub fn actual_len(&self) -> usize {
        size_of::<Ext4FakeDirEntry>() + self.name_len as usize
    }

    /// 计算对齐后的目录项长度（包括填充字节）
    pub fn used_len_aligned(&self) -> usize {
        let mut len = self.actual_len();
        if len % 4 != 0 {
            len += 4 - (len % 4);
        }
        len
    }

    pub fn write_entry(&mut self, entry_len: u16, inode: u32, name: &str, de_type: DirEntryType) {
        self.inode = inode;
        self.entry_len = entry_len;
        self.name_len = name.len() as u8;
        self.inner.inode_type = de_type.bits();
        self.name[..name.len()].copy_from_slice(name.as_bytes());
    }
}

impl Ext4DirEntry {
    /// Get the checksum of the directory entry.
    #[allow(unused)]
    pub fn ext4_dir_get_csum(&self, s: &Ext4Superblock, blk_data: &[u8], ino_gen: u32) -> u32 {
        let ino_index = self.inode;

        let mut csum = 0;

        let uuid = s.uuid;

        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
        csum = ext4_crc32c(csum, &ino_index.to_le_bytes(), 4);
        csum = ext4_crc32c(csum, &ino_gen.to_le_bytes(), 4);
        let mut data = [0u8; 0xff4];
        unsafe {
            core::ptr::copy_nonoverlapping(blk_data.as_ptr(), data.as_mut_ptr(), blk_data.len());
        }

        csum = ext4_crc32c(csum, &data[..], 0xff4);
        csum
    }

    /// Write de to block
    pub fn write_de_to_blk(&self, dst_blk: &mut Block, offset: usize) {
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        let data = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, count) };
        dst_blk.data.splice(
            offset..offset + core::mem::size_of::<Ext4DirEntry>(),
            data.iter().cloned(),
        );
        // assert_eq!(dst_blk.block_data[offset..offset + core::mem::size_of::<Ext4DirEntry>()], data[..]);
    }

    /// Copy the directory entry to a slice.
    pub fn copy_to_slice(&self, array: &mut [u8], offset: usize) {
        let de_ptr = self as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        unsafe {
            core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
        }
    }
}

impl Ext4DirEntry {
}

impl Ext4DirEntryTail {
    pub fn new() -> Self {
        Self {
            reserved_zero1: 0,
            rec_len: size_of::<Ext4DirEntryTail>() as u16,
            reserved_zero2: 0,
            reserved_ft: 0xDE,
            checksum: 0,
        }
    }

    pub fn tail_set_csum(
        &mut self,
        s: &Ext4Superblock,
        diren: &Ext4DirEntry,
        blk_data: &[u8],
        ino_gen: u32,
    ) {
        let csum = diren.ext4_dir_get_csum(s, blk_data, ino_gen);
        self.checksum = csum;
    }

    pub fn copy_to_slice(&self, array: &mut [u8]) {
        unsafe {
            let offset = BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>();
            let de_ptr = self as *const Ext4DirEntryTail as *const u8;
            let array_ptr = array as *mut [u8] as *mut u8;
            let count = core::mem::size_of::<Ext4DirEntryTail>();
            core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
        }
    }
}

#[allow(unused)]
impl Ext4FileSystem {
    /// Find a directory entry in a directory
    ///
    /// Params:
    /// parent_inode: u32 - inode number of the parent directory
    /// name: &str - name of the entry to find
    /// result: &mut Ext4DirSearchResult - result of the search
    ///
    /// Returns:
    /// `Result<usize>` - status of the search
    pub fn dir_find_entry(
        &self,
        parent_inode: u32,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> Result<usize, Ext4Error> {
        // 加载父目录Inode
        let parent = self.get_inode_ref(parent_inode);
        // println!("[kernel dir_find_entry] Get Parent InodeRef: {:#?}", parent);
        assert!(parent.inode.is_dir());

        // start from the first logical block
        let mut iblock = 0;
        // physical block id
        let mut fblock: Ext4Fsblk = 0;

        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let total_blocks: u64 = inode_size / BLOCK_SIZE as u64;

        // iterate all blocks
        while iblock < total_blocks {
            let search_path = self.find_extent(&parent, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // get physical block id
                fblock = path.pblock;

                // load physical block
                let mut ext4block =
                    Block::load_offset(self.block_device.clone(), fblock as usize * BLOCK_SIZE);

                // find entry in block
                let r = self.dir_find_in_block(&ext4block, name, result);

                if r.is_ok() {
                    result.pblock_id = fblock as usize;
                    return Ok(EOK);
                }
            } else {
                println!("[kernel direntry] dir search fail");
                return Err(Ext4Error::new(Errno::ENOENT));
            }
            // go to next block
            iblock += 1
        }

        println!("[kernel direntry] dir search fail");
        return Err(Ext4Error::new(Errno::ENOENT));
    }

    /// Find a directory entry in a block
    ///
    /// Params:
    /// block: &mut Block - block to search in
    /// name: &str - name of the entry to find
    ///
    /// Returns:
    /// result: Ext4DirEntry - result of the search
    pub fn dir_find_in_block(
        &self,
        block: &Block,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> Result<Ext4DirEntry, isize> {
        let mut offset = 0;
        let mut prev_de_offset = 0;

        // start from the first entry
        while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
            let de: Ext4DirEntry = block.read_offset_as(offset);
            if !de.unused() && de.compare_name(name) {
                result.dentry = de;
                result.offset = offset;
                result.prev_offset = prev_de_offset;
                return Ok(de);
            }

            prev_de_offset = offset;
            // go to next entry
            offset = offset + de.entry_len() as usize;
        }
        println!("[kernel direntry] dir find in block failed");
        return Err(Errno::ENOENT as isize);
    }

    /// 获取指定Inode的目录项
    /// # 参数
    /// + inode: u32 - 目录文件的inode号
    /// # 返回值
    /// + `Vec<Ext4DirEntry>` - 目录项列表
    pub fn dir_get_entries(&self, inode: u32) -> Vec<Ext4DirEntry> {
        let mut entries = Vec::new();

        // 加载inode
        let inode_ref = self.get_inode_ref(inode);
        // assert!(inode_ref.inode.is_dir());
        if !inode_ref.inode.is_dir() {
            return Vec::new();
        }

        // 计算总块数
        let inode_size = inode_ref.inode.size();
        let total_blocks = inode_size / BLOCK_SIZE as u64;

        // 从第一个逻辑块开始
        let mut iblock = 0;

        // 遍历所有块
        while iblock < total_blocks {
            // 获取逻辑块号对应的物理块号(此处为路径，路径中包含有物理块号)
            let search_path = self.find_extent(&inode_ref, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // 获取物理块号
                let fblock = path.pblock;

                // 加载物理块
                let ext4block =
                    Block::load_offset(self.block_device.clone(), fblock as usize * BLOCK_SIZE);
                let mut offset = 0;

                // 遍历块内所有项
                while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
                    let de: Ext4DirEntry = ext4block.read_offset_as(offset);
                    if !de.unused() {
                        entries.push(de);
                    }
                    offset += de.entry_len() as usize;
                }
            }

            // 前往下一个逻辑块
            iblock += 1;
        }
        entries
    }

    pub fn dir_get_entries_from_inode_ref(&self, inode_ref: Arc<Ext4InodeRef>) -> Vec<Ext4DirEntry> {
        let mut entries = Vec::new();

        // 加载inode
        assert!(inode_ref.inode.is_dir());

        // 计算总块数
        let inode_size = inode_ref.inode.size();
        let total_blocks = inode_size / BLOCK_SIZE as u64;

        // 从第一个逻辑块开始
        let mut iblock = 0;

        // 遍历所有块
        while iblock < total_blocks {
            // 获取逻辑块号对应的物理块号
            let search_path = self.find_extent(&inode_ref, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // 获取物理块号
                let fblock = path.pblock;

                // 加载物理块
                let ext4block =
                    Block::load_offset(self.block_device.clone(), fblock as usize * BLOCK_SIZE);
                let mut offset = 0;

                // 遍历块内所有项
                while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
                    let de: Ext4DirEntry = ext4block.read_offset_as(offset);
                    if !de.unused() {
                        entries.push(de);
                    }
                    offset += de.entry_len() as usize;
                }
            }

            // 前往下一个逻辑块
            iblock += 1;
        }
        entries
    }

    pub fn dir_set_csum(&self, dst_blk: &mut Block, ino_gen: u32) {
        let parent_de: Ext4DirEntry = dst_blk.read_offset_as(0);

        let tail_offset = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
        let mut tail: Ext4DirEntryTail = *dst_blk.read_offset_as_mut(tail_offset);

        tail.tail_set_csum(&self.superblock, &parent_de, &dst_blk.data[..], ino_gen);

        tail.copy_to_slice(&mut dst_blk.data);
    }

    /// Add a new entry to a directory
    ///
    /// Params:
    /// parent: &mut Ext4InodeRef - parent directory inode reference
    /// child: &mut Ext4InodeRef - child inode reference
    /// path: &str - path of the new entry
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn dir_add_entry(
        &self,
        parent: &mut Ext4InodeRef,
        child: &Ext4InodeRef,
        name: &str,
    ) -> Result<usize, isize> {
        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let block_size = self.superblock.block_size();
        let total_blocks: u64 = inode_size / block_size as u64;

        // iterate all blocks
        let mut iblock = 0;
        while iblock < total_blocks {
            // get physical block id of a logical block id
            let pblock = self.get_pblock_idx(parent, iblock as u32)?;

            // load physical block
            let mut ext4block =
                Block::load_offset(self.block_device.clone(), pblock as usize * BLOCK_SIZE);

            let result = self.try_insert_to_existing_block(&mut ext4block, name, child.inode_num);

            if result.is_ok() {
                // set checksum
                self.dir_set_csum(&mut ext4block, parent.inode.generation());
                ext4block.sync_blk_to_disk(self.block_device.clone());

                return Ok(EOK);
            }

            // go ot next block
            iblock += 1;
        }

        // no space in existing blocks, need to add new block
        let new_block = self.append_inode_pblk(parent)?;

        // load new block
        let mut new_ext4block =
            Block::load_offset(self.block_device.clone(), new_block as usize * BLOCK_SIZE);

        // write new entry to the new block
        // must succeed, as we just allocated the block
        let de_type = DirEntryType::EXT4_DE_DIR;
        self.insert_to_new_block(&mut new_ext4block, child.inode_num, name, de_type);

        // set checksum
        self.dir_set_csum(&mut new_ext4block, parent.inode.generation());
        new_ext4block.sync_blk_to_disk(self.block_device.clone());

        Ok(EOK)
    }

    /// Try to insert a new entry to an existing block
    ///
    /// Params:
    /// block: &mut Block - block to insert the new entry
    /// name: &str - name of the new entry
    /// inode: u32 - inode number of the new entry
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn try_insert_to_existing_block(
        &self,
        block: &mut Block,
        name: &str,
        child_inode: u32,
    ) -> Result<usize, isize> {
        // required length aligned to 4 bytes
        let required_len = {
            let mut len = size_of::<Ext4DirEntry>() + name.len();
            if len % 4 != 0 {
                len += 4 - (len % 4);
            }
            len
        };

        let mut offset = 0;

        // Start from the first entry
        while offset < BLOCK_SIZE - size_of::<Ext4DirEntryTail>() {
            let mut de = Ext4DirEntry::try_from(&block.data[offset..]).unwrap();

            if de.unused() {
                continue;
            }

            let inode = de.inode;
            let rec_len = de.entry_len;

            let used_len = de.name_len as usize;
            let mut sz = core::mem::size_of::<Ext4FakeDirEntry>() + used_len;
            if used_len % 4 != 0 {
                sz += 4 - used_len % 4;
            }

            let free_space = rec_len as usize - sz;

            // If there is enough free space
            if free_space >= required_len {
                // Create new directory entry
                let mut new_entry = Ext4DirEntry::default();

                // Update existing entry length and copy both entries back to block data
                de.entry_len = sz as u16;

                let de_type = DirEntryType::EXT4_DE_DIR;
                new_entry.write_entry(free_space as u16, child_inode, name, de_type);

                // update parent_de and new_de to blk_data
                de.copy_to_slice(&mut block.data, offset);
                new_entry.copy_to_slice(&mut block.data, offset + sz);

                // Sync to disk
                block.sync_blk_to_disk(self.block_device.clone());

                return Ok(EOK);
            }

            // Move to the next entry
            offset += de.entry_len() as usize;
        }

        println!("[kernel direntry] No space in block for new entry");
        return Err(Errno::ENOSPC as isize);
    }

    /// Insert a new entry to a new block
    ///
    /// Params:
    /// block: &mut Block - block to insert the new entry
    /// name: &str - name of the new entry
    /// inode: u32 - inode number of the new entry
    pub fn insert_to_new_block(
        &self,
        block: &mut Block,
        inode: u32,
        name: &str,
        de_type: DirEntryType,
    ) {
        // write new entry
        let mut new_entry = Ext4DirEntry::default();
        let el = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
        new_entry.write_entry(el as u16, inode, name, de_type);
        new_entry.copy_to_slice(&mut block.data, 0);

        copy_dir_entry_to_array(&new_entry, &mut block.data, 0);

        // init tail for new block
        let tail = Ext4DirEntryTail::new();
        tail.copy_to_slice(&mut block.data);
    }

    pub fn dir_remove_entry(&self, parent: &mut Ext4InodeRef, path: &str) -> Result<usize, isize> {
        // get remove_entry pos in parent and its prev entry
        let mut result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        // let r = self.dir_find_entry(parent.inode_num, path, &mut result)?;
        let r = self.dir_find_entry(parent.inode_num, path, &mut result);

        let mut ext4block =
            Block::load_offset(self.block_device.clone(), result.pblock_id * BLOCK_SIZE);

        let de_del_entry_len = result.dentry.entry_len();

        // prev entry
        let pde: &mut Ext4DirEntry = ext4block.read_offset_as_mut(result.prev_offset);

        pde.entry_len += de_del_entry_len;

        let de_del: &mut Ext4DirEntry = ext4block.read_offset_as_mut(result.offset);

        de_del.inode = 0;

        self.dir_set_csum(&mut ext4block, parent.inode.generation());
        ext4block.sync_blk_to_disk(self.block_device.clone());

        Ok(EOK)
    }

    pub fn dir_has_entry(&self, dir_inode: u32) -> bool {
        // load parent inode
        let parent = self.get_inode_ref(dir_inode);
        assert!(parent.inode.is_dir());

        // start from the first logical block
        let mut iblock = 0;
        // physical block id
        let mut fblock: Ext4Fsblk = 0;

        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let total_blocks: u64 = inode_size / BLOCK_SIZE as u64;

        // iterate all blocks
        while iblock < total_blocks {
            let search_path = self.find_extent(&parent, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // get physical block id
                fblock = path.pblock;

                // load physical block
                let ext4block =
                    Block::load_offset(self.block_device.clone(), fblock as usize * BLOCK_SIZE);

                // start from the first entry
                let mut offset = 0;
                while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
                    let de: Ext4DirEntry = ext4block.read_offset_as(offset);
                    offset = offset + de.entry_len as usize;
                    if de.inode == 0 {
                        continue;
                    }
                    // skip . and ..
                    if de.get_name() == "." || de.get_name() == ".." {
                        continue;
                    }
                    return true;
                }
            }
            // go to next block
            iblock += 1
        }

        false
    }

    pub fn dir_remove(&self, parent: u32, path: &str) -> Result<usize, isize> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        // let r = self.dir_find_entry(parent as u32, path, &mut search_result)?;
        let r = self.dir_find_entry(parent, path, &mut search_result);

        let mut parent_inode_ref = self.get_inode_ref(parent);
        let mut child_inode_ref = self.get_inode_ref(search_result.dentry.inode);

        if self.dir_has_entry(child_inode_ref.inode_num) {
            println!("[kernel] rm dir with chidren not supported");
            return Err(Errno::ENOTSUP as isize);
        }

        self.truncate_inode(&mut child_inode_ref, 0)?;

        self.unlink(&mut parent_inode_ref, &mut child_inode_ref, path)?;

        self.write_back_inode(&mut parent_inode_ref);

        // to do
        // ext4_inode_set_del_time
        // ext4_inode_set_links_cnt
        // ext4_fs_free_inode(&child)

        Ok(EOK)
    }
}

pub fn copy_dir_entry_to_array(header: &Ext4DirEntry, array: &mut [u8], offset: usize) {
    unsafe {
        let de_ptr = header as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
    }
}
