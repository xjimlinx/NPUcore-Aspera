use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use spin::Mutex;

#[derive(Debug)]
/// Zram错误枚举
pub enum ZramError {
    /// 无效索引
    InvalidIndex,
    /// 空间不足
    NoSpace,
    /// 未分配
    NotAllocated,
}

#[derive(Debug)]
/// zram跟踪器
pub struct ZramTracker(pub usize);

impl Drop for ZramTracker {
    /// 自动释放Zram资源
    fn drop(&mut self) {
        ZRAM_DEVICE.lock().discard(self.0).unwrap();
    }
}

/// Zram结构
pub struct Zram {
    /// 压缩数据存储
    compressed: Vec<Option<Vec<u8>>>,
    /// 回收的索引
    recycled: Vec<u16>,
    /// 当前分配的位置
    tail: u16,
}

impl Zram {
    /// 构造方法
    pub fn new(capacity: usize) -> Self {
        // 预分配制定容量的向量
        let mut compressed = Vec::with_capacity(capacity);
        // 初始化为全None状态
        compressed.resize(compressed.capacity(), None);
        Self {
            compressed,
            // 回收列表为空
            recycled: Vec::new(),
            // tail 从0开始
            tail: 0,
        }
    }
    /// 数据插入
    fn insert(&mut self, data: Vec<u8>) -> Result<Arc<ZramTracker>, ZramError> {
        // 优先使用回收的索引
        let zram_id = match self.recycled.pop() {
            Some(zram_id) => zram_id as usize,
            None => {
                if self.tail as usize == self.compressed.len() {
                    // 空间不足，返回错误
                    return Err(ZramError::NoSpace);
                } else {
                    // 更新tail
                    self.tail += 1;
                    (self.tail - 1) as usize
                }
            }
        };
        // 存储数据
        self.compressed[zram_id] = Some(data);
        // 返回跟踪器
        Ok(Arc::new(ZramTracker(zram_id)))
    }
    /// 获取数据
    fn get(&self, zram_id: usize) -> Result<&Vec<u8>, ZramError> {
        // 如果zram_id大于容器大小
        if zram_id >= self.compressed.len() {
            return Err(ZramError::InvalidIndex);
        }
        match &self.compressed[zram_id] {
            Some(compressed_data) => Ok(compressed_data),
            None => Err(ZramError::NotAllocated),
        }
    }
    /// 移除数据
    fn remove(&mut self, zram_id: usize) -> Result<Vec<u8>, ZramError> {
        // 如果索引大于容器大小
        if zram_id >= self.compressed.len() {
            return Err(ZramError::InvalidIndex);
        }
        // 刚好等于最后一个分配的id
        if zram_id == (self.tail - 1) as usize {
            // 回退tail
            self.tail = zram_id as u16;
        } else {
            // 加入回收列表
            self.recycled.push(zram_id as u16);
        }
        match self.compressed[zram_id].take() {
            Some(compressed_data) => Ok(compressed_data),
            None => Err(ZramError::NotAllocated),
        }
    }
    /// 读接口
    pub fn read(&mut self, zram_id: usize, buf: &mut [u8]) -> Result<(), ZramError> {
        // 获取压缩数据
        match self.get(zram_id) {
            Ok(compressed_data) => {
                // 解压数据
                let decompressed_data =
                    decompress_size_prepended(compressed_data.as_slice()).unwrap();
                // 复制到输出缓冲区
                buf.copy_from_slice(decompressed_data.as_slice());
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    /// 写接口
    pub fn write(&mut self, buf: &[u8]) -> Result<Arc<ZramTracker>, ZramError> {
        // 压缩输入数据
        let mut compressed = compress_prepend_size(buf);
        // 释放多余容量
        compressed.shrink_to_fit();
        log::trace!("[zram] compressed len: {}", compressed.len());
        // 插入数据并返回跟踪器
        self.insert(compressed)
    }
    #[inline(always)]
    /// 释放
    pub fn discard(&mut self, zram_id: usize) -> Result<(), ZramError> {
        match self.remove(zram_id) {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

lazy_static! {
    /// 全局ZRAM设备
    pub static ref ZRAM_DEVICE: Arc<Mutex<Zram>> = Arc::new(Mutex::new(Zram::new(2048)));
}
