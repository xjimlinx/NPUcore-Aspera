/*
    此文件用于管理任务的调度
    内容与RISCV版本相同，无需修改
*/
use core::cmp::Ordering;

#[cfg(feature = "oom_handler")]
use crate::config::SYSTEM_TASK_LIMIT;
#[cfg(feature = "oom_handler")]
use alloc::vec::Vec;

use crate::timer::TimeSpec;

use super::{current_task, TaskControlBlock};
use alloc::collections::{BinaryHeap, VecDeque};
use alloc::sync::{Arc, Weak};
use lazy_static::*;
use spin::Mutex;

#[cfg(feature = "oom_handler")]
/// 任务的激活状态跟踪器
pub struct ActiveTracker {
    /// 存储激活状态的位图
    bitmap: Vec<u64>,
}

#[cfg(feature = "oom_handler")]
#[allow(unused)]
impl ActiveTracker {
    /// 默认大小为128
    pub const DEFAULT_SIZE: usize = SYSTEM_TASK_LIMIT;
    /// 构造函数
    pub fn new() -> Self {
        // 计算位图长度，向上取整
        let len = (Self::DEFAULT_SIZE + 63) / 64;
        // 初始化位图
        let mut bitmap = Vec::with_capacity(len);
        // 位图全部置0
        bitmap.resize(len, 0);
        Self { bitmap }
    }
    /// 检查制定pid的任务是否处于激活状态
    pub fn check_active(&self, pid: usize) -> bool {
        (self.bitmap[pid / 64] & (1 << (pid % 64))) != 0
    }
    /// 检查制定pid的任务是否处于非激活状态
    pub fn check_inactive(&self, pid: usize) -> bool {
        (self.bitmap[pid / 64] & (1 << (pid % 64))) == 0
    }
    /// 标记指定pid的任务为激活状态
    pub fn mark_active(&mut self, pid: usize) {
        self.bitmap[pid / 64] |= 1 << (pid % 64)
    }
    /// 标记指定pid的任务为非激活状态
    pub fn mark_inactive(&mut self, pid: usize) {
        self.bitmap[pid / 64] &= !(1 << (pid % 64))
    }
}

#[cfg(feature = "oom_handler")]
/// 任务管理器
pub struct TaskManager {
    /// 一个双端队列，用于存储就绪态任务
    pub ready_queue: VecDeque<Arc<TaskControlBlock>>,
    /// 一个双端队列，用于存储可中断状态任务
    pub interruptible_queue: VecDeque<Arc<TaskControlBlock>>,
    /// 任务激活状态跟踪器，用于跟踪任务的激活状态，并在OOM时释放内存
    pub active_tracker: ActiveTracker,
}

