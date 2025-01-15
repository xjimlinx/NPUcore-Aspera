use crate::fs::ext4::{block_group::Ext4BlockGroup, BLOCK_SIZE};

use super::{
    bitmap::{ext4_bmap_bit_clr, ext4_bmap_bit_find_clr, ext4_bmap_bit_set},
    error::Errno,
    ext4fs::Ext4FileSystem,
};

impl Ext4FileSystem {
    /// 分配inode号
    /// # 参数
    /// + is_dir: 是否是文件夹
    /// # 返回值
    /// + 新的inode号
    pub fn ialloc_alloc_inode(&self, is_dir: bool) -> Result<u32, isize> {
        let mut bgid = 0;
        let bg_count = self.superblock.block_group_count();
        let mut super_block = self.superblock;

        while bgid <= bg_count {
            if bgid == bg_count {
                bgid = 0;
                continue;
            }

            // 获取块组
            let mut bg =
                Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

            let mut free_inodes = bg.get_free_inodes_count();

            if free_inodes > 0 {
                let inode_bitmap_block = bg.get_inode_bitmap_block(&super_block);

                let mut raw_data = [0u8; BLOCK_SIZE];
                self.block_device
                    .read_block(inode_bitmap_block as usize, &mut raw_data);

                let inodes_in_bg = super_block.get_inodes_in_group_cnt(bgid);

                let bitmap_data = &mut raw_data[..];

                let mut idx_in_bg = 0;

                ext4_bmap_bit_find_clr(bitmap_data, 0, inodes_in_bg, &mut idx_in_bg);
                ext4_bmap_bit_set(bitmap_data, idx_in_bg);

                // update bitmap in disk
                // 此处因为是直接进行块单位的写入，所以不需要考虑对齐
                self.block_device
                    .write_block(inode_bitmap_block as usize, bitmap_data);

                bg.set_block_group_ialloc_bitmap_csum(&super_block, bitmap_data);

                // 修改文件系统计数器
                free_inodes -= 1;
                bg.set_free_inodes_count(&super_block, free_inodes);

                /* Increment used directories counter */
                if is_dir {
                    let used_dirs = bg.get_used_dirs_count(&super_block) + 1;
                    bg.set_used_dirs_count(&super_block, used_dirs);
                }

                // 减少未使用inode计数
                let mut unused = bg.get_itable_unused(&super_block);
                let free = inodes_in_bg - unused;
                if idx_in_bg >= free {
                    unused = inodes_in_bg - (idx_in_bg + 1);
                    bg.set_itable_unused(&super_block, unused);
                }

                // 同步块组内容
                bg.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

                // 更新超级块
                super_block.decrease_free_inodes_count();
                super_block.sync_to_disk_with_csum(self.block_device.clone());
                // 看是否写入成功
                let mut test_super_block = [0u8; BLOCK_SIZE];
                self.block_device.read_block(0, &mut test_super_block);

                /* Compute the absolute i-nodex number */
                // 计算inode号
                let inodes_per_group = super_block.inodes_per_group();
                let inode_num = bgid * inodes_per_group + (idx_in_bg + 1);

                return Ok(inode_num);
            }

            bgid += 1;
        }

        println!("[kernel ialloc] alloc inode failed");
        return Err(Errno::ENOSPC as isize);
    }

    pub fn ialloc_free_inode(&self, index: u32, is_dir: bool) {
        // Compute index of block group
        let bgid = self.get_bgid_of_inode(index);
        let block_device = self.block_device.clone();

        let mut super_block = self.superblock;
        let mut bg =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

        // Load inode bitmap block
        let inode_bitmap_block = bg.get_inode_bitmap_block(&self.superblock);
        let mut bitmap_data = [0u8; BLOCK_SIZE];
        self.block_device
            .read_block(inode_bitmap_block as usize, &mut bitmap_data);

        // Find index within group and clear bit
        let index_in_group = self.inode_to_bgidx(index);
        ext4_bmap_bit_clr(&mut bitmap_data, index_in_group);

        // Set new checksum after modification
        // update bitmap in disk
        // 此处与上面的ialloc_alloc_inode函数一样，不需要考虑对齐
        self.block_device
            .write_block(inode_bitmap_block as usize, &bitmap_data);
        bg.set_block_group_ialloc_bitmap_csum(&super_block, &bitmap_data);

        // Update free inodes count in block group
        let free_inodes = bg.get_free_inodes_count() + 1;
        bg.set_free_inodes_count(&self.superblock, free_inodes);

        // If inode was a directory, decrement the used directories count
        if is_dir {
            let used_dirs = bg.get_used_dirs_count(&self.superblock) - 1;
            bg.set_used_dirs_count(&self.superblock, used_dirs);
        }

        bg.sync_to_disk_with_csum(block_device.clone(), bgid as usize, &super_block);

        super_block.decrease_free_inodes_count();
        super_block.sync_to_disk_with_csum(self.block_device.clone());
    }
}
