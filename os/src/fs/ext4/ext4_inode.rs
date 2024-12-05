use crate::fs::inode::InodeTrait;

use super::{EXT4_INODE_MODE_PERM_MASK, EXT4_INODE_MODE_TYPE_MASK};

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

    fn get_file_size_rlock(&self, _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>) -> u32 {
        todo!()
    }

    fn get_file_size_wlock(&self, _inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>) -> u32 {
        todo!()
    }

    fn is_dir(&self) -> bool {
        todo!()
    }

    fn is_file(&self) -> bool {
        todo!()
    }

    fn get_inode_num_lock(&self, lock: &spin::RwLockReadGuard<crate::fs::fat32::fat_inode::FileContent>) -> Option<u32> {
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

    fn get_single_cache(&self, inner_cache_id: usize) -> alloc::sync::Arc<spin::Mutex<super::PageCache>> {
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
    ) -> alloc::vec::Vec<(alloc::string::String, crate::fs::fat32::layout::FATShortDirEnt, u32)> {
        todo!()
    }

    fn dirent_info_lock(
        &self,
        inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>,
        offset: u32,
        length: usize,
    ) -> Result<alloc::vec::Vec<(alloc::string::String, usize, u64, crate::fs::fat32::layout::FATDiskInodeType)>, ()> {
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

    fn stat_lock(&self, _inode_lock: &spin::RwLockReadGuard<crate::fs::inode::InodeLock>) -> (i64, i64, i64, i64, u64) {
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

    fn is_empty_dir_lock(&self, inode_lock: &spin::RwLockWriteGuard<crate::fs::inode::InodeLock>) -> bool {
        todo!()
    }

    fn from_ent(parent_dir: &alloc::sync::Arc<Self>, ent: &crate::fs::fat32::layout::FATShortDirEnt, offset: u32) -> alloc::sync::Arc<Self> {
        todo!()
    }
}