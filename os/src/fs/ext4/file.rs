use alloc::vec::Vec;

pub struct Ext4FileContent {
    /// The size of the file.
    pub size: u32,
    /// The block list.
    block_list: Vec<u32>,
    /// The inode number.
    inode: u32,
}

impl Ext4FileContent {
    pub fn new(size: u32, block_list: Vec<u32>, inode: u32) -> Self {
        Self {
            size,
            block_list,
            inode,
        }
    }

    pub fn get_block_list(&self) -> &Vec<u32> {
        &self.block_list
    }
}