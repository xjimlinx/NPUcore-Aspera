#![allow(unused)]
use core::arch::asm;
use core::ptr::addr_of;

use crate::arch;

// use super::{layout::BPB, Cache};
use super::Cache;
use super::{BlockCacheManager, BlockDevice, Ext4};
use alloc::{sync::Arc, vec::Vec};

pub struct EasyFileSystem {
    /// Partition/Device the FAT32 is hosted on.
    pub block_device: Arc<dyn BlockDevice>,
    /// EXT4 information
    pub ext4: Ext4,
    /// The first data sector beyond the root directory
    pub data_area_start_block: u32,
    /// This is set to the cluster number of the first cluster of the root directory,
    /// usually 2 but not required to be 2.
    pub root_clus: u32,
    /// sector per cluster, usually 8 for SD card
    pub sec_per_clus: u8,
    /// Bytes per sector, 512 for SD card
    pub byts_per_sec: u16,
}

// export implementation of methods from Ext4.
impl EasyFileSystem {
    // TODO:
}

impl EasyFileSystem {
    pub fn first_data_sector(&self) -> u32 {
        self.data_area_start_block
    }
}

impl EasyFileSystem {
    /// Open the filesystem object.
    /// # Arguments
    /// + `block_device`: pointer of hardware device
    /// + `index_cache_mgr`: fat cache manager
    pub fn open(
        block_device: Arc<dyn BlockDevice>,
        index_cache_mgr: Arc<spin::Mutex<BlockCacheManager>>,
    ) -> Arc<Self> {
        todo!()
    }
}
