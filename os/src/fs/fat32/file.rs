use alloc::vec::Vec;

pub struct Fat32FileContent {
    /// For FAT32, size is a value computed from FAT.
    /// You should iterate around the FAT32 to get the size.
    pub size: u32,
    /// The cluster list.
    clus_list: Vec<u32>,
    /// If this file is a directory, hint will record the position of last directory entry(the first byte is 0x00).
    hint: u32,
}

impl Fat32FileContent {
    pub fn new(size: u32, clus_list: Vec<u32>, hint: u32) -> Self {
        Self {
            size,
            clus_list,
            hint,
        }
    }

    pub fn get_clus_list(&self) -> &Vec<u32> {
        &self.clus_list
    }
}