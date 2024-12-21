use core::any::Any;

/// 此处的BLOCK_SZ为2048
use crate::arch::BLOCK_SZ;
/// 我们需要规范当失败时这个特征的行为
/// 比如，在 read_block 中当 buf.len() 大于 BLOCK_SZ 的情况
/// 比如，在 write_block 和 read_block 中，buf.len() != BLOCK_SZ 的时候，
/// read_block 会将block余下的内容清除为0吗
/// 比如，在 write_block 中当 buf.len() 小于 BLOCK_SZ 的情况
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
    /// *需要为K210重写API,因为其支持原生多块清除*
    fn clear_block(&self, block_id: usize, num: u8) {
        self.write_block(block_id, &[num; BLOCK_SZ]);
    }

    /// # Note
    /// *同上，需要为K210重写API*
    fn clear_mult_block(&self, block_id: usize, cnt: usize, num: u8) {
        for i in block_id..block_id + cnt {
            self.write_block(i, &[num; BLOCK_SZ]);
        }
    }
}
