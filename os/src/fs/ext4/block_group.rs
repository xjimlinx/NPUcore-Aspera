use alloc::{sync::Arc, vec::Vec};

use super::{
    crc::{ext4_crc32c, EXT4_CRC32_INIT},
    superblock::Ext4Superblock,
    BlockDevice, EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE, EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE,
};
// use crate::arch::BLOCK_SZ;
use super::BLOCK_SIZE;

#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
/// Ext4块组描述符
pub struct Ext4BlockGroup {
    pub block_bitmap_lo: u32,            // 块位图块
    pub inode_bitmap_lo: u32,            // 节点位图块
    pub inode_table_first_block_lo: u32, // 节点表块
    pub free_blocks_count_lo: u16,       // 空闲块数
    pub free_inodes_count_lo: u16,       // 空闲节点数
    pub used_dirs_count_lo: u16,         // 目录数
    pub flags: u16,                      // EXT4_BG_flags (INODE_UNINIT, etc)
    pub exclude_bitmap_lo: u32,          // 快照排除位图
    pub block_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+bbitmap) LE
    pub inode_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+ibitmap) LE
    pub itable_unused_lo: u16,           // 未使用的节点数
    pub checksum: u16,                   // crc16(sb_uuid+group+desc)

    pub block_bitmap_hi: u32,            // 块位图块 MSB
    pub inode_bitmap_hi: u32,            // 节点位图块 MSB
    pub inode_table_first_block_hi: u32, // 节点表块 MSB
    pub free_blocks_count_hi: u16,       // 空闲块数 MSB
    pub free_inodes_count_hi: u16,       // 空闲节点数 MSB
    pub used_dirs_count_hi: u16,         // 目录数 MSB
    pub itable_unused_hi: u16,           // 未使用的节点数 MSB
    pub exclude_bitmap_hi: u32,          // 快照排除位图 MSB
    pub block_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+bbitmap) BE
    pub inode_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+ibitmap) BE
    pub reserved: u32,                   // 填充
}
impl Ext4BlockGroup {
    /// 从磁盘加载块组描述符
    pub fn load_new(
        block_device: Arc<dyn BlockDevice>,
        super_block: &Ext4Superblock,
        block_group_idx: usize,
    ) -> Self {
        // 计算一个块可以放多少块组描述符
        // 这里因为BLOCK_SIZE是2048,而块组描述符大小为64
        // 所以一个块可以放32个块组描述符
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        // 计算块组描述符在第几个块
        let dsc_id = block_group_idx / dsc_cnt;
        // 从超级块中获取第一个数据块的块号
        let first_data_block = super_block.first_data_block;
        // 计算所在块的块号
        // 加上1是因为第0块是超级块
        let block_id = first_data_block as usize + dsc_id + 1;
        // 计算偏移量
        // 计算公式为：
        // 块组中的偏移量 = (块组中的索引 % 每个块的块组描述符数量) * 块组描述符大小
        let offset = (block_group_idx % dsc_cnt) * super_block.desc_size as usize;
        // 从块设备读取块
        let ext4block = Block::load_offset(block_device, block_id * BLOCK_SIZE);
        // 使用Block的read_offset_as方法将数据读取为Ext4BlockGroup
        let bg: Ext4BlockGroup = ext4block.read_offset_as(offset);

        bg
    }
}

impl Ext4BlockGroup {
    /// Get the block number of the block bitmap for this block group.
    pub fn get_block_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.block_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.block_bitmap_hi as u64) << 32;
        }
        v
    }

    /// Get the block number of the inode bitmap for this block group.
    pub fn get_inode_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.inode_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.inode_bitmap_hi as u64) << 32;
        }
        v
    }

    /// Get the count of unused inodes in this block group.
    pub fn get_itable_unused(&mut self, s: &Ext4Superblock) -> u32 {
        let mut v = self.itable_unused_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.itable_unused_hi as u64) << 32) as u32;
        }
        v
    }

    /// Get the count of used directories in this block group.
    pub fn get_used_dirs_count(&self, s: &Ext4Superblock) -> u32 {
        let mut v = self.used_dirs_count_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.used_dirs_count_hi as u64) << 32) as u32;
        }
        v
    }

    /// Set the count of used directories in this block group.
    pub fn set_used_dirs_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// Set the count of unused inodes in this block group.
    pub fn set_itable_unused(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// Set the count of free inodes in this block group.
    pub fn set_free_inodes_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.free_inodes_count_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.free_inodes_count_hi = (cnt >> 16) as u16;
        }
    }

    /// Get the count of free inodes in this block group.
    pub fn get_free_inodes_count(&self) -> u32 {
        ((self.free_inodes_count_hi as u64) << 32) as u32 | self.free_inodes_count_lo as u32
    }

    /// Get the block number of the inode table for this block group.
    pub fn get_inode_table_blk_num(&self) -> u32 {
        ((self.inode_table_first_block_hi as u64) << 32) as u32 | self.inode_table_first_block_lo
    }
}

