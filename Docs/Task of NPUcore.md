# Task of NPUcore

方法论为：

从个性中找共性，从共性中找个性。

留下共性，封装个性，对齐接口。

已经处理好的内容&不需要处理的内容

- [x] context
- [x] elf
- [x] manager
- [x] threads
- [ ] task
- [ ] signal
- [x] processor
- [x] pid
- [ ] mod

## context

内容最简单的一个文件，包含了一个结构体类型`TaskContext`以及其方法实现

```rust
#[repr(C)]
/// 任务上下文
pub struct TaskContext {
    // 返回地址
    ra: usize,
    // 栈指针
    sp: usize,
    // 通用寄存器
    s: [usize; 12],
}
```

方法：

```rust
impl TaskContext {
    // 空初始化
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    // 
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
```

## elf

包含了一个枚举类型 `AuxvType`，即Auxiliary Vector，辅助向量。

辅助向量是操作系统传递给程序的一些额外信息，通常用于程序的初始化和运行时环境配置。

```rust
#[derive(Clone, Copy)]
#[allow(non_camel_case_types, unused)]
#[repr(usize)]
pub enum AuxvType {
    NULL = 0,
    IGNORE = 1,
    EXECFD = 2,
    PHDR = 3,
    PHENT = 4,
    PHNUM = 5,
    PAGESZ = 6, //系统页大小
    BASE = 7,
    FLAGS = 8,
    ENTRY = 9,	// 程序入口地址
    NOTELF = 10,
    UID = 11,
    EUID = 12,
    GID = 13,
    EGID = 14,
    PLATFORM = 15,
    HWCAP = 16,
    CLKTCK = 17,
    FPUCW = 18,
    DCACHEBSIZE = 19,
    ICACHEBSIZE = 20,
    UCACHEBSIZE = 21,
    IGNOREPPC = 22,
    SECURE = 23,
    BASE_PLATFORM = 24,
    RANDOM = 25,
    HWCAP2 = 26,
    EXECFN = 31,
    SYSINFO = 32,
    SYSINFO_EHDR = 33,
    L1I_CACHESHAPE = 34,
    L1D_CACHESHAPE = 35,
    L2_CACHESHAPE = 36,
    L3_CACHESHAPE = 37,
    L1I_CACHESIZE = 40,
    L1I_CACHEGEOMETRY = 41,
    L1D_CACHESIZE = 42,
    L1D_CACHEGEOMETRY = 43,
    L2_CACHESIZE = 44,
    L2_CACHEGEOMETRY = 45,
    L3_CACHESIZE = 46,
    L3_CACHEGEOMETRY = 47,
    MINSIGSTKSZ = 51,
}
```

再然后是一个`AuxvEntry`结构体类型

```rust
#[derive(Clone, Copy)]
#[allow(unused)]
#[repr(C)]
pub struct AuxvEntry {
    // 一个AuxvType字段
    auxv_type: AuxvType,
    auxv_val: usize,
}

impl AuxvEntry {
    pub fn new(auxv_type: AuxvType, auxv_val: usize) -> Self {
        Self {
            auxv_type,
            auxv_val,
        }
    }
}
```

接着是一个ELFInfo结构体，用于存储ELF文件的相关信息。

```rust
#[repr(C)]
pub struct ELFInfo {
    // 入口地址
    pub entry: usize,
    // 解析器入口地址
    pub interp_entry: Option<usize>,
    // 基地址
    pub base: usize,
    // 程序头表条目数量
    pub phnum: usize,
    // 程序头表条目大小
    pub phent: usize,
    // 程序头表地址
    pub phdr: usize,
}
```

最后是一个`load_elf_interp`函数，用于加载ELF文件的解释器

