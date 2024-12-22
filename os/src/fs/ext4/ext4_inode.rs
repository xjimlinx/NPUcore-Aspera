use core::cmp::min;

use alloc::{sync::Arc, vec::Vec};

use crate::fs::{inode::InodeTrait, vfs::VFS};

use super::*;
use super::{
    bitmap::{ext4_bmap_bit_find_clr, ext4_bmap_bit_set},
    block_group::{Block, Ext4BlockGroup},
    crc::{ext4_crc32c, EXT4_CRC32_INIT},
    direntry::DirEntryType,
    error::Errno,
    ext4fs::Ext4FileSystem,
    extent::{Ext4Extent, Ext4ExtentHeader, Ext4ExtentIndex},
    superblock::Ext4Superblock,
    BlockDevice,
};

bitflags! {
    pub struct InodeFileType: u16 {
        const S_IFIFO = 0x1000;
        const S_IFCHR = 0x2000;
        const S_IFDIR = 0x4000;
        const S_IFBLK = 0x6000;
        const S_IFREG = 0x8000;
        const S_IFSOCK = 0xC000;
        const S_IFLNK = 0xA000;
    }
}

bitflags! {
    pub struct InodePerm: u16 {
        const S_IREAD = 0x0100;
        const S_IWRITE = 0x0080;
        const S_IEXEC = 0x0040;
        const S_ISUID = 0x0800;
        const S_ISGID = 0x0400;
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Inode {
    pub mode: u16,        // 文件类型和权限
    pub uid: u16,         // 所有者用户 ID
    pub size: u32,        // 低 32 位文件大小
    pub atime: u32,       // 最近访问时间
    pub ctime: u32,       // 创建时间
    pub mtime: u32,       // 最近修改时间
    pub dtime: u32,       // 删除时间
    pub gid: u16,         // 所有者组 ID
    pub links_count: u16, // 链接计数
    pub blocks: u32,      // 已分配的块数
    pub flags: u32,       // 文件标志
    pub osd1: u32,        // 操作系统相关的字段1
    pub block: [u32; 15], // 数据块指针
    pub generation: u32,  // 文件版本（NFS）
    pub file_acl: u32,    // 文件 ACL
    pub size_hi: u32,     // 高 32 位文件大小
    pub faddr: u32,       // 已废弃的碎片地址
    pub osd2: Linux2,     // 操作系统相关的字段2

    pub i_extra_isize: u16,  // 额外的 inode 大小
    pub i_checksum_hi: u16,  // 高位校验和（crc32c(uuid+inum+inode) BE）
    pub i_ctime_extra: u32,  // 额外的创建时间（纳秒 << 2 | 纪元）
    pub i_mtime_extra: u32,  // 额外的修改时间（纳秒 << 2 | 纪元）
    pub i_atime_extra: u32,  // 额外的访问时间（纳秒 << 2 | 纪元）
    pub i_crtime: u32,       // 创建时间
    pub i_crtime_extra: u32, // 额外的创建时间（纳秒 << 2 | 纪元）
    pub i_version_hi: u32,   // 高 32 位版本
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
    pub l_i_blocks_high: u16,   // 高 16 位已分配块数
    pub l_i_file_acl_high: u16, // 高 16 位文件 ACL
    pub l_i_uid_high: u16,      // 高 16 位用户 ID
    pub l_i_gid_high: u16,      // 高 16 位组 ID
    pub l_i_checksum_lo: u16,   // 低位校验和
    pub l_i_reserved: u16,      // 保留字段
}

impl Ext4Inode {
    pub fn root_inode(ext4fs: &Arc<dyn VFS>) -> Arc<Self> {
        let ext4fs = Arc::downcast::<Ext4FileSystem>(ext4fs.clone()).unwrap();
        todo!()
        // 尝试获取根目录的Inode节点
    }

    pub fn mode(&self) -> u16 {
        self.mode
    }

    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
    }

    pub fn uid(&self) -> u16 {
        self.uid
    }

    pub fn set_uid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn size(&self) -> u64 {
        self.size as u64 | ((self.size_hi as u64) << 32)
    }

    pub fn set_size(&mut self, size: u64) {
        self.size = (size & 0xffffffff) as u32;
        self.size_hi = (size >> 32) as u32;
    }

    pub fn atime(&self) -> u32 {
        self.atime
    }

    pub fn set_atime(&mut self, atime: u32) {
        self.atime = atime;
    }

    pub fn ctime(&self) -> u32 {
        self.ctime
    }

    pub fn set_ctime(&mut self, ctime: u32) {
        self.ctime = ctime;
    }

    pub fn mtime(&self) -> u32 {
        self.mtime
    }

    pub fn set_mtime(&mut self, mtime: u32) {
        self.mtime = mtime;
    }

    pub fn dtime(&self) -> u32 {
        self.dtime
    }

    pub fn set_dtime(&mut self, dtime: u32) {
        self.dtime = dtime;
    }

    pub fn gid(&self) -> u16 {
        self.gid
    }

    pub fn set_gid(&mut self, gid: u16) {
        self.gid = gid;
    }

    pub fn links_count(&self) -> u16 {
        self.links_count
    }

    pub fn set_links_count(&mut self, links_count: u16) {
        self.links_count = links_count;
    }

    pub fn blocks_count(&self) -> u64 {
        let mut blocks = self.blocks as u64;
        if self.osd2.l_i_blocks_high != 0 {
            blocks |= (self.osd2.l_i_blocks_high as u64) << 32;
        }
        blocks
    }

    pub fn set_blocks_count(&mut self, blocks: u64) {
        self.blocks = (blocks & 0xFFFFFFFF) as u32;
        self.osd2.l_i_blocks_high = (blocks >> 32) as u16;
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }

    pub fn osd1(&self) -> u32 {
        self.osd1
    }

    pub fn set_osd1(&mut self, osd1: u32) {
        self.osd1 = osd1;
    }

    pub fn block(&self) -> [u32; 15] {
        self.block
    }

    pub fn set_block(&mut self, block: [u32; 15]) {
        self.block = block;
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn file_acl(&self) -> u32 {
        self.file_acl
    }

    pub fn set_file_acl(&mut self, file_acl: u32) {
        self.file_acl = file_acl;
    }

    pub fn size_hi(&self) -> u32 {
        self.size_hi
    }

    pub fn set_size_hi(&mut self, size_hi: u32) {
        self.size_hi = size_hi;
    }

    pub fn faddr(&self) -> u32 {
        self.faddr
    }

    pub fn set_faddr(&mut self, faddr: u32) {
        self.faddr = faddr;
    }

    pub fn osd2(&self) -> Linux2 {
        self.osd2
    }

    pub fn set_osd2(&mut self, osd2: Linux2) {
        self.osd2 = osd2;
    }

    pub fn i_extra_isize(&self) -> u16 {
        self.i_extra_isize
    }

    pub fn set_i_extra_isize(&mut self, i_extra_isize: u16) {
        self.i_extra_isize = i_extra_isize;
    }

    pub fn i_checksum_hi(&self) -> u16 {
        self.i_checksum_hi
    }

    pub fn set_i_checksum_hi(&mut self, i_checksum_hi: u16) {
        self.i_checksum_hi = i_checksum_hi;
    }

    pub fn i_ctime_extra(&self) -> u32 {
        self.i_ctime_extra
    }

    pub fn set_i_ctime_extra(&mut self, i_ctime_extra: u32) {
        self.i_ctime_extra = i_ctime_extra;
    }

    pub fn i_mtime_extra(&self) -> u32 {
        self.i_mtime_extra
    }

    pub fn set_i_mtime_extra(&mut self, i_mtime_extra: u32) {
        self.i_mtime_extra = i_mtime_extra;
    }

    pub fn i_atime_extra(&self) -> u32 {
        self.i_atime_extra
    }

    pub fn set_i_atime_extra(&mut self, i_atime_extra: u32) {
        self.i_atime_extra = i_atime_extra;
    }

    pub fn i_crtime(&self) -> u32 {
        self.i_crtime
    }

    pub fn set_i_crtime(&mut self, i_crtime: u32) {
        self.i_crtime = i_crtime;
    }

    pub fn i_crtime_extra(&self) -> u32 {
        self.i_crtime_extra
    }

    pub fn set_i_crtime_extra(&mut self, i_crtime_extra: u32) {
        self.i_crtime_extra = i_crtime_extra;
    }

    pub fn i_version_hi(&self) -> u32 {
        self.i_version_hi
    }

    pub fn set_i_version_hi(&mut self, i_version_hi: u32) {
        self.i_version_hi = i_version_hi;
    }
}

impl Ext4Inode {
    pub fn file_type(&self) -> InodeFileType {
        InodeFileType::from_bits_truncate(self.mode & EXT4_INODE_MODE_TYPE_MASK)
    }

    pub fn file_perm(&self) -> InodePerm {
        InodePerm::from_bits_truncate(self.mode & EXT4_INODE_MODE_PERM_MASK)
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == InodeFileType::S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.file_type() == InodeFileType::S_IFREG
    }

    pub fn is_link(&self) -> bool {
        self.file_type() == InodeFileType::S_IFLNK
    }

    pub fn can_read(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IREAD)
    }

    pub fn can_write(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IWRITE)
    }

    pub fn can_exec(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IEXEC)
    }

    pub fn set_file_type(&mut self, kind: InodeFileType) {
        self.mode |= kind.bits();
    }

    pub fn set_file_perm(&mut self, perm: InodePerm) {
        self.mode |= perm.bits();
    }
}

