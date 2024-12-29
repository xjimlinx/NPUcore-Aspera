use super::layout::BAD_BLOCK;
use super::{BlockCacheManager, BlockDevice, Cache};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

const VACANT_CLUS_CACHE_SIZE: usize = 64;
const FAT_ENTRY_FREE: u32 = 0;
const FAT_ENTRY_RESERVED_TO_END: u32 = 0x0FFF_FFF8;
/// fat中簇的结束位置
pub const EOC: u32 = 0x0FFF_FFFF;
/// *In-memory* data structure
/// 内存内的fat数据结构.
/// 在Fat32文件系统中，有两个fat表，这里只使用第一张fat表
/// 也就是说还没有实现fat的检错功能
pub struct Fat {
    /// Cache manager for fat
    fat_cache_mgr: Arc<Mutex<BlockCacheManager>>,
    /// The first block id of FAT.
    /// In FAT32, this is equal to bpb.rsvd_sec_cnt
    start_block_id: usize,
    /// size fo sector in bytes copied from BPB
    byts_per_sec: usize,
    /// The total number of FAT entries
    tot_ent: usize,
    /// The queue used to store known vacant clusters
    vacant_clus: Mutex<VecDeque<u32>>,
    /// The final unused cluster id we found
    hint: Mutex<usize>,
}