```rust

pub fn load_elf_interp(path: &str) -> Result<&'static [u8], isize> {
    // 只读方式打开指定path的文件
    match ROOT_FD.open(path, OpenFlags::O_RDONLY, false) {
        Ok(file) => {
            // 文件大小小于4个字节（即'\x7fELF'），返回错误 ELIBBAD
            if file.get_size() < 4 {
                return Err(ELIBBAD);
            }
            let mut magic_number = Box::<[u8; 4]>::new([0; 4]);
            // 原作者注：这个操作可能开销过大，后期得注意
            file.read(Some(&mut 0usize), magic_number.as_mut_slice());
            match magic_number.as_slice() {
                // 如果是ELF文件魔数
                b"\x7fELF" => {
                    // 获取内核空间的最高地址处
                    let buffer_addr = KERNEL_SPACE.lock().highest_addr();
                    // 从内核空间的最高地址处分配一个缓冲区，大小为文件大小
                    let buffer = unsafe {
                        core::slice::from_raw_parts_mut(buffer_addr.0 as *mut u8, file.get_size())
                    };
                    // 获取文件内容的缓存
                    let caches = file.get_all_caches().unwrap();
                    // 将文件内容缓存映射到缓冲区中
                    let frames = caches
                        .iter()
                        .map(|cache| Frame::InMemory(cache.try_lock().unwrap().get_tracker()))
                        .collect();

                    // 
                    crate::mm::KERNEL_SPACE
                        .lock()
                        .insert_program_area(
                            buffer_addr.into(),
                            crate::mm::MapPermission::R | crate::mm::MapPermission::W,
                            frames,
                        )
                        .unwrap();

                    return Ok(buffer);
                }
                // 不是ELF文件魔数
                _ => Err(ELIBBAD),
            }
        }
        // 其他类型错误
        Err(errno) => Err(errno),
    }
}
```

## manager

| 类型名称         | 作用                                                         | 方法                                                         |
| ---------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| ActiveTracker    | 内部是一个位图，位图用于存放任务的状态，`1`是激活，`0`是未激活 | new、check_activate、check_inactivate、mark_active、mark_inactive |
| TaskManager      | 存放有两个双端队列，分别对应就绪态任务以及可中断状态任务，然后还有一个`ActiveTracker`。有一个该类型的全局变量`TASK_MANAGER` | new, add, fetch, add_interruptible, drop_interruptible, find_by_pid, find_by_tgid, ready_count, interruptible_count, wake_interruptible, try_wake_interruptible, DEBUG::show_ready & show_interruptible |
| WaitQueue        | 等待队列，内部是一个双端队列，包装着一个任务控制块的弱引用   | new, add_task, pop_task, contains, is_empty, wake_all, wake_at_most |
| WaitQueueError   | 等待队列错误枚举类型，只存储了一种错误：`AlreadyWaken`       |                                                              |
| TimeoutWaiter    | 表示一个等待超时的任务。包含了一个`task`字段和一个`timeout`字段（时间戳类型） |                                                              |
| TimeoutWaitQueue | 用于管理所有等待超时的任务。包含了一个`inner`字段，为包含`TimeoutWaiter`的二叉堆，有一个该类型的全局变量`TIMEOUT_WAITQUEUE` | new, add_task, wake_expired, show_waiter                     |

首先是一个`ActiveTracker`类型

```rust
#[cfg(feature = "oom_handler")]
pub struct ActiveTracker {
    bitmap: Vec<u64>,
}
```

然后是其方法

```rust
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
```

紧接着是一个`TaskManager`类型

```rust
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
```

其方法

```rust

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
```

接着是一个TASK_MANAGER变量

```rust
lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}
```

以及围绕这个全局变量的多个函数：

```rust
/// 添加一个任务到任务管理器
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

/// 从任务管理器中取出一个任务
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
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
```

再接着是一个等待队列类型以及对应的等待错误类型

```rust
/// 等待队列
pub struct WaitQueue {
    inner: VecDeque<Weak<TaskControlBlock>>,
}

/// 等待队列错误类型
pub enum WaitQueueError {
    AlreadyWaken,
}
```