#[derive(Clone, Debug)]
pub struct Ext4InodeRef {
    pub inode_num: u32,
    pub inode: Ext4Inode,
}

impl Ext4Inode {
    /// Get the depth of the extent tree from an inode.
    pub fn root_header_depth(&self) -> u16 {
        self.root_extent_header().depth
    }

    pub fn root_extent_header_ref(&self) -> &Ext4ExtentHeader {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { &*header_ptr }
    }

    pub fn root_extent_header(&self) -> Ext4ExtentHeader {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { *header_ptr }
    }

    pub fn root_extent_header_mut(&mut self) -> &mut Ext4ExtentHeader {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *header_ptr }
    }

    pub fn root_extent_mut_at(&mut self, pos: usize) -> &mut Ext4Extent {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *(header_ptr.add(1) as *mut Ext4Extent).add(pos) }
    }

    pub fn root_extent_ref_at(&mut self, pos: usize) -> &Ext4Extent {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { &*(header_ptr.add(1) as *const Ext4Extent).add(pos) }
    }

    pub fn root_extent_at(&mut self, pos: usize) -> Ext4Extent {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { *(header_ptr.add(1) as *const Ext4Extent).add(pos) }
    }

    pub fn root_first_index_mut(&mut self) -> &mut Ext4ExtentIndex {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *(header_ptr.add(1) as *mut Ext4ExtentIndex) }
    }

    pub fn extent_tree_init(&mut self) {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe {
            (*header_ptr).set_magic();
            (*header_ptr).set_entries_count(0);
            (*header_ptr).set_max_entries_count(4); // 假设最大条目数为 4
            (*header_ptr).set_depth(0);
            (*header_ptr).set_generation(0);
        }
    }

    fn get_checksum(&self, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size;
        let mut v: u32 = self.osd2.l_i_checksum_lo as u32;
        if inode_size > 128 {
            v |= (self.i_checksum_hi as u32) << 16;
        }
        v
    }
    #[allow(unused)]
    pub fn set_inode_checksum_value(
        &mut self,
        super_block: &Ext4Superblock,
        inode_id: u32,
        checksum: u32,
    ) {
        let inode_size = super_block.inode_size();

        self.osd2.l_i_checksum_lo = (checksum & 0xffff) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }
    fn copy_to_slice(&self, slice: &mut [u8]) {
        unsafe {
            let inode_ptr = self as *const Ext4Inode as *const u8;
            let array_ptr = slice.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
        }
    }
    #[allow(unused)]
    pub fn get_inode_checksum(&mut self, inode_id: u32, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size();

        let orig_checksum = self.get_checksum(super_block);
        let mut checksum = 0;

        let ino_index = inode_id as u32;
        let ino_gen = self.generation;

        // Preparation: temporarily set bg checksum to 0
        self.osd2.l_i_checksum_lo = 0;
        self.i_checksum_hi = 0;

        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );
        checksum = ext4_crc32c(checksum, &ino_index.to_le_bytes(), 4);
        checksum = ext4_crc32c(checksum, &ino_gen.to_le_bytes(), 4);

        let mut raw_data = [0u8; 0x100];
        self.copy_to_slice(&mut raw_data);

        // inode checksum
        checksum = ext4_crc32c(checksum, &raw_data, inode_size as u32);

        self.set_inode_checksum_value(super_block, inode_id, checksum);

        if inode_size == 128 {
            checksum &= 0xFFFF;
        }

        checksum
    }

    pub fn set_inode_checksum(&mut self, super_block: &Ext4Superblock, inode_id: u32) {
        let inode_size = super_block.inode_size();
        let checksum = self.get_inode_checksum(inode_id, super_block);

        self.osd2.l_i_checksum_lo = ((checksum << 16) >> 16) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }

    pub fn sync_inode_to_disk(&self, block_device: Arc<dyn BlockDevice>, inode_pos: usize) {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::intrinsics::size_of::<Ext4Inode>(),
            )
        };
        block_device.write_block(inode_pos, data);
    }
}

