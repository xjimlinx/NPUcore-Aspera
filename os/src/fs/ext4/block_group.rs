use core::panic;

use super::{
    crc::{ext4_crc32c, EXT4_CRC32_INIT},
    superblock::Ext4Superblock,
    BlockDevice, EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE, EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE,
};
use crate::math::is_power_of;
use alloc::{sync::Arc, vec::Vec};
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
    /// # 参数
    /// + block_device: 块设备对象
    /// + super_block: 超级块
    /// + block_group_idx: 块组号/索引
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
        // 计算块内偏移量
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
    /// 获取块组的块位图块号
    pub fn get_block_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.block_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.block_bitmap_hi as u64) << 32;
        }
        v
    }

    /// 获取块组的Inode节点位图块号
    pub fn get_inode_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.inode_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.inode_bitmap_hi as u64) << 32;
        }
        v
    }

    /// 获取块组中未使用的Inode节点数
    pub fn get_itable_unused(&mut self, s: &Ext4Superblock) -> u32 {
        let mut v = self.itable_unused_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.itable_unused_hi as u64) << 32) as u32;
        }
        v
    }

    /// 获取块组中使用的目录数
    pub fn get_used_dirs_count(&self, s: &Ext4Superblock) -> u32 {
        let mut v = self.used_dirs_count_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.used_dirs_count_hi as u64) << 32) as u32;
        }
        v
    }

    /// 设置块组中使用的目录数
    pub fn set_used_dirs_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// 设置块组中未使用的Inode节点数
    pub fn set_itable_unused(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// 设置块组中空闲的Inode节点数
    pub fn set_free_inodes_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.free_inodes_count_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.free_inodes_count_hi = (cnt >> 16) as u16;
        }
    }

    /// 获取块组中空闲的Inode节点数
    pub fn get_free_inodes_count(&self) -> u32 {
        ((self.free_inodes_count_hi as u64) << 32) as u32 | self.free_inodes_count_lo as u32
    }

    /// 获取块组的Inode节点表块号
    pub fn get_inode_table_blk_num(&self) -> u32 {
        ((self.inode_table_first_block_hi as u64) << 32) as u32 | self.inode_table_first_block_lo
    }
}

