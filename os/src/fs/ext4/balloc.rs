use core::cmp::min;

use alloc::vec::Vec;

use crate::fs::ext4::bitmap::{ext4_bmap_bit_find_clr, ext4_bmap_bit_set, ext4_bmap_is_bit_clr};
use crate::fs::ext4::block_group::{Block, Ext4BlockGroup};
use crate::fs::ext4::error::Errno;

use super::bitmap::ext4_bmap_bits_free;

use super::ext4fs::Ext4FileSystem;
use super::*;

impl Ext4FileSystem {
    /// Compute number of block group from block address.
    ///
    /// Params:
    ///
    /// `baddr` - Absolute address of block.
    ///
    /// # Returns
    /// `u32` - Block group index
    pub fn get_bgid_of_block(&self, baddr: u64) -> u32 {
        let mut baddr = baddr;
        if self.superblock.first_data_block() != 0 && baddr != 0 {
            baddr -= 1;
        }
        (baddr / self.superblock.blocks_per_group() as u64) as u32
    }

    /// Compute the starting block address of a block group.
    ///
    /// Params:
    /// `bgid` - Block group index
    ///
    /// Returns:
    /// `u64` - Block address
    pub fn get_block_of_bgid(&self, bgid: u32) -> u64 {
        let mut baddr = 0;
        if self.superblock.first_data_block() != 0 {
            baddr += 1;
        }
        baddr + bgid as u64 * self.superblock.blocks_per_group() as u64
    }

    /// Convert block address to relative index in block group.
    ///
    /// Params:
    /// `baddr` - Block number to convert.
    ///
    /// Returns:
    /// `u32` - Relative number of block.
    pub fn addr_to_idx_bg(&self, baddr: u64) -> u32 {
        let mut baddr = baddr;
        if self.superblock.first_data_block() != 0 && baddr != 0 {
            baddr -= 1;
        }
        (baddr % self.superblock.blocks_per_group() as u64) as u32
    }

    /// Convert relative block address in group to absolute address.
    ///
    /// # Arguments
    ///
    /// * `index` - Relative block address.
    /// * `bgid` - Block group.
    ///
    /// # Returns
    ///
    /// * `Ext4Fsblk` - Absolute block address.
    pub fn bg_idx_to_addr(&self, index: u32, bgid: u32) -> Ext4Fsblk {
        let mut index = index;
        if self.superblock.first_data_block() != 0 {
            index += 1;
        }
        (self.superblock.blocks_per_group() as u64 * bgid as u64) + index as u64
    }

    /// Allocate a new block.
    ///
    /// Params:
    /// `inode_ref` - Reference to the inode.
    /// `goal` - Absolute address of the block.
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - The physical block number allocated.
    pub fn balloc_alloc_block(
        &self,
        inode_ref: &mut Ext4InodeRef,
        goal: Option<Ext4Fsblk>,
    ) -> Result<Ext4Fsblk, isize> {
        let mut alloc: Ext4Fsblk = 0;
        let super_block = &self.superblock;
        let blocks_per_group = super_block.blocks_per_group();
        let mut bgid;
        let mut idx_in_bg;

        if let Some(goal) = goal {
            bgid = self.get_bgid_of_block(goal);
            idx_in_bg = self.addr_to_idx_bg(goal);
        } else {
            bgid = 1;
            idx_in_bg = 0;
        }

        let block_group_count = super_block.block_group_count();
        let mut count = block_group_count;

        while count > 0 {
            // Load block group reference
            let mut block_group =
                Ext4BlockGroup::load_new(self.block_device.clone(), super_block, bgid as usize);

            let free_blocks = block_group.get_free_blocks_count();
            if free_blocks == 0 {
                // Try next block group
                bgid = (bgid + 1) % block_group_count;
                count -= 1;

                if count == 0 {
                    println!("[balloc] No free blocks available in all block groups");
                    return Err(Errno::ENOSPC as isize);
                }
                continue;
            }

            // Compute indexes
            let first_in_bg = self.get_block_of_bgid(bgid);
            let first_in_bg_index = self.addr_to_idx_bg(first_in_bg);

            if idx_in_bg < first_in_bg_index {
                idx_in_bg = first_in_bg_index;
            }

            // Load block with bitmap
            let bmp_blk_adr = block_group.get_block_bitmap_block(super_block);
            let mut bitmap_block =
                Block::load_offset(self.block_device.clone(), bmp_blk_adr as usize * BLOCK_SIZE);

            // Check if goal is free
            if ext4_bmap_is_bit_clr(&bitmap_block.data, idx_in_bg) {
                ext4_bmap_bit_set(&mut bitmap_block.data, idx_in_bg);
                block_group.set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                // 此处不需要考虑对齐
                self.block_device
                    .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                alloc = self.bg_idx_to_addr(idx_in_bg, bgid);

                /* Update free block counts */
                self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;
                return Ok(alloc);
            }

            // Try to find free block near to goal
            let blk_in_bg = blocks_per_group;
            let end_idx = min((idx_in_bg + 63) & !63, blk_in_bg);

            for tmp_idx in (idx_in_bg + 1)..end_idx {
                if ext4_bmap_is_bit_clr(&bitmap_block.data, tmp_idx) {
                    ext4_bmap_bit_set(&mut bitmap_block.data, tmp_idx);
                    block_group
                        .set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                    // 此处不需要考虑对齐
                    self.block_device
                        .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                    alloc = self.bg_idx_to_addr(tmp_idx, bgid);
                    self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;
                    return Ok(alloc);
                }
            }

            // Find free bit in bitmap
            let mut rel_blk_idx = 0;
            if ext4_bmap_bit_find_clr(&bitmap_block.data, idx_in_bg, blk_in_bg, &mut rel_blk_idx) {
                ext4_bmap_bit_set(&mut bitmap_block.data, rel_blk_idx);
                block_group.set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                // 此处不需要考虑对齐
                self.block_device
                    .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                alloc = self.bg_idx_to_addr(rel_blk_idx, bgid);
                self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;
                return Ok(alloc);
            }

            // No free block found in this group, try other block groups
            bgid = (bgid + 1) % block_group_count;
            count -= 1;
        }

        println!("No free blocks available in all block groups");
        return Err(Errno::ENOSPC as isize);
    }