方法

```rust
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
```

最后是`TimeoutWaiter`以及`TimeoutWaitQueue`

```rust
/// 表示一个等待超时的任务
pub struct TimeoutWaiter {
    /// 任务的弱引用
    task: Weak<TaskControlBlock>,
    /// 任务超时时间
    timeout: TimeSpec,
}

/// 等待超时任务队列
pub struct TimeoutWaitQueue {
    /// 使用二叉堆存储任务（最大堆），按超时时间排序
    inner: BinaryHeap<TimeoutWaiter>,
}
```

方法

```rust
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
```

全局变量

```rust
lazy_static! {
    /// 全局超时等待队列
    pub static ref TIMEOUT_WAITQUEUE: Mutex<TimeoutWaitQueue> = Mutex::new(TimeoutWaitQueue::new());
}

// 围绕这个变量的函数

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
```

## threads

| 类型名称 | 作用             | 方法                      |
| -------- | ---------------- | ------------------------- |
| Futexcmd |                  |                           |
| Futex    | 用于存储等待队列 | new, wake, requeue, clear |

首先是一个Futexcmd类型

```rust
#[allow(unused)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
/// 定义了Futex支持的操作类型
pub enum FutexCmd {
    Wait = 0,
    Wake = 1,
    Fd = 2,
    Requeue = 3,
    CmpRequeue = 4,
    WakeOp = 5,
    LockPi = 6,
    UnlockPi = 7,
    TrylockPi = 8,
    WaitBitset = 9,
    #[num_enum(default)]
    // 不在范围内，默认值为Invalid
    Invalid,
}
```

然后是Futex类型

```rust
/// Fast Userspace Mutex
/// 快速用户空间互斥锁
/// # 作用
/// + 用于存储等待队列
/// # 参数
/// + key：usize
/// + value：WaitQueue
pub struct Futex {
    inner: BTreeMap<usize, WaitQueue>,
}
```

Futex的方法

```rust
impl Futex {
    /// 创建一个新的Futex
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// 唤醒等待在指定 Futex 地址上的最多 val 个任务
    pub fn wake(&mut self, futex_word_addr: usize, val: u32) -> isize {
        if let Some(mut wait_queue) = self.inner.remove(&futex_word_addr) {
            let ret = wait_queue.wake_at_most(val as usize);
            if !wait_queue.is_empty() {
                self.inner.insert(futex_word_addr, wait_queue);
            }
            ret as isize
        } else {
            0
        }
    }

    /// 重新排列
    pub fn requeue(&mut self, futex_word: &u32, futex_word_2: &u32, val: u32, val2: u32) -> isize {
        let futex_word_addr = futex_word as *const u32 as usize;
        let futex_word_addr_2 = futex_word_2 as *const u32 as usize;
        let wake_cnt = if val != 0 {
            self.wake(futex_word_addr, val)
        } else {
            0
        };
        if let Some(mut wait_queue) = self.inner.remove(&futex_word_addr) {
            let mut wait_queue_2 = if let Some(wait_queue) = self.inner.remove(&futex_word_addr_2) {
                wait_queue
            } else {
                WaitQueue::new()
            };
            let mut requeue_cnt = 0;
            if val2 != 0 {
                while let Some(task) = wait_queue.pop_task() {
                    wait_queue_2.add_task(task);
                    requeue_cnt += 1;
                    if requeue_cnt == val2 as isize {
                        break;
                    }
                }
            }
            if !wait_queue.is_empty() {
                self.inner.insert(futex_word_addr, wait_queue);
            }
            if !wait_queue_2.is_empty() {
                self.inner.insert(futex_word_addr_2, wait_queue_2);
            }
            wake_cnt + requeue_cnt
        } else {
            wake_cnt
        }
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
```

然后最后是一个函数`do_futex_wait`

