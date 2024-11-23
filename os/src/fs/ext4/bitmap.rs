// use super::layout::BAD_BLOCK;
use super::{BlockCacheManager, BlockDevice, Cache};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

pub const EOC: u32 = 0x0FFF_FFFF;

pub struct Ext4 {
    /// Cache manager for ext4
    ext4_cache_mgr: Arc<Mutex<BlockCacheManager>>,
}

impl Ext4 {
    /// Constructor for ext4
    /// # Argument
    /// # Return value
    /// Ext4
    pub fn new(
        ext4_cache_mgr: Arc<Mutex<BlockCacheManager>>,
    ) -> Self {
        Self {
            ext4_cache_mgr,
        }
    }

    #[inline(always)]
    pub fn alloc(
        &self,
    ) -> Vec<u32> {
        todo!()
    }

    fn alloc_one(
        &self,
    ) -> Option<u32> {
        todo!()
    }

    pub fn free(
        &self,
    ) {
        todo!()
    }
}