    /// Allocate a new block start from a specific bgid
    ///
    /// Params:
    /// `inode_ref` - Reference to the inode.
    /// `start_bgid` - Start bgid of free block search
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - The physical block number allocated.
    pub fn balloc_alloc_block_from(
        &self,
        inode_ref: &mut Ext4InodeRef,
        start_bgid: &mut u32,
    ) -> Result<Ext4Fsblk, isize> {
        let mut alloc: Ext4Fsblk = 0;
        let super_block = &self.superblock;
        let blocks_per_group = super_block.blocks_per_group();

        let mut bgid = *start_bgid;
        let mut idx_in_bg = 0;

        let block_group_count = super_block.block_group_count();
        let mut count = block_group_count;

        while count > 0 {
            // Load block group reference
            let mut block_group =
                Ext4BlockGroup::load_new(self.block_device.clone(), super_block, bgid as usize);

            let free_blocks = block_group.get_free_blocks_count();
            if free_blocks == 0 {
                // Try next block group
                bgid = (bgid + 1) % block_group_count;
                count -= 1;

                if count == 0 {
                    println!("No free blocks available in all block groups");
                    return Err(Errno::ENOSPC as isize);
                }
                continue;
            }

            // Compute indexes
            let first_in_bg = self.get_block_of_bgid(bgid);
            let first_in_bg_index = self.addr_to_idx_bg(first_in_bg);

            if idx_in_bg < first_in_bg_index {
                idx_in_bg = first_in_bg_index;
            }

            // Load block with bitmap
            let bmp_blk_adr = block_group.get_block_bitmap_block(super_block);
            let mut bitmap_block =
                Block::load_offset(self.block_device.clone(), bmp_blk_adr as usize * BLOCK_SIZE);

            // Check if goal is free
            if ext4_bmap_is_bit_clr(&bitmap_block.data, idx_in_bg) {
                ext4_bmap_bit_set(&mut bitmap_block.data, idx_in_bg);
                block_group.set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                // 此处不需要考虑对齐
                self.block_device
                    .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                alloc = self.bg_idx_to_addr(idx_in_bg, bgid);

                /* Update free block counts */
                self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;

                *start_bgid = bgid;
                return Ok(alloc);
            }

            // Try to find free block near to goal
            let blk_in_bg = blocks_per_group;
            let end_idx = min((idx_in_bg + 63) & !63, blk_in_bg);

            for tmp_idx in (idx_in_bg + 1)..end_idx {
                if ext4_bmap_is_bit_clr(&bitmap_block.data, tmp_idx) {
                    ext4_bmap_bit_set(&mut bitmap_block.data, tmp_idx);
                    block_group
                        .set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                    // 此处不需要考虑对齐
                    self.block_device
                        .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                    alloc = self.bg_idx_to_addr(tmp_idx, bgid);
                    self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;

                    *start_bgid = bgid;
                    return Ok(alloc);
                }
            }

            // Find free bit in bitmap
            let mut rel_blk_idx = 0;
            if ext4_bmap_bit_find_clr(&bitmap_block.data, idx_in_bg, blk_in_bg, &mut rel_blk_idx) {
                ext4_bmap_bit_set(&mut bitmap_block.data, rel_blk_idx);
                block_group.set_block_group_balloc_bitmap_csum(super_block, &bitmap_block.data);
                // 此处不需要考虑对齐
                self.block_device
                    .write_block(bmp_blk_adr as usize, &bitmap_block.data);
                alloc = self.bg_idx_to_addr(rel_blk_idx, bgid);
                self.update_free_block_counts(inode_ref, &mut block_group, bgid as usize)?;

                *start_bgid = bgid;
                return Ok(alloc);
            }

            // No free block found in this group, try other block groups
            bgid = (bgid + 1) % block_group_count;
            count -= 1;
        }
        println!("No free blocks available in all block groups");
        return Err(Errno::ENOSPC as isize);
    }