#[cfg(not(feature = "oom_handler"))]
pub struct TaskManager {
    pub ready_queue: VecDeque<Arc<TaskControlBlock>>,
    pub interruptible_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// 简单的FIFO调度器
impl TaskManager {
    #[cfg(feature = "oom_handler")]
    /// 构造函数
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            interruptible_queue: VecDeque::new(),
            active_tracker: ActiveTracker::new(),
        }
    }
    #[cfg(not(feature = "oom_handler"))]
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            interruptible_queue: VecDeque::new(),
        }
    }
    /// 添加一个任务到就绪队列
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// 从就绪队列中取出一个任务
    #[cfg(feature = "oom_handler")]
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        match self.ready_queue.pop_front() {
            Some(task) => {
                // 标记任务为激活状态
                self.active_tracker.mark_active(task.pid.0);
                Some(task)
            }
            None => None,
        }
    }
    #[cfg(not(feature = "oom_handler"))]
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// 添加一个任务到可中断队列
    pub fn add_interruptible(&mut self, task: Arc<TaskControlBlock>) {
        self.interruptible_queue.push_back(task);
    }
    /// 从可中断队列中删除一个任务
    pub fn drop_interruptible(&mut self, task: &Arc<TaskControlBlock>) {
        self.interruptible_queue
            // 使用retain过滤掉与指定任务相同的任务
            .retain(|task_in_queue| Arc::as_ptr(task_in_queue) != Arc::as_ptr(task));
    }
    /// 根据pid查找任务
    pub fn find_by_pid(&self, pid: usize) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue
            .iter()
            .chain(self.interruptible_queue.iter())
            .find(|task| task.pid.0 == pid)
            .cloned()
    }
    /// 根据tgid(线程组id)查找任务
    pub fn find_by_tgid(&self, tgid: usize) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue
            .iter()
            .chain(self.interruptible_queue.iter())
            .find(|task| task.tgid == tgid)
            .cloned()
    }
    /// 就绪队列中任务数量
    pub fn ready_count(&self) -> u16 {
        self.ready_queue.len() as u16
    }
    /// 可中断队列中任务数量
    pub fn interruptible_count(&self) -> u16 {
        self.interruptible_queue.len() as u16
    }
    /// 这个函数会将`task`从`interruptible_queue`中删除，并加入`ready_queue`。
    /// 如果一切正常的话，这个`task`将会被加入`ready_queue`。如果`task`已经被唤醒，那么什么也不会发生。
    /// # 注意
    /// 这个函数不会改变`task_status`，你应该手动改变它以保持一致性。
    pub fn wake_interruptible(&mut self, task: Arc<TaskControlBlock>) {
        match self.try_wake_interruptible(task) {
            Ok(_) => {}
            Err(_) => {
                log::trace!("[wake_interruptible] already waken");
            }
        }
    }
    /// 这个函数会将`task`从`interruptible_queue`中删除，并加入`ready_queue`。
    /// 如果一切正常的话，这个`task`将会被加入`ready_queue`。如果`task`已经被唤醒，那么返回`Err()`。
    /// # 注意
    /// 这个函数不会改变`task_status`，你应该手动改变它以保持一致性。
    pub fn try_wake_interruptible(
        &mut self,
        task: Arc<TaskControlBlock>,
    ) -> Result<(), WaitQueueError> {
        // 从可中断队列中删除指定任务
        self.drop_interruptible(&task);
        // 如果任务不在就绪队列中，将其加入就绪队列
        if self.find_by_pid(task.pid.0).is_none() {
            self.add(task);
            Ok(())
        } else {
            Err(WaitQueueError::AlreadyWaken)
        }
    }
    #[allow(unused)]
    /// 调试方法
    /// 打印就绪队列中的任务ID
    pub fn show_ready(&self) {
        self.ready_queue.iter().for_each(|task| {
            log::error!("[show_ready] pid: {}", task.pid.0);
        })
    }
    #[allow(unused)]
    /// 调试方法
    /// 打印可中断队列中的任务ID
    pub fn show_interruptible(&self) {
        self.interruptible_queue.iter().for_each(|task| {
            log::error!("[show_interruptible] pid: {}", task.pid.0);
        })
    }
}

