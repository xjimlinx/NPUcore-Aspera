use core::any::Any;

use crate::hal::BLOCK_SZ;
/// We should regulate the behavior of this trait on FAILURE
/// e.g. What if buf.len()>BLOCK_SZ for read_block?
/// e.g. Does read_block clean the rest part of the block to be zero for buf.len()!=BLOCK_SZ in write_block() & read_block()
/// e.g. What if buf.len()<BLOCK_SZ for write_block?
pub trait BlockDevice: Send + Sync + Any {
    /// 从块设备对象读一个块
    /// # 参数
    /// * `block_id`: 要读取的第一个块的块号
    /// * `buf`: 来存储读取数据的buffer
    /// # 崩溃
    /// 当buf大小不为BLOCK_SZ的整数倍的时候，该函数会崩溃
    /// 但是现在更改块设备对象了，应该不会崩溃了？
    /// 可能还得再试试
    fn read_block(&self, block_id: usize, buf: &mut [u8]);

    /// 将块写回块设备对象
    /// # 参数
    /// * `block_id`: 要写入内容的第一个块号
    /// * `buf`: 存储要写入内容的buffer
    /// # 崩溃
    /// 当buf大小不为BLOCK_SZ的整数倍的时候，该函数会崩溃
    fn write_block(&self, block_id: usize, buf: &[u8]);

    /// # 注意
    /// 需要为K210重新编写API,因为其支持原生的multi-block清除
    fn clear_block(&self, block_id: usize, num: u8) {
        self.write_block(block_id, &[num; BLOCK_SZ]);
    }

    /// # 注意
    /// 需要为K210重新编写API,因为其支持原生的multi-block清除
    fn clear_mult_block(&self, block_id: usize, cnt: usize, num: u8) {
        for i in block_id..block_id + cnt {
            self.write_block(i, &[num; BLOCK_SZ]);
        }
    }
}
