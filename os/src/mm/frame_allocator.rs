#[cfg(feature = "oom_handler")]
use super::super::fs;
use super::{PhysAddr, PhysPageNum};
use crate::hal::MEMORY_END;
#[cfg(feature = "oom_handler")]
use crate::task::current_task;

use alloc::{sync::Arc, vec::Vec};
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;
use spin::RwLock;

/// 物理帧跟踪器
pub struct FrameTracker {
    /// 跟踪的物理页号
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // 清理页面内容
        let dwords_array = ppn.get_dwords_array();
        for i in dwords_array {
            *i = 0;
        }
        Self { ppn }
    }
    pub unsafe fn new_uninit(ppn: PhysPageNum) -> Self {
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}
impl Drop for FrameTracker {
    // 自动回收物理帧
    fn drop(&mut self) {
        // println!("do drop at {}", self.ppn.0);
        frame_dealloc(self.ppn);
    }
}

/// 帧分配器接口
trait FrameAllocator {
    fn new() -> Self;
    /// 分配
    fn alloc(&mut self) -> Option<FrameTracker>;
    unsafe fn alloc_uninit(&mut self) -> Option<FrameTracker>;
    /// 释放
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 栈式帧分配器
pub struct StackFrameAllocator {
    // 当前分配器的位置，指向可分配区域的开始
    current: usize,
    // 分配器的结束地址，表示可分配内存区域的末尾
    end: usize,
    // 已回收的页面（内存框架）的列表
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 初始化方法
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        let last_frames = self.end - self.current;
        self.recycled.reserve(last_frames);
        println!("last {} Physical Frames.", last_frames);
    }
    /// 计算未分配的大小
    pub fn unallocated_frames(&self) -> usize {
        self.end - self.current + self.recycled.len()
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 分配一个物理页
    fn alloc(&mut self) -> Option<FrameTracker> {
        // 优先使用回收的帧
        if let Some(ppn) = self.recycled.pop() {
            let frame_tracker = FrameTracker::new(ppn.into());
            log::trace!("[frame_alloc] {:?}", frame_tracker);
            Some(frame_tracker)
        } else if self.current == self.end {
            // 无可用帧
            None
        } else {
            // 否则分配当前页
            self.current += 1;
            #[cfg(not(feature = "zero_init"))]
            let frame_tracker = FrameTracker::new((self.current - 1).into());
            #[cfg(feature = "zero_init")]
            let frame_tracker = unsafe { FrameTracker::new_uninit((self.current - 1).into()) };
            log::trace!("[frame_alloc] {:?}", frame_tracker);
            Some(frame_tracker)
        }
    }
    unsafe fn alloc_uninit(&mut self) -> Option<FrameTracker> {
        if let Some(ppn) = self.recycled.pop() {
            let frame_tracker = FrameTracker::new_uninit(ppn.into());
            //log::trace!("[frame_alloc_uninit] {:?}", frame_tracker);
            Some(frame_tracker)
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            let frame_tracker = FrameTracker::new_uninit((self.current - 1).into());
            log::trace!("[frame_alloc_uninit] {:?}", frame_tracker);
            Some(frame_tracker)
        }
    }
    /// 释放一个物理页
    fn dealloc(&mut self, ppn: PhysPageNum) {
        log::trace!("[frame_dealloc] {:?}", ppn);
        let ppn = ppn.0;
        // 验证帧的有效性（DEBUG模式下），RELEASE中这个检查不必要，并且这个检查可能会显著降低回收速度
        if option_env!("MODE") == Some("debug") && ppn >= self.current
            || self.recycled.iter().find(|&v| *v == ppn).is_some()
        {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 全局帧分配器
    pub static ref FRAME_ALLOCATOR: RwLock<FrameAllocatorImpl> =
        RwLock::new(FrameAllocatorImpl::new());
}
/// 初始化全局帧分配器
pub fn init_frame_allocator() {
    extern "C" {
        // 内核结束地址？
        fn ekernel();
    }
    FRAME_ALLOCATOR.write().init(
        // 从内核结束地址ekernel
        PhysAddr::from(ekernel as usize).ceil(),
        // 到内存结束地址
        PhysAddr::from(MEMORY_END).floor(),
        // 作为可用物理内存
    );
}

/// 尝试使用所有可能的方法来释放制定数量为`req`的页
/// 成功返回Ok(())，失败返回Err(())
#[cfg(feature = "oom_handler")]
pub fn oom_handler(req: usize) -> Result<(), ()> {
    // step 1: 清理文件系统缓存
    let mut released = 0;
    released += fs::directory_tree::oom();
    if released >= req {
        return Ok(());
    }
    // step 2: 清理当前任务的内存
    let task = current_task().unwrap();
    if let Some(mut memory_set) = task.vm.try_lock() {
        released += memory_set.do_shallow_clean();
        log::warn!("[oom_handler] current task released: {}", released);
    } else {
        log::warn!("[oom_handler] try lock current task vm failed!");
    }
    if released >= req {
        return Ok(());
    }
    // step 3: 清理所有任务的内存
    log::warn!("[oom_handler] notify all tasks!");
    crate::task::do_oom(req - released)
}

#[cfg(feature = "oom_handler")]
/// 帧预留机制
/// # 参数
/// + num: 指定要保留的帧数量
pub fn frame_reserve(num: usize) {
    // 获取还可分配的帧数量
    let remain = FRAME_ALLOCATOR.read().unallocated_frames();
    if remain < num {
        oom_handler(num - remain).unwrap()
    }
}

#[cfg(not(feature = "oom_handler"))]
pub fn frame_reserve(_num: usize) {
    // do nothing
}

#[cfg(feature = "oom_handler")]
/// 带OOM的分配操作
pub fn frame_alloc() -> Option<Arc<FrameTracker>> {
    let result = FRAME_ALLOCATOR.write().alloc();
    match result {
        Some(frame_tracker) => Some(Arc::new(frame_tracker)),
        None => {
            crate::show_frame_consumption! {
                "GC";
                oom_handler(1).unwrap();
            };
            FRAME_ALLOCATOR
                .write()
                .alloc()
                .map(|frame_tracker| Arc::new(frame_tracker))
        }
    }
}

#[cfg(not(feature = "oom_handler"))]
/// 常规分配操作
pub fn frame_alloc() -> Option<Arc<FrameTracker>> {
    FRAME_ALLOCATOR
        .write()
        .alloc()
        .map(|frame_tracker| Arc::new(frame_tracker))
}

#[cfg(feature = "oom_handler")]
pub unsafe fn frame_alloc_uninit() -> Option<Arc<FrameTracker>> {
    let result = FRAME_ALLOCATOR.write().alloc_uninit();
    match result {
        Some(frame_tracker) => Some(Arc::new(frame_tracker)),
        None => {
            crate::show_frame_consumption! {
                "GC";
                oom_handler(1).unwrap();
            };
            FRAME_ALLOCATOR
                .write()
                .alloc_uninit()
                .map(|frame_tracker| Arc::new(frame_tracker))
        }
    }
}

#[cfg(not(feature = "oom_handler"))]
pub unsafe fn frame_alloc_uninit() -> Option<Arc<FrameTracker>> {
    FRAME_ALLOCATOR
        .write()
        .alloc_uninit()
        .map(|frame_tracker| Arc::new(frame_tracker))
}

/// 释放帧
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.write().dealloc(ppn);
}

/// 计算可用帧数量
pub fn unallocated_frames() -> usize {
    FRAME_ALLOCATOR.write().unallocated_frames()
}

#[macro_export]
/// * `$place`: the name tag for the promotion.
/// * `statement`: the enclosed
/// * `before`:
/// 用于测量代码块的帧消耗情况
macro_rules! show_frame_consumption {
    ($place:literal; $($statement:stmt); *;) => {
        let __frame_consumption_before = crate::mm::unallocated_frames();
        $($statement)*
        let __frame_consumption_after = crate::mm::unallocated_frames();
        log::debug!("[{}] consumed frames: {}, last frames: {}", $place, (__frame_consumption_before - __frame_consumption_after) as isize, __frame_consumption_after)
    };
    ($place:literal, $before:ident) => {
        log::debug!(
            "[{}] consumed frames:{}, last frames:{}",
            $place,
            ($before - crate::mm::unallocated_frames()) as isize,
            crate::mm::unallocated_frames()
        );
    };
}