#[allow(unused)]
impl InodeTrait for Ext4Inode {
    fn read(&self) -> spin::RwLockReadGuard<crate::fs::inode::InodeLock> {
        todo!()
    }

    fn write(&self) -> spin::RwLockWriteGuard<crate::fs::inode::InodeLock> {
        todo!()
    }

    fn get_file_type_lock(&self) -> spin::MutexGuard<crate::fs::DiskInodeType> {
        todo!()
    }

    fn get_file_type(&self) -> crate::fs::DiskInodeType {
        todo!()
    }

    fn get_file_size(&self) -> u32 {
        todo!()
    }

    fn get_file_size_rlock(
        &self,
        _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>,
    ) -> u32 {
        todo!()
    }

    fn get_file_size_wlock(
        &self,
        _inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
    ) -> u32 {
        todo!()
    }

    fn is_dir(&self) -> bool {
        todo!()
    }

    fn is_file(&self) -> bool {
        todo!()
    }

    fn get_inode_num_lock(
        &self,
        lock: &spin::RwLockReadGuard<crate::fs::fat32::fat_inode::FileContent>,
    ) -> Option<u32> {
        todo!()
    }

    fn get_block_id(
        &self,
        lock: &spin::RwLockReadGuard<crate::fs::fat32::fat_inode::FileContent>,
        inner_cache_id: u32,
    ) -> Option<u32> {
        todo!()
    }