lazy_static! {
    /// 全局任务管理器（带互斥锁）
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

/// 添加一个任务到任务管理器
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

/// 从任务管理器中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}

/// 尝试释放所有任务的内存空间，直到释放`req`页。
#[cfg(feature = "oom_handler")]
pub fn do_oom(req: usize) -> Result<(), ()> {
    let mut manager = TASK_MANAGER.lock();
    let mut cleaned = Vec::with_capacity(16);
    let mut total_released = 0;
    for task in manager
        .interruptible_queue
        .iter()
        .filter(|task| manager.active_tracker.check_active(task.pid.0))
    {
        let released = task.vm.lock().do_deep_clean();
        log::warn!("deep clean on task: {}, released: {}", task.tgid, released);
        cleaned.push(task.pid.0);
        total_released += released;
        if total_released >= req {
            while let Some(pid) = cleaned.pop() {
                manager.active_tracker.mark_inactive(pid)
            }
            return Ok(());
        };
    }
    for task in manager
        .ready_queue
        .iter()
        .rev()
        .filter(|task| manager.active_tracker.check_active(task.pid.0))
    {
        let released = task.vm.lock().do_shallow_clean();
        log::warn!(
            "shallow clean on task: {}, released: {}",
            task.tgid,
            released
        );
        cleaned.push(task.pid.0);
        total_released += released;
        if total_released >= req {
            while let Some(pid) = cleaned.pop() {
                manager.active_tracker.mark_inactive(pid)
            }
            return Ok(());
        };
    }
    Err(())
}

#[cfg(not(feature = "oom_handler"))]
#[allow(unused)]
pub fn do_oom() {
    // do nothing
}

/// 这个函数会将`task`加入到`interruptible_queue`，
/// 但不会从`ready_queue`中删除。
/// 所以需要确保`task`不会出现在`ready_queue`中。
/// 在一般情况下，一个`task`在被调度后会从`ready_queue`中删除，
/// 并且你可以使用`take_current_task()`来获取当前`task`的所有权。
/// # 注意
/// 你应该找一个地方保存`task`的`Arc<TaskControlBlock>`，
/// 否则你将无法在将来使用`wake_interruptible()`来唤醒它。
/// 这个函数不会改变`task_status`，你应该手动改变它以保持一致性。
pub fn sleep_interruptible(task: Arc<TaskControlBlock>) {
    // 将任务加入可中断队列
    TASK_MANAGER.lock().add_interruptible(task);
}

/// 这个函数会将`task`从`interruptible_queue`中删除，并加入到`ready_queue`中。
/// 这个`task`会在一切正常的情况下被调度。如果`task`已经被唤醒，什么也不会发生。
/// # 注意
/// 这个函数不会改变`task_status`，你应该手动改变它以保持一致性。
pub fn wake_interruptible(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().wake_interruptible(task)
}

/// # 警告
/// 这里的`pid`是唯一的，用户会将其视为`tid`
pub fn find_task_by_pid(pid: usize) -> Option<Arc<TaskControlBlock>> {
    // 获取当前任务
    let task = current_task().unwrap();
    // 如果当前任务的pid与指定的pid相同，返回当前任务
    if task.pid.0 == pid {
        Some(task)
    } else {
        // 否则从任务管理器中查找
        TASK_MANAGER.lock().find_by_pid(pid)
    }
}

/// 返回线程组ID为`tgid`的任意任务。
pub fn find_task_by_tgid(tgid: usize) -> Option<Arc<TaskControlBlock>> {
    // 获取当前任务
    let task = current_task().unwrap();
    // 如果当前任务的tgid与指定的tgid相同，返回当前任务
    if task.tgid == tgid {
        Some(task)
    } else {
        // 否则从任务管理器中查找
        TASK_MANAGER.lock().find_by_tgid(tgid)
    }
}

/// 返回就绪队列中的任务数量
pub fn procs_count() -> u16 {
    let manager = TASK_MANAGER.lock();
    manager.ready_count() + manager.interruptible_count()
}

/// 等待队列错误类型
pub enum WaitQueueError {
    /// 已经唤醒
    AlreadyWaken,
}

/// 等待队列
/// 内部是一个存储任务控制块弱引用的双端队列
pub struct WaitQueue {
    inner: VecDeque<Weak<TaskControlBlock>>,
}

#[allow(unused)]
impl WaitQueue {
    /// 构造函数
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }
    /// 这个函数将一个`task`添加到 `WaitQueue`但是不会阻塞这个任务
    /// 如果想要阻塞一个`task`，使用`block_current_and_run_next()`
    pub fn add_task(&mut self, task: Weak<TaskControlBlock>) {
        // 将task添加到back端
        self.inner.push_back(task);
    }
    /// 这个函数会尝试从`WaitQueue`中弹出一个`task`，但是不会唤醒它
    pub fn pop_task(&mut self) -> Option<Weak<TaskControlBlock>> {
        // 将front端的任务弹出
        self.inner.pop_front()
    }
    /// 判断等待队列是否包含给定的task
    pub fn contains(&self, task: &Weak<TaskControlBlock>) -> bool {
        self.inner
            .iter()
            .any(|task_in_queue| Weak::as_ptr(task_in_queue) == Weak::as_ptr(task))
    }
    /// 判断等待队列是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// 这个函数将会唤醒等待队列中所有的任务，并将它们的任务状态改变为就绪态，
    /// 如果一切正常，这些任务会在将来被调度。
    /// # 警告
    /// 这个函数会为每个被唤醒的`task`调用`acquire_inner_lock`，请注意**死锁**
    pub fn wake_all(&mut self) -> usize {
        self.wake_at_most(usize::MAX)
    }
    /// 唤醒不超过`limit`个`task`，返回唤醒的`task`数量。
    /// # 警告
    /// 这个函数会为每个被唤醒的`task`调用`acquire_inner_lock`，请注意**死锁**
    pub fn wake_at_most(&mut self, limit: usize) -> usize {
        // 如果limit为0，直接返回0
        if limit == 0 {
            return 0;
        }
        // 获取全局任务管理器
        let mut manager = TASK_MANAGER.lock();
        // 初始化计数器
        let mut cnt = 0;
        // 遍历内部队列，从self.inner中逐个取出任务处理
        while let Some(task) = self.inner.pop_front() {
            // 检查任务的弱引用是否仍然有效
            // 将弱引用升级为强引用
            match task.upgrade() {
                Some(task) => {
                    // 获取任务的内部锁
                    let mut inner = task.acquire_inner_lock();
                    // 检查任务状态
                    match inner.task_status {
                        // 可中断状态
                        super::TaskStatus::Interruptible => {
                            // 将任务状态改为就绪态
                            inner.task_status = super::task::TaskStatus::Ready
                        }
                        // 对于处于 就绪态或运行态的任务，不需要做唤醒操作
                        // 对于处于僵尸态的任务，做唤醒操作会搞砸进程管理
                        _ => continue,
                    }
                    // 释放内部锁
                    drop(inner);
                    // 唤醒任务
                    if manager.try_wake_interruptible(task).is_ok() {
                        cnt += 1;
                    }
                    // 到达数量限制，停止遍历
                    if cnt == limit {
                        break;
                    }
                }
                // task is dead, just ignore
                None => continue,
            }
        }
        cnt
    }
}