/// 同步块组到磁盘
impl Ext4BlockGroup {
    /// 计算并返回块组描述符的校验和。
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
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 0x40) };

        // bg checksum
        checksum = ext4_crc32c(checksum, self_bytes, desc_size as u32);

        self.checksum = orig_checksum;

        (checksum & 0xFFFF) as u16
    }

    /// 将块组数据同步到磁盘。
    /// # 参数
    /// + `block_device`: 块设备对象
    /// + `bgid`: 块组号
    /// + `super_block`: 超级块
    pub fn sync_block_group_to_disk(
        &self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        // 获取每块上组描述符的数量
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        // let dsc_per_block = dsc_cnt;
        // 获取块组描述符在第几个块
        let dsc_id = bgid / dsc_cnt;
        // let first_meta_bg = super_block.first_meta_bg;
        let first_data_block = super_block.first_data_block;
        // 计算块组描述符在第几个块
        let block_id = first_data_block as usize + dsc_id + 1;
        // 计算偏移量
        let offset = (bgid % dsc_cnt) * super_block.desc_size as usize;

        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Ext4BlockGroup>(),
            )
        };

        // 确保数据不会超出块大小
        if offset + data.len() >= BLOCK_SIZE {
            panic!("Data exceeds block size");
        }

        // 因为是块组描述符，所以不会超过一个块
        // 先获取要写入的块
        let mut origin_block_data = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut origin_block_data);
        // 然后按偏移量将数据覆写到读取的块数据
        for i in offset..offset+data.len() {
            origin_block_data[i] = data[i - offset];
        }
        // origin_block_data[offset..offset + data.len()].copy_from_slice(data);
        // 最后写入块
        block_device.write_block(block_id, &origin_block_data);
        // block_device.write_block(block_id, buf);
    }

    /// 设置块组描述符的校验和。
    pub fn set_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) {
        let csum = self.get_block_group_checksum(bgid, super_block);
        self.checksum = csum;
    }

    /// 将块组数据与校验和同步到磁盘。
    pub fn sync_to_disk_with_csum(
        &mut self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        // 设置校验和
        self.set_block_group_checksum(bgid as u32, super_block);
        // 同步块组描述符到磁盘
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

/// 块
pub struct Block {
    // 在磁盘上的偏移量
    pub disk_offset: usize,
    // 数据，大小为BLOCK_SIZE
    pub data: Vec<u8>,
}

#[allow(dead_code)]
impl Block {
    /// 使用块号加载一个块
    pub fn load_id(block_device: Arc<dyn BlockDevice>, block_id: usize, offset: usize) -> Self {
        let mut buf = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut buf);
        let data = buf.to_vec();
        Block {
            disk_offset: offset,
            data,
        }
    }
    /// 使用偏移量加载一个块
    /// # 说明
    /// + 通过 offset/BLOCK_SIZE 获取 block_id 也即块号
    /// + 然后调用load_id
    pub fn load_offset(block_device: Arc<dyn BlockDevice>, offset: usize) -> Self {
        // let mut buf = [0u8; BLOCK_SIZE];
        // block_device.read_block(offset, &mut buf);
        // let data = buf.to_vec();
        // Block {
        //     disk_offset: offset,
        //     data,
        // }
        let block_id = offset / BLOCK_SIZE;
        Self::load_id(block_device, block_id, offset)
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
            ptr.read_unaligned()
        }
    }

    // 将读到的块作为指定的类型，同时附带一个偏移量
    pub fn read_offset_as<T>(&self, offset: usize) -> T {
        unsafe {
            let offset = offset % BLOCK_SIZE;
            let ptr = self.data.as_ptr().add(offset) as *const T;
            ptr.read_unaligned()
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
            let offset = offset % BLOCK_SIZE;
            let ptr = self.data.as_mut_ptr().add(offset) as *mut T;
            &mut *ptr
        }
    }

    /// 将数据写入到块
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
    /// 考虑根据len找到最后一个块，读取最后一个块之后，再分批次写入
    /// 同时也需要读取第一个块
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        if self.data.len() % BLOCK_SIZE != 0 {
            panic!(
                "[todo fix the write_offset function] write_length is not a multiple of BLOCK_SIZE"
            )
        }
        if self.disk_offset % BLOCK_SIZE != 0 {
            panic!(
                "[todo fix the write_offset function] write_offset is not a multiple of BLOCK_SIZE"
            )
        }
        let block_id = self.disk_offset / BLOCK_SIZE;
        block_device.write_block(block_id, &self.data);
    }
}

impl Ext4BlockGroup {
    pub fn dump_block_group_info(
        &self,
        blk_grp_idx: usize,
        blk_per_grp: usize,
        ino_table_len: usize,
    ) {
        fn check_if_has_superblock(blk_grp_idx: usize) -> bool {
            // 只有在第0个块组中才有主超级块
            if blk_grp_idx == 0 {
                return true;
            }
            // 其余的在第1或者3、5、7的幂次数块组中有备份超级块
            if blk_grp_idx == 1 {
                return true;
            }
            if is_power_of(blk_grp_idx as u64, 3) {
                return true;
            }
            if is_power_of(blk_grp_idx as u64, 5) {
                return true;
            }
            if is_power_of(blk_grp_idx as u64, 7) {
                return true;
            }
            return false;
        }
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
        let blk_grp_start = blk_grp_idx * blk_per_grp;
        let blk_grp_end = blk_grp_start + blk_per_grp - 1;
        println!(
            "Group {}: (blocks {}-{}) checksum 0x{:x} [ITABLE_ZEROED]",
            blk_grp_idx, blk_grp_start, blk_grp_end, checksum
        );
        if blk_grp_idx == 0 {
            println!("Main superblock is located at block 0, group descriptor at blocks 1-1");
            // println!("Reserved GDT blocks are located at blocks 39-1062");
        } else if check_if_has_superblock(blk_grp_idx) {
            println!(
                "Backup superblock is located at block {}, group descriptor at blocks {}-{}",
                blk_grp_idx,
                blk_grp_start + 1,
                blk_grp_start + 1
            );
            // println!("Reserved GDT blocks are located at blocks 39-1062");
        }
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
            inode_table + ino_table_len as u64 - 1,
            inode_table
        );
        println!(
            "{} free blocks, {} free inodes, {} directories",
            free_blocks, free_inodes, used_dirs
        );
    }
}
