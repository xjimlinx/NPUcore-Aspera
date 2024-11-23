use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;
use crate::{arch::BLOCK_SZ, config::PAGE_SIZE, drivers::BLOCK_DEVICE};
use super::directory_tree::FILE_SYSTEM;
use lazy_static::*;

lazy_static! {
    // 用于为 SwapTracker 类型实现 Drop 特征
    pub static ref SWAP_DEVICE: Mutex<Swap> = Mutex::new(Swap::new(16));
}

#[derive(Debug)]
pub struct SwapTracker(pub usize);

impl Drop for SwapTracker {
    fn drop(&mut self) {
        // 丢弃 SwapTracker 实例时，将该页标记为已释放
        SWAP_DEVICE.lock().discard(self.0);
    }
}

pub struct Swap {
    bitmap: Vec<u64>,
    block_ids: Vec<usize>,
}
// 每块的大小
// 每页大小为 4KiB，每块大小为 2KiB
// 一个页面有 2 个块
const BLK_PER_PG: usize = PAGE_SIZE / BLOCK_SZ;
// 交换空间的大小 1MiB
// 一个交换空间有 256 个页面
const SWAP_SIZE: usize = 1024 * 1024;
impl Swap {
    /// size: the number of megabytes in swap
    pub fn new(size: usize) -> Self {
        let bit = size * (SWAP_SIZE / PAGE_SIZE); // 1MiB = 4KiB*256
        let vec_len = bit / usize::MAX.count_ones() as usize;
        let mut bitmap = Vec::<u64>::with_capacity(vec_len);
        bitmap.resize(bitmap.capacity(), 0);
        let blocks = size * (SWAP_SIZE / BLOCK_SZ); // 1MiB = 512B * 2048
        Self {
            bitmap,
            // 此处调用了文件系统的 alloc_blocks() 函数
            block_ids: FILE_SYSTEM.alloc_blocks(blocks),
        }
    }

    // 从交换空间读取数据，块级读取
    fn read_page(block_ids: &[usize], buf: &mut [u8]) {
        assert!(block_ids[0] + BLK_PER_PG - 1 == block_ids[BLK_PER_PG - 1]);
        BLOCK_DEVICE.read_block(block_ids[0], buf);
    }

    // 向交换空间写入数据，块级写入
    fn write_page(block_ids: &[usize], buf: &[u8]) {
        assert!(block_ids[0] + (BLK_PER_PG - 1) == block_ids[BLK_PER_PG - 1]);
        BLOCK_DEVICE.write_block(block_ids[0], buf);
    }

    // 位图中某一位置 1 ，表示该页已被使用
    fn set_bit(&mut self, pos: usize) {
        self.bitmap[pos / 64] |= 1 << (pos % 64);
    }

    // 位图中某一位置 0 ，表示该页被释放
    fn clear_bit(&mut self, pos: usize) {
        self.bitmap[pos / 64] &= !(1 << (pos % 64));
    }

    // 尝试为交换空间分配一个页面
    // 如果找到了一个未分配的页面（位图中的位为 0）
    // 它会返回该页面的 swap_id，否则返回 None。
    fn alloc_page(&self) -> Option<usize> {
        for (i, bit) in self.bitmap.iter().enumerate() {
            if !*bit == 0 {
                continue;
            }
            return Some(i * 64 + (!*bit).trailing_zeros() as usize);
        }
        None
    }

    // 根据 swap_id 获取该交换页面对应的块 ID 列表。
    fn get_block_ids(&self, swap_id: usize) -> &[usize] {
        // 获取一个切片
        // 范围是 [swap_id * BLK_PER_PG, swap_id * BLK_PER_PG + BLK_PER_PG]
        &self.block_ids[swap_id * BLK_PER_PG + 0..swap_id * BLK_PER_PG + BLK_PER_PG]
    }

    // 从交换空间读取数据，会调用 read_page() 函数
    pub fn read(&mut self, swap_id: usize, buf: &mut [u8]) {
        Self::read_page(self.get_block_ids(swap_id), buf);
    }
    // 向交换空间写入数据，会调用 write_page() 函数
    pub fn write(&mut self, buf: &[u8]) -> Arc<SwapTracker> {
        let swap_id = self.alloc_page().unwrap();
        Self::write_page(self.get_block_ids(swap_id), buf);
        self.set_bit(swap_id);
        Arc::new(SwapTracker(swap_id))
    }
    #[inline(always)]
    // 调用 clear_bit()，将位图中的某一位清零
    pub fn discard(&mut self, swap_id: usize) {
        self.clear_bit(swap_id);
    }
}