    fn read_at_block_cache_rlock(
        &self,
        _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize {
        todo!()
    }

    fn read_at_block_cache_wlock(
        &self,
        _inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        offset: usize,
        buf: &mut [u8],
    ) -> usize {
        todo!()
    }

    fn read_at_block_cache(&self, offset: usize, buf: &mut [u8]) -> usize {
        todo!()
    }

    fn write_at_block_cache_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        offset: usize,
        buf: &[u8],
    ) -> usize {
        todo!()
    }

    fn write_at_block_cache(&self, offset: usize, buf: &[u8]) -> usize {
        todo!()
    }

    fn get_single_cache(
        &self,
        inner_cache_id: usize,
    ) -> alloc::sync::Arc<spin::Mutex<super::PageCache>> {
        todo!()
    }

    fn get_single_cache_lock(
        &self,
        _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>,
        inner_cache_id: usize,
    ) -> alloc::sync::Arc<spin::Mutex<super::PageCache>> {
        todo!()
    }

    fn get_all_cache(&self) -> alloc::vec::Vec<alloc::sync::Arc<spin::Mutex<super::PageCache>>> {
        todo!()
    }

    fn get_all_files_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
    ) -> alloc::vec::Vec<(
        alloc::string::String,
        crate::fs::fat32::layout::FATShortDirEnt,
        u32,
    )> {
        todo!()
    }

    fn dirent_info_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        offset: u32,
        length: usize,
    ) -> Result<
        alloc::vec::Vec<(
            alloc::string::String,
            usize,
            u64,
            crate::fs::fat32::layout::FATDiskInodeType,
        )>,
        (),
    > {
        todo!()
    }

    fn delete_self_dir_ent(&self) -> Result<(), ()> {
        todo!()
    }

    fn unlink_lock(
        &self,
        _inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        delete: bool,
    ) -> Result<(), isize> {
        todo!()
    }

    fn stat_lock(
        &self,
        _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>,
    ) -> (i64, i64, i64, i64, u64) {
        todo!()
    }

    fn time(&self) -> spin::MutexGuard<crate::fs::inode::InodeTime> {
        todo!()
    }

    fn oom(&self) -> usize {
        todo!()
    }

    fn modify_size_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        diff: isize,
        clear: bool,
    ) {
        todo!()
    }

    fn is_empty_dir_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
    ) -> bool {
        todo!()
    }

    fn from_ent(
        &self,
        parent_dir: &alloc::sync::Arc<dyn InodeTrait>,
        ent: &crate::fs::fat32::layout::FATShortDirEnt,
        offset: u32,
    ) -> alloc::sync::Arc<dyn InodeTrait> {
        todo!()
    }

    fn link_par_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        parent_dir: &alloc::sync::Arc<dyn InodeTrait>,
        parent_inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        name: alloc::string::String,
    ) -> Result<(), ()>
    where
        Self: Sized,
    {
        todo!()
    }
    #[allow(unused)]
    fn create_lock(
        &self,
        parent_dir: &alloc::sync::Arc<dyn InodeTrait>,
        parent_inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        name: alloc::string::String,
        file_type: crate::fs::DiskInodeType,
    ) -> Result<alloc::sync::Arc<dyn InodeTrait>, ()>
    where
        Self: Sized,
    {
        todo!()
    }

    fn gen_short_name_slice(
        parent_dir: &alloc::sync::Arc<Self>,
        parent_inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        name: &alloc::string::String,
    ) -> [u8; 11]
    where
        Self: Sized,
    {
        todo!()
    }

    fn gen_name_slice(
        parent_dir: &alloc::sync::Arc<Self>,
        parent_inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        name: &alloc::string::String,
    ) -> ([u8; 11], alloc::vec::Vec<[u16; 13]>)
    where
        Self: Sized,
    {
        todo!()
    }

    fn gen_long_name_slice(name: &alloc::string::String, long_ent_index: usize) -> [u16; 13]
    where
        Self: Sized,
    {
        todo!()
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn root_inode(efs: &Arc<dyn VFS>) -> Arc<Self> {
        Ext4Inode::root_inode(efs)
    }
}