```rust
/// 用于实现Futex的Wait操作
/// # 参数
/// - `futex_word`: 指向 Futex 变量的指针（可变引用）。
/// - `val`: 期望的 Futex 变量的值。如果 Futex 变量的当前值不等于 `val`，则立即返回错误。
/// - `timeout`: 可选的超时时间。如果指定了超时时间，任务将在超时后自动唤醒。
///
/// # 返回值
/// - 成功时返回 `SUCCESS`。
/// - 如果 Futex 变量的值不等于 `val`，返回 `EAGAIN`。
/// - 如果任务被信号中断，返回 `EINTR`。
pub fn do_futex_wait(futex_word: &mut u32, val: u32, timeout: Option<TimeSpec>) -> isize {
    // 将超时时间转换为绝对时间（当前时间 + 超时时间）
    let timeout = timeout.map(|t| t + TimeSpec::now());

    // 获取 Futex 变量的地址（转换为 usize 以便作为键使用）
    let futex_word_addr = futex_word as *const u32 as usize;

    // 检查 Futex 变量的当前值是否等于期望值 `val`
    if *futex_word != val {
        // 不匹配，记录日志并返回`EAGAIN`错误
        trace!(
            "[futex] --wait-- **not match** futex: {:X}, val: {:X}",
            *futex_word,
            val
        );
        return EAGAIN;
    } else {
        // 获取当前任务的引用。
        let task = current_task().unwrap();

        // 获取 Futex 的锁，以便修改等待队列。
        let mut futex = task.futex.lock();

        // 从 Futex 的等待队列中移除当前地址对应的队列（如果存在），否则创建一个新的等待队列。
        let mut wait_queue = if let Some(wait_queue) = futex.inner.remove(&futex_word_addr) {
            wait_queue
        } else {
            WaitQueue::new()
        };

        // 将当前任务添加到等待队列中
        // 使用 `Arc::downgrade` 将任务的强引用转换为弱引用，避免循环利用
        wait_queue.add_task(Arc::downgrade(&task));

        // 将更新后的等待队列重新插入到 Futex 的等待队列中。
        futex.inner.insert(futex_word_addr, wait_queue);

        // 如果指定了超时时间，将任务添加到超时等待队列中
        if let Some(timeout) = timeout {
            trace!("[do_futex_wait] sleep with timeout: {:?}", timeout);
            wait_with_timeout(Arc::downgrade(&task), timeout);
        }

        // 释放 Futex 锁和任务引用，避免死锁
        drop(futex);
        drop(task);

        // 阻塞当前任务并切换到下一个任务。
        block_current_and_run_next();

        // 当前任务被唤醒后，重新获取当前任务的引用。
        let task = current_task().unwrap();

        // 获取任务内部锁，以便检查信号。
        let inner = task.acquire_inner_lock();
        // 检查是否有未屏蔽的信号挂起
        if !inner.sigpending.difference(inner.sigmask).is_empty() {
            // 有未屏蔽的信号，返回 `EINTR` 错误。
            return EINTR;
        }

        // 如果没有信号中断，返回成功。
        SUCCESS
    }
}
```

## task



## signal



## processor

不同点：

+ RISCV有一个unused的current_kstack_top()，而LA版本没有，不管这个函数
+ LA有一个is_vacant函数，实现很简单，判断Option是否为空，无需特殊处理

| 类型名称  | 作用                                                     | 方法                                                        |
| --------- | -------------------------------------------------------- | ----------------------------------------------------------- |
| Processor | 表示一个处理器，管理当前正在运行的任务和空闲任务的上下文 | new, get_idle_task_cx_ptr, take_current, current, is_vacant |

```rust
/// 处理器对象
pub struct Processor {
    /// 当前正在运行的任务
    current: Option<Arc<TaskControlBlock>>,
    /// 空闲任务的上下文，用于在任务切换时保存和恢复状态
    idle_task_cx: TaskContext,
}
```

`Processor`的方法

