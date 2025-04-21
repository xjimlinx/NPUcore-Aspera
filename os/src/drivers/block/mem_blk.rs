use super::BlockDevice;
use crate::{config::DISK_IMAGE_BASE, hal::BLOCK_SZ};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use spin::Mutex;
struct MemBlock(usize);

impl MemBlock {
    const BLOCK_SIZE: usize = BLOCK_SZ;
    /// 获取从指定块开始的指定长度的数据
    pub fn block_ref(&self, block_id: usize, len: usize) -> &[u8] {
        unsafe { from_raw_parts((self.0 + block_id * Self::BLOCK_SIZE) as *const u8, len) }
    }
    /// 获取从指定块开始的指定长度的数据并返回可变引用
    pub fn block_refmut(&self, block_id: usize, len: usize) -> &mut [u8] {
        unsafe { from_raw_parts_mut((self.0 + block_id * Self::BLOCK_SIZE) as *mut u8, len) }
    }
}

pub struct MemBlockWrapper(Mutex<MemBlock>);

#[allow(unused)]
impl MemBlockWrapper {
    const BASE_ADDR: usize = DISK_IMAGE_BASE;
    /// 这个MemBlockWrapper在RiscV下可能有问题
    pub fn new() -> Self {
        Self(Mutex::new(MemBlock(MemBlockWrapper::BASE_ADDR)))
    }
}
use log::info;
impl BlockDevice for MemBlockWrapper {
    /// 从块设备对象读取一个块
    /// # 参数：
    /// + block_id: 块号
    /// + buf: 读取的数据存放的缓冲区
    /// 此处buf长度需要考虑对齐吗？
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        info!("[mem read_block] len : {}", buf.len());
        let blk = self.0.lock();
        buf.copy_from_slice(blk.block_ref(block_id, buf.len()));
    }
    /// 向块设备对象写入一个块
    /// # 参数
    /// + block_id: 块号
    /// + buf: 写入的数据
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        info!("[mem write_block] len : {}", buf.len());
        let blk = self.0.lock();
        blk.block_refmut(block_id, buf.len()).copy_from_slice(buf);
    }
}
