pub use crate::hal::{trap_cx_bottom_from_tid, ustack_bottom_from_tid};
use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;

/// 用于分配pid的结构体
pub struct RecycleAllocator {
    /// 当前分配的id
    current: usize,
    /// 存储已经回收的id，供后续分配使用
    recycled: Vec<usize>,
}

impl RecycleAllocator {
    /// 构造函数
    pub fn new() -> Self {
        RecycleAllocator {
            // 当前分配的id数量初始化为0
            current: 0,
            // 初始化为空向量
            recycled: Vec::new(),
        }
    }
    /// 分配一个新的id
    pub fn alloc(&mut self) -> usize {
        // 从回收的id中取出一个，如果没有则分配一个新的
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            // 当前分配的id数量加1
            self.current += 1;
            // 返回分配的id号
            self.current - 1
        }
    }
    /// 回收一个id
    pub fn dealloc(&mut self, id: usize) {
        // 检查id是否合法
        assert!(id < self.current);
        // 检查id是否已经被回收
        assert!(
            !self.recycled.iter().any(|i| *i == id),
            "id {} has been deallocated!",
            id
        );
        // 将id回收，放入回收向量中
        self.recycled.push(id);
    }
    /// 获取已经分配的id数量
    pub fn get_allocated(&self) -> usize {
        // 返回当前分配的id数量减去已经回收的id数量
        self.current - self.recycled.len()
    }
}

lazy_static! {
    /// 全局的PID分配器对象，使用Mutex进行包装保证线程安全
    static ref PID_ALLOCATOR: Mutex<RecycleAllocator> = Mutex::new(RecycleAllocator::new());
}

/// 表示一个pid的句柄
/// 包装有一个pid号
pub struct PidHandle(pub usize);

/// 分配一个pid
pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.lock().alloc())
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}