```rust
impl Processor {
    /// 构造函数
    pub fn new() -> Self {
        Self {
            // 初始化时处理器为空闲
            current: None,
            // 空闲任务的上下文
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    /// 获取空闲任务的上下文指针
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    /// 取出当前正在运行的任务
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        // 将current字段置空，并返回其中的值
        self.current.take()
    }
    /// 获取当前正在运行的任务的克隆
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
    /// 检查当前 Processor 是否为空闲
    pub fn is_vacant(&self) -> bool {
        self.current.is_none()
    }
}
```

紧接着是一个全局的处理器对象

```rust
lazy_static! {
    /// 全局的处理器对象
    pub static ref PROCESSOR: Mutex<Processor> = Mutex::new(Processor::new());
}
```

围绕这个全局处理器对象有多个函数：

```rust
/// 运行任务调度
/// # 作用
/// 运行任务调度器，不断从任务队列中取出任务并运行
pub fn run_tasks() {
    loop {
        // 获取全局处理器对象
        let mut processor = PROCESSOR.lock();
        // 尝试从全局变量 TASK_MANAGER 中取出一个任务
        if let Some(task) = fetch_task() {
            // 获取当前空闲任务的上下文指针
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // 独占地访问即将运行的任务的 TCB
            let next_task_cx_ptr = {
                let mut task_inner = task.acquire_inner_lock();
                task_inner.task_status = TaskStatus::Running;
                &task_inner.task_cx as *const TaskContext
            };
            // 设置当前正在运行的任务
            processor.current = Some(task);
            // 手动释放处理器
            drop(processor);
            unsafe {
                // 调用__switch 函数(汇编)切换任务
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            // 如果没有任务
            // 释放处理器的锁
            drop(processor);
            // 没有就绪的任务，尝试唤醒一些任务
            do_wake_expired();
        }
    }
}

/// 取出当前正在运行的任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.lock().take_current()
}

/// 获取当前正在运行的任务
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.lock().current()
}

/// 获取当前正在运行的任务的用户态页表令牌
pub fn current_user_token() -> usize {
    current_task().unwrap().get_user_token()
}

/// 获取当前正在运行的任务的陷阱上下文
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_cx()
}

/// 切换到空闲任务上下文
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    // 获取空闲任务的上下文指针
    let idle_task_cx_ptr = PROCESSOR.lock().get_idle_task_cx_ptr();
    unsafe {
        // 调用__switch 函数(汇编)切换任务
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
```

## pid

不同点：

+ RV版本多了一个`KSTACK_ALLOCATOR`
+ LA版本多了一个`KernelStackImpl`

LA版本可能出于什么原因去掉了这个变量，同时这部分不一样的内容LA版本有一个叫做`kern_stack.rs`的文件，不一致的内容基本都存放在这里面。

鉴于这两部分内容都和具体体系结构相关，所以在arch模块下的riscv下创建一个kern_stack.rs文件，用于存放不一样的内容，而外处使用cfg选项或者包装来实现两种内容的切换。

此处先从LA版本开始分析

| 类型名称         | 作用                                     | 方法                               |
| ---------------- | ---------------------------------------- | ---------------------------------- |
| RecycleAllocator | 带有回收机制的分配器，用于分配和回收`id` | new, alloc, dealloc, get_allocated |
| PidHandle        |                                          |                                    |

```rust
/// 用于分配pid的结构体
pub struct RecycleAllocator {
    /// 当前分配的id
    current: usize,
    /// 存储已经回收的id，供后续分配使用
    recycled: Vec<usize>,
}
```

`RecycleAllocator`的方法

```rust
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
```

然后是一个全局变量

```rust
lazy_static! {
    /// 全局的PID分配器对象，使用Mutex进行包装保证线程安全
    static ref PID_ALLOCATOR: Mutex<RecycleAllocator> = Mutex::new(RecycleAllocator::new());
}
```

一个用于表示pid的句柄的类型`PidHandle`