/// 同步块组到磁盘
impl Ext4BlockGroup {
    /// Calculate and return the checksum of the block group descriptor.
    #[allow(unused)]
    pub fn get_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) -> u16 {
        let desc_size = super_block.desc_size();

        let mut orig_checksum = 0;
        let mut checksum = 0;

        orig_checksum = self.checksum;

        // 准备：暂时将bg校验和设为0
        self.checksum = 0;

        // uuid checksum
        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );

        // bgid checksum
        checksum = ext4_crc32c(checksum, &bgid.to_le_bytes(), 4);

        // cast self to &[u8]
        let self_bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 0x40 as usize) };

        // bg checksum
        checksum = ext4_crc32c(checksum, self_bytes, desc_size as u32);

        self.checksum = orig_checksum;

        let crc = (checksum & 0xFFFF) as u16;

        crc
    }

    /// Synchronize the block group data to disk.
    pub fn sync_block_group_to_disk(
        &self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        // let dsc_per_block = dsc_cnt;
        let dsc_id = bgid / dsc_cnt;
        // let first_meta_bg = super_block.first_meta_bg;
        let first_data_block = super_block.first_data_block;
        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (bgid % dsc_cnt) * super_block.desc_size as usize;

        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Ext4BlockGroup>(),
            )
        };
        block_device.write_block(block_id * BLOCK_SIZE + offset, data);
    }

    /// Set the checksum of the block group descriptor.
    pub fn set_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) {
        let csum = self.get_block_group_checksum(bgid, super_block);
        self.checksum = csum;
    }

    /// Synchronize the block group data to disk with checksum.
    pub fn sync_to_disk_with_csum(
        &mut self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        self.set_block_group_checksum(bgid as u32, super_block);
        self.sync_block_group_to_disk(block_device, bgid, super_block)
    }

    /// Set the block allocation bitmap checksum for this block group.
    pub fn set_block_group_balloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = s.ext4_balloc_bitmap_csum(bitmap);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.block_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.block_bitmap_csum_hi = hi_csum as u16;
        }
    }

    /// Get the count of free blocks in this block group.
    pub fn get_free_blocks_count(&self) -> u64 {
        let mut v = self.free_blocks_count_lo as u64;
        if self.free_blocks_count_hi != 0 {
            v |= (self.free_blocks_count_hi as u64) << 32;
        }
        v
    }

    /// Set the count of free blocks in this block group.
    pub fn set_free_blocks_count(&mut self, cnt: u32) {
        self.free_blocks_count_lo = (cnt & 0xffff) as u16;
        self.free_blocks_count_hi = (cnt >> 16) as u16;
    }

    /// Set the inode allocation bitmap checksum for this block group.
    pub fn set_block_group_ialloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = s.ext4_ialloc_bitmap_csum(bitmap);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.inode_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.inode_bitmap_csum_hi = hi_csum as u16;
        }
    }
}

pub struct Block {
    pub disk_offset: usize,
    pub data: Vec<u8>,
}