/// 表示一个等待超时的任务
pub struct TimeoutWaiter {
    /// 任务的弱引用
    task: Weak<TaskControlBlock>,
    /// 任务超时时间
    timeout: TimeSpec,
}

// 二叉堆是最大堆，所以我们需要反转排序
impl Ord for TimeoutWaiter {
    fn cmp(&self, other: &Self) -> Ordering {
        Ordering::reverse(self.timeout.cmp(&other.timeout))
    }
}

impl PartialOrd for TimeoutWaiter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for TimeoutWaiter {}

impl PartialEq for TimeoutWaiter {
    /// 仅通过比较timeout字段
    fn eq(&self, other: &Self) -> bool {
        self.timeout.eq(&other.timeout)
    }
}

/// 等待超时任务队列
pub struct TimeoutWaitQueue {
    /// 使用二叉堆存储任务（最大堆），按超时时间排序
    inner: BinaryHeap<TimeoutWaiter>,
}

impl TimeoutWaitQueue {
    /// 构造函数
    pub fn new() -> Self {
        Self {
            inner: BinaryHeap::new(),
        }
    }
    /// 这个函数会将一个`task`添加到`WaitQueue`但是**不会**阻塞这个任务，
    /// 如果想要阻塞一个`task`，使用`block_current_and_run_next()`函数
    pub fn add_task(&mut self, task: Weak<TaskControlBlock>, timeout: TimeSpec) {
        self.inner.push(TimeoutWaiter { task, timeout });
    }
    /// 唤醒所有超时的任务
    pub fn wake_expired(&mut self, now: TimeSpec) {
        // 获取任务管理器
        let mut manager = TASK_MANAGER.lock();
        // 循环处理超时任务
        while let Some(waiter) = self.inner.pop() {
            // 堆中剩下的任务还没有超时
            if waiter.timeout > now {
                // 若超时时间大于当前时间，说明后面的任务都没有超时
                log::trace!(
                    "[wake_expired] no more expired, next pending task timeout: {:?}, now: {:?}",
                    waiter.timeout,
                    now
                );
                self.inner.push(waiter);
                break;
            // 唤醒超时任务
            } else {
                // 将弱引用升级为强引用
                match waiter.task.upgrade() {
                    Some(task) => {
                        // 获取内部锁
                        let mut inner = task.acquire_inner_lock();
                        match inner.task_status {
                            // 若状态为可中断状态，改为就绪态
                            super::TaskStatus::Interruptible => {
                                inner.task_status = super::task::TaskStatus::Ready
                            }
                            // 对于处于 就绪态或运行态的任务，不需要做唤醒操作
                            // 对于处于僵尸态的任务，做唤醒操作会搞砸进程管理
                            _ => continue,
                        }
                        // 释放锁
                        drop(inner);
                        log::trace!(
                            "[wake_expired] pid: {}, timeout: {:?}",
                            task.pid.0,
                            waiter.timeout
                        );
                        manager.wake_interruptible(task);
                    }
                    // task is dead, just ignore
                    None => continue,
                }
            }
        }
    }
    #[allow(unused)]
    // debug use only
    pub fn show_waiter(&self) {
        for waiter in self.inner.iter() {
            log::error!("[show_waiter] timeout: {:?}", waiter.timeout);
        }
    }
}

lazy_static! {
    /// 全局超时等待队列
    pub static ref TIMEOUT_WAITQUEUE: Mutex<TimeoutWaitQueue> = Mutex::new(TimeoutWaitQueue::new());
}

/// 这个函数会将一个`task`添加到全局超时等待队列中，但是不会阻塞它
/// 如果想要阻塞一个任务，使用`block_current_and_run_next()`函数
pub fn wait_with_timeout(task: Weak<TaskControlBlock>, timeout: TimeSpec) {
    TIMEOUT_WAITQUEUE.lock().add_task(task, timeout)
}

/// 唤醒全局超时等待队列中所有已超时的任务
pub fn do_wake_expired() {
    TIMEOUT_WAITQUEUE
        .lock()
        .wake_expired(crate::timer::TimeSpec::now());
}