```rust
/// 表示一个pid的句柄
/// 包装有一个pid号
pub struct PidHandle(pub usize);
```

接着是围绕上面那个pid分配器的函数：

```rust
/// 分配一个pid
pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.lock().alloc())
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}
```

最后是和RV版本不一样的地方

```rust
pub type KernelStackImpl = crate::hal::arch::KernelStack;
#[inline(always)]
pub fn kstack_alloc() -> KernelStackImpl {
    KernelStackImpl::new()
}
```

可以看到这个`KernelStackImpl`完全可以直接使用`KernelStack`替代

所以我们找到`kern_stack.rs`

```rust
pub struct KernelStack(Vec<u8>);
impl KernelStack {
    pub fn new() -> Self {
        Self(alloc::vec![0_u8; KERNEL_STACK_SIZE])
    }
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = Self::kernel_stack_position(&self.0);
        kernel_stack_top
    }
    /// Return (bottom, top) of a kernel stack in kernel space.
    fn kernel_stack_position(v: &Vec<u8>) -> (usize, usize) {
        /* let top: usize = TRAMPOLINE - kstack_id * (KERNEL_STACK_SIZE + PAGE_SIZE); */
        let bottom = &v[0] as *const u8 as usize;
        let top: usize = bottom + KERNEL_STACK_SIZE;
        (bottom, top)
    }
}

pub fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

pub fn ustack_bottom_from_tid(tid: usize) -> usize {
    USER_STACK_BASE - tid * (PAGE_SIZE + USER_STACK_SIZE)
}
```

然后将上面的不一样的地方进行改动，插到`kern_stack.rs`文件下面：

```rust
#[inline(always)]
pub fn kstack_alloc() -> KernelStack {
    KernelStack::new()
}
```

相应的要更改包引用的关系，原来的kstack_alloc就可以删掉，然后在使用的地方改为如下形式：

```rust
pub use kern_stack::kstack_alloc;
```

此外，还需要对RV版本的这些内容做进一步处理：为arch下的riscv模块也创建一个`kern_stack.rs`文件

如下是针对2022版本处理过后的内容，后面可能需要进一步修改（**因为目前不知道如何高效的让`rust-analyzer`解析cfg中配置条件为假的内容，来回改动Cargo.toml文件略显麻烦，后面去查找是否存在点一下就可以切换解析内容的工具**）

`kern_stack.rs of NPUcore ver.RV`

```rust
use super::config::{
    KERNEL_STACK_SIZE, PAGE_SIZE, TRAP_CONTEXT_BASE, USER_STACK_BASE, USER_STACK_SIZE,
};
use crate::mm::{MapPermission, VirtAddr, KERNEL_SPACE};
use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;
lazy_static! {
    static ref KSTACK_ALLOCATOR: Mutex<RecycleAllocator> = Mutex::new(RecycleAllocator::new());
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(kstack_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - kstack_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub struct KernelStack(pub usize);

pub fn kstack_alloc() -> KernelStack {
    let kstack_id = KSTACK_ALLOCATOR.lock().alloc();
    let (kstack_bottom, kstack_top) = kernel_stack_position(kstack_id);
    KERNEL_SPACE.lock().insert_framed_area(
        kstack_bottom.into(),
        kstack_top.into(),
        MapPermission::R | MapPermission::W,
    );
    KernelStack(kstack_id)
}

impl Drop for KernelStack {
    fn drop(&mut self) {...}
}

impl KernelStack {
    #[allow(unused)]
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {...}
    pub fn get_top(&self) -> usize {...}
}

pub fn trap_cx_bottom_from_tid(tid: usize) -> usize {...}
pub fn ustack_bottom_from_tid(tid: usize) -> usize {...}
```

相应的还得在`hal/arch/riscv`下添加这几个文件：

+ config.rs  体系结构对应配置文件
+ switch.S 上下文切换的汇编代码
+ switch.rs 调用上述汇编代码的代码文件

## mod