impl Fat {
    /// 获取当前fat表项指向的的下一个簇号
    /// # 参数
    /// + `current_clus_num`: 当前簇号
    /// + `block_device`: 指向块设备的指针
    /// # 返回值
    /// 下一个簇的簇号
    pub fn get_next_clus_num(
        &self,
        current_clus_num: u32,
        block_device: &Arc<dyn BlockDevice>,
    ) -> u32 {
        self.fat_cache_mgr
            .lock()
            .get_block_cache(self.this_fat_sec_num(current_clus_num), block_device)
            .lock()
            .read(
                self.this_fat_ent_offset(current_clus_num),
                |fat_entry: &u32| -> u32 { *fat_entry },
            )
            & EOC
    }
    /// Get all cluster numbers after the current cluster number
    /// # Arguments
    /// + `current_clus_num`: current cluster number
    /// + `block_device`: pointer of block device
    /// # Return value
    /// List of cluster numbers
    pub fn get_all_clus_num(
        &self,
        mut current_clus_num: u32,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Vec<u32> {
        let mut v = Vec::with_capacity(8);
        loop {
            v.push(current_clus_num);
            current_clus_num = self.get_next_clus_num(current_clus_num, &block_device);
            if [BAD_BLOCK, FAT_ENTRY_FREE].contains(&current_clus_num)
                || current_clus_num >= FAT_ENTRY_RESERVED_TO_END
            {
                break;
            }
        }
        v
    }

    /// Constructor for fat
    /// # Argument
    /// + `rsvd_sec_cnt`: size in bytes of BPB
    /// + `byts_per_sec`: bytes per sector
    /// + `clus`: the total number of FAT entries
    /// + `fat_cache_mgr`: fat cache manager
    /// # Return value
    /// Fat
    pub fn new(
        rsvd_sec_cnt: usize,
        byts_per_sec: usize,
        clus: usize,
        fat_cache_mgr: Arc<Mutex<BlockCacheManager>>,
    ) -> Self {
        Self {
            //used_marker: Default::default(),
            fat_cache_mgr,
            start_block_id: rsvd_sec_cnt,
            byts_per_sec,
            tot_ent: clus,
            vacant_clus: Mutex::new(VecDeque::new()),
            hint: Mutex::new(0),
        }
    }

    /// For a given cluster number, calculate its sector ID in the fat region
    /// # Argument
    /// + `clus_num`: cluster number
    /// # Return value
    /// sector ID
    #[inline(always)]
    pub fn this_fat_sec_num(&self, clus_num: u32) -> usize {
        let fat_offset = clus_num * 4;
        (self.start_block_id as u32 + (fat_offset / (self.byts_per_sec as u32))) as usize
    }
    #[inline(always)]
    /// 对于给定的簇号，计算它在fat分区的扇区中的偏移量
    /// # 参数
    /// + `clus_num`: 簇号
    /// # 返回值
    /// 偏移量
    pub fn this_fat_ent_offset(&self, clus_num: u32) -> usize {
        let fat_offset = clus_num * 4;
        (fat_offset % (self.byts_per_sec as u32)) as usize
    }
    /// 将簇项从当前指向下一个
    /// 如果 current 是空值，忽略该操作
    /// # 参数
    /// + `block_device`: 块设备对象
    /// + `current`: 当前簇号
    /// + `next`: 要设置的下一个簇
    fn set_next_clus(&self, block_device: &Arc<dyn BlockDevice>, current: Option<u32>, next: u32) {
        if current.is_none() {
            return;
        }
        let current = current.unwrap();
        self.fat_cache_mgr
            .lock()
            .get_block_cache(self.this_fat_sec_num(current), block_device)
            .lock()
            .modify(
                self.this_fat_ent_offset(current),
                |bitmap_block: &mut u32| {
                    //println!("[set_next_clus]bitmap_block:{}->{}", *bitmap_block, next);
                    *bitmap_block = next;
                },
            )
    }

    /// 尽可能多的分配簇，但是不会比alloc_num大
    /// # 参数
    /// + `block_device`: 目标块设备
    /// + `alloc_num`: 要分配的簇的数量
    /// + `last`: 待分配簇的前一个簇
    /// # 返回值
    /// 簇号列表
    pub fn alloc(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        alloc_num: usize,
        mut last: Option<u32>,
    ) -> Vec<u32> {
        // 先在内存中创建一个空的簇号列表
        let mut allocated_cluster = Vec::with_capacity(alloc_num);
        // 需要一个锁来保证进程间的互斥
        let mut hlock = self.hint.lock();
        // 从0到alloc_num，循环直到分配完alloc_num个簇
        for _ in 0..alloc_num {
            // 获取簇
            last = self.alloc_one(block_device, last, &mut hlock);
            if last.is_none() {
                // 已经没有空闲簇了，或者last的下一个簇是有效的
                log::error!("[alloc]: alloc error, last: {:?}", last);
                break;
            }
            // 将分配的簇号加入到allocated_cluster中
            allocated_cluster.push(last.unwrap());
        }
        // 设置最后一个簇的下一个簇为EOC
        self.set_next_clus(block_device, last, EOC);
        allocated_cluster
    }

    /// 从数据区寻找并分配一个簇
    /// # 参数
    /// + `block_device`: 目标块设备
    /// + `last`: 要分配的簇号的前一个簇
    /// + `hlock`: The lock of hint(Fat).
    /// # 返回值
    /// 如果成功分配，返回分配的簇号
    /// 否则返回空
    fn alloc_one(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        last: Option<u32>,
        hlock: &mut MutexGuard<usize>,
    ) -> Option<u32> {
        if last.is_some() {
            let next_cluster_of_current = self.get_next_clus_num(last.unwrap(), block_device);
            debug_assert!(next_cluster_of_current >= FAT_ENTRY_RESERVED_TO_END);
        }
        // 现在我们可以自由的分配簇了

        // 从 vacant_clus 中获取一个空闲簇
        if let Some(free_clus_id) = self.vacant_clus.lock().pop_back() {
            self.set_next_clus(block_device, last, free_clus_id);
            return Some(free_clus_id);
        }

        // Allocate a free cluster starts with `hint`
        let start = **hlock;
        let free_clus_id = self.get_next_free_clus(start as u32, block_device);
        if free_clus_id.is_none() {
            return None;
        }
        let free_clus_id = free_clus_id.unwrap();
        **hlock = (free_clus_id + 1) as usize % self.tot_ent;

        self.set_next_clus(block_device, last, free_clus_id);
        Some(free_clus_id)
    }

    /// Find next free cluster from data area.
    /// # Argument
    /// + `start`: The cluster id to traverse to find the next free cluster
    /// + `block_device`: The target block_device.
    /// # Return value
    /// If successful, return free cluster number
    /// otherwise, return None
    fn get_next_free_clus(&self, start: u32, block_device: &Arc<dyn BlockDevice>) -> Option<u32> {
        for clus_id in start..self.tot_ent as u32 {
            if FAT_ENTRY_FREE == self.get_next_clus_num(clus_id, block_device) {
                return Some(clus_id);
            }
        }
        for clus_id in 0..start {
            if FAT_ENTRY_FREE == self.get_next_clus_num(clus_id, block_device) {
                return Some(clus_id);
            }
        }
        None
    }

    /// Free multiple clusters from the data area.
    /// # Argument
    /// + `block_device`: Pointer to block_device.
    /// + `cluster_list`: List of clusters that need to be freed
    pub fn free(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        cluster_list: Vec<u32>,
        last: Option<u32>,
    ) {
        // Before freeing, a lock
        let mut lock = self.vacant_clus.lock();
        for cluster_id in cluster_list {
            self.set_next_clus(block_device, Some(cluster_id), FAT_ENTRY_FREE);
            if lock.len() < VACANT_CLUS_CACHE_SIZE {
                lock.push_back(cluster_id);
            }
        }
        self.set_next_clus(block_device, last, EOC);
    }
}