impl Block {
    // 从块设备读取块
    pub fn load_offset(block_device: Arc<dyn BlockDevice>, offset: usize) -> Self {
        // let mut buf = [0u8; BLOCK_SIZE];
        // block_device.read_block(offset, &mut buf);
        // let data = buf.to_vec();
        // Block {
        //     disk_offset: offset,
        //     data,
        // }
        let block_id = offset / BLOCK_SIZE;
        Self::load(block_device, block_id)
    }
    pub fn load(block_device: Arc<dyn BlockDevice>, block_id: usize) -> Self {
        let mut buf = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut buf);
        let data = buf.to_vec();
        Block {
            disk_offset: block_id * BLOCK_SIZE,
            data,
        }
    }
    // 从inode块读取块
    pub fn load_inode_root_block(data: &[u32; 15]) -> Self {
        let data_bytes: &[u8; 60] = unsafe { core::mem::transmute(data) };
        Block {
            disk_offset: 0,
            data: data_bytes.to_vec(),
        }
    }

    // 将读到的块作为指定的类型
    pub fn read_as<T>(&self) -> T {
        unsafe {
            let ptr = self.data.as_ptr() as *const T;
            let value = ptr.read_unaligned();
            value
        }
    }

    // 将读到的块作为指定的类型，同时附带一个偏移量
    pub fn read_offset_as<T>(&self, offset: usize) -> T {
        unsafe {
            let ptr = self.data.as_ptr().add(offset) as *const T;
            let value = ptr.read_unaligned();
            value
        }
    }

    // 将读到的块作为指定的类型，并且返回一个可变引用
    pub fn read_as_mut<T>(&mut self) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            &mut *ptr
        }
    }

    // 将读到的块作为指定的类型，同时附带一个偏移量，并且返回一个可变引用
    pub fn read_offset_as_mut<T>(&mut self, offset: usize) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr().add(offset) as *mut T;
            &mut *ptr
        }
    }

    // 将数据写入到块设备
    // pub fn write_offset(&self, block_device: Arc<dyn BlockDevice>) {
    //     block_device.write_block(self.disk_offset, &self.data);
    // }
    pub fn write_offset(&mut self, offset: usize, data: &[u8], len: usize) {
        let end = offset + len;
        if end <= self.data.len() {
            let slice_end = len.min(data.len());
            self.data[offset..end].copy_from_slice(&data[..slice_end]);
        } else {
            panic!("Write would overflow the block buffer");
        }
    }

    // 同步内存上的数据到块设备
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        block_device.write_block(self.disk_offset, &self.data);
    }
}

impl Ext4BlockGroup {
    pub fn dump_block_group_info(&self, blk_grp_idx: usize, blk_per_grp: usize) {
        // Function to combine low and high parts of the fields, now supports u16 and u32
        fn lo_hi_add_u16(lo: u16, hi: u16, shift: u32) -> u64 {
            lo as u64 + ((hi as u64) << shift)
        }

        fn lo_hi_add_u32(lo: u32, hi: u32, shift: u32) -> u64 {
            lo as u64 + ((hi as u64) << shift)
        }

        // Calculate the block bitmap, inode bitmap, and inode table (with high and low parts combined)
        let block_bitmap = lo_hi_add_u32(self.block_bitmap_lo, self.block_bitmap_hi, 32);
        let inode_bitmap = lo_hi_add_u32(self.inode_bitmap_lo, self.inode_bitmap_hi, 32);

        // Use the lo_hi_add_u32 function for inode_table with a shift of 32
        let inode_table = lo_hi_add_u32(
            self.inode_table_first_block_lo,
            self.inode_table_first_block_hi,
            32,
        );

        // Use the lo_hi_add_u16 function for free_blocks, free_inodes, and used_dirs with a shift of 16
        let free_blocks = lo_hi_add_u16(self.free_blocks_count_lo, self.free_blocks_count_hi, 16);
        let free_inodes = lo_hi_add_u16(self.free_inodes_count_lo, self.free_inodes_count_hi, 16);
        let used_dirs = lo_hi_add_u16(self.used_dirs_count_lo, self.used_dirs_count_hi, 16);
        let checksum = self.checksum;
        let block_bitmap_csum =
            lo_hi_add_u16(self.block_bitmap_csum_lo, self.block_bitmap_csum_hi, 16);
        let inode_bitmap_csum =
            lo_hi_add_u16(self.inode_bitmap_csum_lo, self.inode_bitmap_csum_hi, 16);
        // Print out block group information similar to `dump2fs`
        println!(
            "Group 0: (blocks 0-32767) checksum 0x{:x} [ITABLE_ZEROED]",
            checksum
        );
        println!("Main superblock is located at block 0, group descriptor at blocks 1-38");
        println!("Reserved GDT blocks are located at blocks 39-1062");
        println!(
            "Block bitmap is at block {} (+{}), checksum 0x{:x}",
            block_bitmap, block_bitmap, block_bitmap_csum
        );
        println!(
            "Inode bitmap is at block {} (+{}), checksum 0x{:x}",
            inode_bitmap, inode_bitmap, inode_bitmap_csum
        );
        println!(
            "Inode table is at blocks {}-{} (+{})",
            inode_table,
            inode_table + 511,
            inode_table
        );
        println!(
            "{} free blocks, {} free inodes, {} directories",
            free_blocks, free_inodes, used_dirs
        );
    }
}