impl Ext4FileSystem {
    pub fn get_bgid_of_inode(&self, inode_num: u32) -> u32 {
        inode_num / self.superblock.inodes_per_group()
    }

    pub fn inode_to_bgidx(&self, inode_num: u32) -> u32 {
        inode_num % self.superblock.inodes_per_group()
    }

    /// 获取Inode的地址
    /// # 参数
    /// + inode_num: Inode号
    /// # 返回值
    /// + inode在块设备上的偏移量
    pub fn inode_disk_pos(&self, inode_num: u32) -> usize {
        // 超级块
        let super_block = self.superblock;
        // 每个块组中包含的inode数量
        let inodes_per_group = super_block.inodes_per_group;
        // inode大小
        let inode_size = super_block.inode_size as u64;
        // 组号
        let group = (inode_num - 1) / inodes_per_group;
        // 块组内索引
        let index = (inode_num - 1) % inodes_per_group;
        // 加载块组描述符
        let block_group =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, group as usize);
        // for temporary test
        println!("\n\nFor temporary test, should be removed\n\n");
        block_group.dump_block_group_info(
            group as usize,
            super_block.blocks_per_group() as usize,
            1024,
        );
        // 获取inode表块号
        let inode_table_blk_num = block_group.get_inode_table_blk_num();
        println!("Current inode_table_blk_num is {}", inode_table_blk_num);
        // 计算字节偏移量
        // 这个计算公式可能有问题
        println!("index is {} and inode_size is {}", index, inode_size);
        let offset =
            inode_table_blk_num as usize * BLOCK_SIZE + index as usize * inode_size as usize;
        // inode_table_blk_num as usize * BLOCK_SIZE + (index as usize + 1) * inode_size as usize;
        offset
    }

    /// Load the inode reference from the disk.
    pub fn get_inode_ref(&self, inode_num: u32) -> Ext4InodeRef {
        let offset = self.inode_disk_pos(inode_num);
        println!(
            "[fstest] read offset in get_inode_ref is {}, and current inode_num is {}",
            offset, inode_num
        );

        // The problem is happened here
        let mut ext4block = Block::load_offset(self.block_device.clone(), offset);

        let inode: &mut Ext4Inode = ext4block.read_offset_as_mut(offset % BLOCK_SIZE);

        Ext4InodeRef {
            inode_num: inode_num,
            inode: *inode,
        }
    }

    /// write back inode with checksum
    pub fn write_back_inode(&self, inode_ref: &mut Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        // make sure self.superblock is up-to-date
        inode_ref
            .inode
            .set_inode_checksum(&self.superblock, inode_ref.inode_num);
        inode_ref
            .inode
            .sync_inode_to_disk(self.block_device.clone(), inode_pos);
    }

    /// write back inode with checksum
    pub fn write_back_inode_without_csum(&self, inode_ref: &Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        inode_ref
            .inode
            .sync_inode_to_disk(self.block_device.clone(), inode_pos);
    }

    /// Get physical block id of a logical block.
    ///
    /// Params:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id
    pub fn get_pblock_idx(
        &self,
        inode_ref: &Ext4InodeRef,
        lblock: Ext4Lblk,
    ) -> Result<Ext4Fsblk, isize> {
        let search_path = self.find_extent(&inode_ref, lblock);
        if let Ok(path) = search_path {
            // get the last path
            let path = path.path.last().unwrap();

            // get physical block id
            let fblock = path.pblock;

            return Ok(fblock);
        }

        // return_errno_with_message(Errno::EIO, "search extent fail")
        Err(Errno::EIO as isize)
    }

    /// Allocate a new block
    pub fn allocate_new_block(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk, isize> {
        let mut super_block = self.superblock;
        let inodes_per_group = super_block.inodes_per_group();
        let bgid = (inode_ref.inode_num - 1) / inodes_per_group;
        let index = (inode_ref.inode_num - 1) % inodes_per_group;

        // load block group
        let mut block_group =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

        let block_bitmap_block = block_group.get_block_bitmap_block(&super_block);

        let mut block_bmap_raw_data = [0u8; BLOCK_SIZE];
        self.block_device
            .read_block(block_bitmap_block as usize, &mut block_bmap_raw_data);
        let mut data: &mut Vec<u8> = &mut block_bmap_raw_data.to_vec();
        let mut rel_blk_idx = 0 as u32;

        ext4_bmap_bit_find_clr(data, index as u32, 0x8000, &mut rel_blk_idx);
        ext4_bmap_bit_set(&mut data, rel_blk_idx);

        block_group.set_block_group_balloc_bitmap_csum(&super_block, &data);
        self.block_device
            .write_block(block_bitmap_block as usize, &data);

        /* Update superblock free blocks count */
        let mut super_blk_free_blocks = super_block.free_blocks_count();
        super_blk_free_blocks -= 1;
        super_block.set_free_blocks_count(super_blk_free_blocks);
        super_block.sync_to_disk_with_csum(self.block_device.clone());

        /* Update inode blocks (different block size!) count */
        let mut inode_blocks = inode_ref.inode.blocks_count();
        inode_blocks += (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE) as u64;
        inode_ref.inode.set_blocks_count(inode_blocks);
        self.write_back_inode(inode_ref);

        /* Update block group free blocks count */
        let mut fb_cnt = block_group.get_free_blocks_count();
        fb_cnt -= 1;
        block_group.set_free_blocks_count(fb_cnt as u32);
        block_group.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

        Ok(rel_blk_idx as Ext4Fsblk)
    }

    /// Append a new block to the inode and update the extent tree.
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// iblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk, isize> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block(inode_ref, None)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Append a new block to the inode and update the extent tree.From a specific bgid
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// bgid: Start bgid of free block search
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk_from(
        &self,
        inode_ref: &mut Ext4InodeRef,
        start_bgid: &mut u32,
    ) -> Result<Ext4Fsblk, isize> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block_from(inode_ref, start_bgid)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Allocate a new inode
    ///
    /// Params:
    /// inode_mode: u16 - inode mode
    ///
    /// Returns:
    /// `Result<u32>` - inode number
    pub fn alloc_inode(&self, is_dir: bool) -> Result<u32, isize> {
        // Allocate inode
        let inode_num = self.ialloc_alloc_inode(is_dir)?;

        Ok(inode_num)
    }

    pub fn correspond_inode_mode(&self, filetype: u8) -> u16 {
        let file_type = DirEntryType::from_bits(filetype).unwrap();
        match file_type {
            DirEntryType::EXT4_DE_REG_FILE => InodeFileType::S_IFREG.bits(),
            DirEntryType::EXT4_DE_DIR => InodeFileType::S_IFDIR.bits(),
            DirEntryType::EXT4_DE_SYMLINK => InodeFileType::S_IFLNK.bits(),
            DirEntryType::EXT4_DE_CHRDEV => InodeFileType::S_IFCHR.bits(),
            DirEntryType::EXT4_DE_BLKDEV => InodeFileType::S_IFBLK.bits(),
            DirEntryType::EXT4_DE_FIFO => InodeFileType::S_IFIFO.bits(),
            DirEntryType::EXT4_DE_SOCK => InodeFileType::S_IFSOCK.bits(),
            _ => {
                // FIXME: unsupported filetype
                InodeFileType::S_IFREG.bits()
            }
        }
    }
}
