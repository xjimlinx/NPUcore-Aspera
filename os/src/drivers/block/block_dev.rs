use core::any::Any;

use crate::arch::BLOCK_SZ;
/// We should regulate the behavior of this trait on FAILURE
/// e.g. What if buf.len()>BLOCK_SZ for read_block?
/// e.g. Does read_block clean the rest part of the block to be zero for buf.len()!=BLOCK_SZ in write_block() & read_block()
/// e.g. What if buf.len()<BLOCK_SZ for write_block?
pub trait BlockDevice: Send + Sync + Any {
    /// 从块设备读取块
    /// # 参数
    /// * `block_id`: the first sector(block) number to be read
    /// * `block_id`: 要读取的第一个块号
    /// * `buf`: 存储读取的数据
    /// # Panic
    /// 当buf大小不是BLOCK_SZ的整数倍时会崩溃
    fn read_block(&self, block_id: usize, buf: &mut [u8]);

    /// 将块写入块设备
    /// # 参数
    /// * `block_id`: 要写入的第一个块号
    /// * `buf`: 存储要写入的数据
    /// # Panic
    /// 当buf大小不是BLOCK_SZ的整数倍时会崩溃
    fn write_block(&self, block_id: usize, buf: &[u8]);

    /// # Note
    /// *We should rewrite the API for K210 since it supports NATIVE multi-block clearing*
    fn clear_block(&self, block_id: usize, num: u8) {
        self.write_block(block_id, &[num; BLOCK_SZ]);
    }

    /// # Note
    /// *We should rewrite the API for K210 if it supports NATIVE multi-block clearing*
    fn clear_mult_block(&self, block_id: usize, cnt: usize, num: u8) {
        for i in block_id..block_id + cnt {
            self.write_block(i, &[num; BLOCK_SZ]);
        }
    }
}