    fn update_free_block_counts(
        &self,
        inode_ref: &mut Ext4InodeRef,
        block_group: &mut Ext4BlockGroup,
        bgid: usize,
    ) -> Result<(), isize> {
        let mut super_block = self.superblock;
        let block_size = BLOCK_SIZE as u64;

        // 更新超级块的空闲块数
        let mut super_blk_free_blocks = super_block.free_blocks_count();
        super_blk_free_blocks -= 1;
        super_block.set_free_blocks_count(super_blk_free_blocks);
        super_block.sync_to_disk_with_csum(self.block_device.clone());

        // Update inode blocks (different block size!) count
        let mut inode_blocks = inode_ref.inode.blocks_count();
        inode_blocks += block_size / EXT4_INODE_BLOCK_SIZE as u64;
        inode_ref.inode.set_blocks_count(inode_blocks);
        self.write_back_inode(inode_ref);

        // Update block group free blocks count
        let mut fb_cnt = block_group.get_free_blocks_count();
        fb_cnt -= 1;
        block_group.set_free_blocks_count(fb_cnt as u32);
        block_group.sync_to_disk_with_csum(self.block_device.clone(), bgid, &super_block);

        Ok(())
    }

    #[allow(unused)]
    pub fn balloc_free_blocks(&self, inode_ref: &mut Ext4InodeRef, start: Ext4Fsblk, count: u32) {
        // log::trace!("balloc_free_blocks start {:x?} count {:x?}", start, count);
        let mut count = count as usize;
        let mut start = start;

        let mut super_block = self.superblock;

        let blocks_per_group = super_block.blocks_per_group();

        let bgid = start / blocks_per_group as u64;

        let mut bg_first = start / blocks_per_group as u64;
        let mut bg_last = (start + count as u64 - 1) / blocks_per_group as u64;

        while bg_first <= bg_last {
            let idx_in_bg = start % blocks_per_group as u64;

            let mut bg =
                Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

            let block_bitmap_block = bg.get_block_bitmap_block(&super_block);
            let mut raw_data = [0u8; BLOCK_SIZE];
            self.block_device
                .read_block(block_bitmap_block as usize, &mut raw_data);
            let mut data: &mut Vec<u8> = &mut raw_data.to_vec();

            let mut free_cnt = BLOCK_SIZE * 8 - idx_in_bg as usize;

            if count > free_cnt {
            } else {
                free_cnt = count;
            }

            ext4_bmap_bits_free(data, idx_in_bg as u32, free_cnt as u32);

            count -= free_cnt;
            start += free_cnt as u64;

            bg.set_block_group_balloc_bitmap_csum(&super_block, data);
            // 此处不需要考虑对齐
            self.block_device
                .write_block(block_bitmap_block as usize, data);

            /* Update superblock free blocks count */
            let mut super_blk_free_blocks = super_block.free_blocks_count();

            super_blk_free_blocks += free_cnt as u64;
            super_block.set_free_blocks_count(super_blk_free_blocks);
            super_block.sync_to_disk_with_csum(self.block_device.clone());

            /* Update inode blocks (different block size!) count */
            let mut inode_blocks = inode_ref.inode.blocks_count();

            inode_blocks -= (free_cnt * (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE)) as u64;
            inode_ref.inode.set_blocks_count(inode_blocks);
            self.write_back_inode(inode_ref);

            /* Update block group free blocks count */
            let mut fb_cnt = bg.get_free_blocks_count();
            fb_cnt += free_cnt as u64;
            bg.set_free_blocks_count(fb_cnt as u32);
            bg.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

            bg_first += 1;
        }
    }
}
