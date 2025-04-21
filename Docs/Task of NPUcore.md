# Task of NPUcore

方法论为：

从个性中找共性，从共性中找个性。

留下共性，封装个性，对齐接口。

已经处理好的内容&不需要处理的内容

- [x] context
- [x] elf
- [x] manager
- [x] threads
- [x] task
- [x] signal
- [x] processor
- [x] pid
- [x] mod

## 一、context

### 1.1 TaskContext

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

## 二、elf

### 2.1 AuxvType

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

### 2.2 AuxvEntry

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

### 2.3 ELFInfo

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

### 3.4 Funcs

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

## 三、manager

| 类型名称         | 作用                                                         | 方法                                                         |
| ---------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| ActiveTracker    | 内部是一个位图，位图用于存放任务的状态，`1`是激活，`0`是未激活 | new、check_activate、check_inactivate、mark_active、mark_inactive |
| TaskManager      | 存放有两个双端队列，分别对应就绪态任务以及可中断状态任务，然后还有一个`ActiveTracker`。有一个该类型的全局变量`TASK_MANAGER` | new, add, fetch, add_interruptible, drop_interruptible, find_by_pid, find_by_tgid, ready_count, interruptible_count, wake_interruptible, try_wake_interruptible, DEBUG::show_ready & show_interruptible |
| WaitQueue        | 等待队列，内部是一个双端队列，包装着一个任务控制块的弱引用   | new, add_task, pop_task, contains, is_empty, wake_all, wake_at_most |
| WaitQueueError   | 等待队列错误枚举类型，只存储了一种错误：`AlreadyWaken`       |                                                              |
| TimeoutWaiter    | 表示一个等待超时的任务。包含了一个`task`字段和一个`timeout`字段（时间戳类型） |                                                              |
| TimeoutWaitQueue | 用于管理所有等待超时的任务。包含了一个`inner`字段，为包含`TimeoutWaiter`的二叉堆，有一个该类型的全局变量`TIMEOUT_WAITQUEUE` | new, add_task, wake_expired, show_waiter                     |

### 3.1 ActiveTracker

首先是一个`ActiveTracker`类型

```rust
#[cfg(feature = "oom_handler")]
pub struct ActiveTracker {
    bitmap: Vec<u64>,
}
```

其方法

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

### 3.2 TaskManager

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

### 3.3 WaitQueue

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

### 3.4 TimeoutWaiter & TimeoutWaitQueue

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

## 四、threads

| 类型名称 | 作用             | 方法                      |
| -------- | ---------------- | ------------------------- |
| Futexcmd |                  |                           |
| Futex    | 用于存储等待队列 | new, wake, requeue, clear |

### 4.1 Futexcmd

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

### 4.2 Futex

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

### 4.3 Funcs

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

## 五、task

二者不一样的地方主要在对translate函数（虚实地址转换函数）结果进行的处理，RV版本还需要进行ppn()方法调用，而LA版本直接省略了这一步。此外LA版本还多了一些socket相关的内容（net模块下），比较好处理。再者就是`update_process_times_leave_trap`函数签名略有不同，LA版本与pid内容中一样，多创建了一个针对于`Trap`的类型`TrapImpl`，这里也比较好处理。

| 类型名称              | 作用                                             | 方法                                                         |
| --------------------- | ------------------------------------------------ | ------------------------------------------------------------ |
| FsStatus              | 表示任务的文件系统状态。                         |                                                              |
| TaskControlBlock      | 表示一个任务的控制块，包含任务的所有状态和资源。 | acquire_inner_lock, trap_cx_user_va, ustack_bottom_from_tid, new, load_elf, sys_clone, get_pid, set_pid, getpgid, get_user_token |
| TaskControlBlockInner | 表示任务内部的动态状态（更接近硬件层/底层）      | get_trap_cx, get_status, is_zombie, add_signal, update_process_times_enter_trap, update_process_times_leave_trap, update_itimer_* |
| RobustList            | 鲁棒列表                                         | default                                                      |
| ProcClock             | 进程时钟                                         | new                                                          |
| Rusage                | 资源使用情况                                     | new                                                          |
| TaskStatus            | 任务状态，包含僵尸态、运行态、就绪态、可中断态   |                                                              |

### 5.1 FsStatus

首先是内容最简单的`FsStatus`

```rust
#[derive(Clone)]
pub struct FsStatus {
    /// 当前工作目录的文件描述符
    pub working_inode: Arc<FileDescriptor>,
}
```

### 5.2 TaskControlBlock

然后是任务控制块类型

```rust
/// 任务控制块
pub struct TaskControlBlock {
    // 不可变字段
    /// 进程ID
    pub pid: PidHandle,
    /// 线程ID
    pub tid: usize,
    /// 线程组ID
    pub tgid: usize,
    /// 内核栈
    pub kstack: KernelStack,
    /// 用户栈基址
    pub ustack_base: usize,
    /// 退出信号
    pub exit_signal: Signals,
    // 可变字段
    /// 任务内部状态，使用互斥锁保护
    inner: Mutex<TaskControlBlockInner>,
    // 可共享&可变字段
    /// 可执行文件描述符
    pub exe: Arc<Mutex<FileDescriptor>>,
    /// 线程ID分配器
    pub tid_allocator: Arc<Mutex<RecycleAllocator>>,
    /// 文件描述符表
    pub files: Arc<Mutex<FdTable>>,
    /// Socket表
    pub socket_table: Arc<Mutex<SocketTable>>,
    /// 文件系统状态
    pub fs: Arc<Mutex<FsStatus>>,
    /// 虚拟内存空间
    #[cfg(feature = "loongarch64")]
    pub vm: Arc<Mutex<MemorySet<PageTableImpl>>>,
    #[cfg(feature = "riscv")]
    pub vm: Arc<Mutex<MemorySet>>,
    /// 信号处理函数表
    pub sighand: Arc<Mutex<Vec<Option<Box<SigAction>>>>>,
    /// 快速用户空间互斥锁
    pub futex: Arc<Mutex<Futex>>,
}
```

其方法

```rust
impl TaskControlBlock {
    /// 获取任务内部状态的互斥锁
    pub fn acquire_inner_lock(&self) -> MutexGuard<TaskControlBlockInner> {
        self.inner.lock()
    }
    /// 获取陷阱上下文的用户虚拟地址
    pub fn trap_cx_user_va(&self) -> usize {
        // 从线程ID计算陷阱上下文的用户虚拟地址
        trap_cx_bottom_from_tid(self.tid)
    }
    /// 获取用户栈的用户虚拟地址
    pub fn ustack_bottom_va(&self) -> usize {
        // 从线程ID计算用户栈的用户虚拟地址
        ustack_bottom_from_tid(self.tid)
    }
    /// !!!!!!!!!!!!!!!!WARNING!!!!!!!!!!!!!!!!!!!!!
    /// 当前仅用于initproc加载。如果在其他地方使用，必须更改bin_path。
    /// 任务创建（仅用于initproc）
    pub fn new(elf: FileDescriptor) -> Self {
        // 将ELF文件映射到内核空间
        let elf_data = elf.map_to_kernel_space(MMAP_BASE);
        // 带有ELF程序头/跳板的内存集（MemorySet）
        // 解析ELF文件，初始化内存映射
        let (mut memory_set, user_heap, elf_info) = MemorySet::from_elf(elf_data).unwrap();
        // 在内核空间中删除ELF区域
        crate::mm::KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(VirtAddr::from(MMAP_BASE).floor())
            .unwrap();

        // 获取线程ID分配器
        let tid_allocator = Arc::new(Mutex::new(RecycleAllocator::new()));
        // 在内核空间中分配一个PID和一个内核栈
        let pid_handle = pid_alloc();
        // 分配线程ID
        let tid = tid_allocator.lock().alloc();
        // 线程组ID和线程ID相同
        let tgid = pid_handle.0;
        let pgid = pid_handle.0;
        // 分配内核栈
        let kstack = kstack_alloc();
        // 获取内核栈的顶部
        let kstack_top = kstack.get_top();

        // 为当前线程分配用户资源
        memory_set.alloc_user_res(tid, true);
        // 获取陷阱上下文的物理页号
        #[cfg(feature = "loongarch64")]
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap();
        #[cfg(feature = "riscv")]
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap()
            .ppn();
        log::trace!("[TCB::new]trap_cx_ppn{:?}", trap_cx_ppn);
        // 创建任务控制块
        let task_control_block = Self {
            pid: pid_handle,
            tid,
            tgid,
            kstack,
            ustack_base: ustack_bottom_from_tid(tid),
            exit_signal: Signals::empty(),
            exe: Arc::new(Mutex::new(elf)),
            tid_allocator,
            files: Arc::new(Mutex::new(FdTable::new({
                let mut vec = Vec::with_capacity(144);
                let tty = Some(ROOT_FD.open("/dev/tty", OpenFlags::O_RDWR, false).unwrap());
                vec.resize(3, tty);
                vec
            }))),
            socket_table: Arc::new(Mutex::new(SocketTable::new())),
            fs: Arc::new(Mutex::new(FsStatus {
                working_inode: Arc::new(
                    ROOT_FD
                        .open(".", OpenFlags::O_RDONLY | OpenFlags::O_DIRECTORY, true)
                        .unwrap(),
                ),
            })),
            vm: Arc::new(Mutex::new(memory_set)),
            sighand: Arc::new(Mutex::new({
                let mut vec = Vec::with_capacity(64);
                vec.resize(64, None);
                vec
            })),
            futex: Arc::new(Mutex::new(Futex::new())),
            inner: Mutex::new(TaskControlBlockInner {
                sigmask: Signals::empty(),
                sigpending: Signals::empty(),
                trap_cx_ppn,
                task_cx: TaskContext::goto_trap_return(kstack_top),
                task_status: TaskStatus::Ready,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
                clear_child_tid: 0,
                robust_list: RobustList::default(),
                heap_bottom: user_heap,
                heap_pt: user_heap,
                pgid,
                rusage: Rusage::new(),
                clock: ProcClock::new(),
                timer: [ITimerVal::new(); 3],
            }),
        };
        // 准备用户空间的陷阱上下文
        let trap_cx = task_control_block.acquire_inner_lock().get_trap_cx();
        // 初始化陷阱上下文
        *trap_cx = TrapContext::app_init_context(
            elf_info.entry,
            ustack_bottom_from_tid(tid),
            KERNEL_SPACE.lock().token(),
            kstack_top,
            trap_handler as usize,
        );
        trace!("[new] trap_cx:{:?}", *trap_cx);
        task_control_block
    }

    /// 加载ELF文件
    pub fn load_elf(
        &self,
        elf: FileDescriptor,
        argv_vec: &Vec<String>,
        envp_vec: &Vec<String>,
    ) -> Result<(), isize> {
        // 将ELF文件映射到内核空间
        let elf_data = elf.map_to_kernel_space(MMAP_BASE);
        // 带有ELF程序头/跳板/陷阱上下文/用户栈的内存集（MemorySet）
        let (mut memory_set, program_break, elf_info) = MemorySet::from_elf(elf_data)?;
        log::trace!("[load_elf] ELF file mapped");
        // 清除临时映射
        crate::mm::KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(VirtAddr::from(MMAP_BASE).floor())
            .unwrap();
        // 为当前线程分配用户资源
        memory_set.alloc_user_res(self.tid, true);
        // 创建ELF参数表
        let user_sp =
            memory_set.create_elf_tables(self.ustack_bottom_va(), argv_vec, envp_vec, &elf_info);
        log::trace!("[load_elf] user sp after pushing parameters: {:X}", user_sp);
        // 初始化陷阱上下文
        let trap_cx = TrapContext::app_init_context(
            if let Some(interp_entry) = elf_info.interp_entry {
                interp_entry
            } else {
                elf_info.entry
            },
            // 用户栈指针
            user_sp,
            // 内核页表令牌
            KERNEL_SPACE.lock().token(),
            // 内核栈顶
            self.kstack.get_top(),
            // 陷阱处理函数地址
            trap_handler as usize,
        );
        // **** 保持当前PCB锁
        let mut inner = self.acquire_inner_lock();
        // 更新陷阱上下文的物理页号
        #[cfg(feature = "loongarch64")]
        {
            inner.trap_cx_ppn = (&memory_set)
                .translate(VirtAddr::from(self.trap_cx_user_va()).into())
                .unwrap();
        }
        #[cfg(feature = "riscv")]
        {
            inner.trap_cx_ppn = (&memory_set)
                .translate(VirtAddr::from(self.trap_cx_user_va()).into())
                .unwrap()
                .ppn();
        }
        // 更新任务上下文
        *inner.get_trap_cx() = trap_cx;
        // 重置clear_child_tid
        inner.clear_child_tid = 0;
        // 重置robust_list
        inner.robust_list = RobustList::default();
        // 更新堆指针
        inner.heap_bottom = program_break;
        inner.heap_pt = program_break;
        // 更新可执行文件描述符
        *self.exe.lock() = elf;
        // 清理资源
        // 关闭原文件描述符
        self.files.lock().iter_mut().for_each(|fd| match fd {
            Some(file) => {
                if file.get_cloexec() {
                    *fd = None;
                }
            }
            None => (),
        });
        // 替换内存映射
        *self.vm.lock() = memory_set;
        // 清空信号处理函数表
        for sigact in self.sighand.lock().iter_mut() {
            *sigact = None;
        }
        // 清空futex
        self.futex.lock().clear();
        // 检查当前任务是否是多线程任务
        if self.tid_allocator.lock().get_allocated() > 1 {
            let mut manager = TASK_MANAGER.lock();
            // 销毁所有其他同一线程组的任务
            manager
                .ready_queue
                .retain(|task| (*task).tgid != (*self).tgid);
            manager
                .interruptible_queue
                .retain(|task| (*task).tgid != (*self).tgid);
        };
        Ok(())
        // **** 释放当前PCB锁
    }
    /// 创建新的任务控制块
    pub fn sys_clone(
        self: &Arc<TaskControlBlock>,
        flags: CloneFlags,
        stack: *const u8,
        tls: usize,
        exit_signal: Signals,
    ) -> Arc<TaskControlBlock> {
        // ---- 保持父PCB锁
        let mut parent_inner = self.acquire_inner_lock();
        // 复制用户空间（包括陷阱上下文）
        let memory_set = if flags.contains(CloneFlags::CLONE_VM) {
            self.vm.clone() // 共享虚拟内存空间（线程）
        } else {
            // 复制地址空间（进程）
            crate::mm::frame_reserve(16);
            Arc::new(Mutex::new(MemorySet::from_existing_user(
                &mut self.vm.lock(),
            )))
        };

        // 复制线程ID分配器
        let tid_allocator = if flags.contains(CloneFlags::CLONE_THREAD) {
            self.tid_allocator.clone()
        } else {
            Arc::new(Mutex::new(RecycleAllocator::new()))
        };
        // 在内核空间分配一个PID和一个内核栈
        let pid_handle = pid_alloc(); // 分配PID
        let tid = tid_allocator.lock().alloc(); // 分配线程ID
        let tgid = if flags.contains(CloneFlags::CLONE_THREAD) {
            // 共享线程组ID
            self.tgid
        } else {
            // 新建线程组ID（进程）
            pid_handle.0
        };
        // 分配内核栈
        let kstack = kstack_alloc();
        let kstack_top = kstack.get_top();

        // 如果是线程，分配用户空间资源
        if flags.contains(CloneFlags::CLONE_THREAD) {
            memory_set.lock().alloc_user_res(tid, stack.is_null());
        }
        // 获取陷阱上下文的物理页号
        #[cfg(feature = "loongarch64")]
        let trap_cx_ppn = memory_set
            .lock()
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap();
        #[cfg(feature = "riscv")]
        let trap_cx_ppn = memory_set
            .lock()
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap()
            .ppn();

        // 创建任务控制块
        let task_control_block = Arc::new(TaskControlBlock {
            // 基础标识信息
            pid: pid_handle,
            tid,
            tgid,
            kstack,
            ustack_base: if !stack.is_null() {
                stack as usize
            } else {
                ustack_bottom_from_tid(tid)
            },
            exit_signal,

            // 资源共享控制
            exe: self.exe.clone(),
            tid_allocator,
            files: if flags.contains(CloneFlags::CLONE_FILES) {
                self.files.clone()
            } else {
                Arc::new(Mutex::new(self.files.lock().clone()))
            },
            socket_table: Arc::new(Mutex::new(
                SocketTable::from_another(&self.socket_table.clone().lock()).unwrap(),
            )),
            fs: if flags.contains(CloneFlags::CLONE_FS) {
                self.fs.clone()
            } else {
                Arc::new(Mutex::new(self.fs.lock().clone()))
            },
            vm: memory_set,
            sighand: if flags.contains(CloneFlags::CLONE_SIGHAND) {
                self.sighand.clone()
            } else {
                Arc::new(Mutex::new(self.sighand.lock().clone()))
            },
            futex: if flags.contains(CloneFlags::CLONE_SYSVSEM) {
                self.futex.clone()
            } else {
                // maybe should do clone here?
                Arc::new(Mutex::new(Futex::new()))
            },
            inner: Mutex::new(TaskControlBlockInner {
                // inherited
                pgid: parent_inner.pgid,
                heap_bottom: parent_inner.heap_bottom,
                heap_pt: parent_inner.heap_pt,
                // clone
                sigpending: parent_inner.sigpending.clone(),
                // new
                children: Vec::new(),
                rusage: Rusage::new(),
                clock: ProcClock::new(),
                clear_child_tid: 0,
                robust_list: RobustList::default(),
                timer: [ITimerVal::new(); 3],
                sigmask: Signals::empty(),
                // compute
                trap_cx_ppn,
                task_cx: TaskContext::goto_trap_return(kstack_top),
                parent: if flags.contains(CloneFlags::CLONE_PARENT)
                    | flags.contains(CloneFlags::CLONE_THREAD)
                {
                    parent_inner.parent.clone()
                } else {
                    Some(Arc::downgrade(self))
                },
                // constants
                task_status: TaskStatus::Ready,
                exit_code: 0,
            }),
        });
        // 添加到父进程或者祖父进程的子进程列表
        if flags.contains(CloneFlags::CLONE_PARENT) || flags.contains(CloneFlags::CLONE_THREAD) {
            if let Some(grandparent) = &parent_inner.parent {
                grandparent
                    .upgrade()
                    .unwrap()
                    .acquire_inner_lock()
                    .children
                    .push(task_control_block.clone());
            }
        } else {
            parent_inner.children.push(task_control_block.clone());
        }
        // 初始化陷阱上下文
        let trap_cx = task_control_block.acquire_inner_lock().get_trap_cx();
        // 如果是线程，复制陷阱上下文
        if flags.contains(CloneFlags::CLONE_THREAD) {
            *trap_cx = *parent_inner.get_trap_cx();
        }
        // we also do not need to prepare parameters on stack, musl has done it for us
        // 处理用户栈指针
        if !stack.is_null() {
            trap_cx.gp.sp = stack as usize;
        }
        // 设置线程寄存器
        if flags.contains(CloneFlags::CLONE_SETTLS) {
            // thread local storage
            // 线程局部存储
            trap_cx.gp.tp = tls;
        }
        // 对于子进程，fork返回0
        trap_cx.gp.a0 = 0;
        // 修改陷阱上下文中的内核栈指针
        trap_cx.kernel_sp = kstack_top;
        // 返回
        task_control_block
        // ---- 释放父PCB锁
    }
    /// 获取进程ID
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
    /// 设置进程组ID
    pub fn setpgid(&self, pgid: usize) -> isize {
        if (pgid as isize) < 0 {
            return -1;
        }
        let mut inner = self.acquire_inner_lock();
        inner.pgid = pgid;
        0
        // 暂时挂起。因为“self”的类型是“Arc”，它不能作为可变引用借用。
    }
    // 获取进程组ID
    pub fn getpgid(&self) -> usize {
        let inner = self.acquire_inner_lock();
        inner.pgid
    }
    /// 获取用户空间的token
    pub fn get_user_token(&self) -> usize {
        self.vm.lock().token()
    }
}
```

### 5.3 TaskControlBlockInner

接着是任务控制块内部状态

```rust
/// 任务控制块内部状态
pub struct TaskControlBlockInner {
    /// 信号掩码
    pub sigmask: Signals,
    /// 待处理信号
    pub sigpending: Signals,
    /// 陷阱上下文的物理页号
    pub trap_cx_ppn: PhysPageNum,
    /// 任务上下文
    pub task_cx: TaskContext,
    /// 任务状态
    pub task_status: TaskStatus,
    /// 父进程
    pub parent: Option<Weak<TaskControlBlock>>,
    /// 子进程
    pub children: Vec<Arc<TaskControlBlock>>,
    /// 退出码
    pub exit_code: u32,
    /// 用于清理子进程的线程ID
    pub clear_child_tid: usize,
    /// 鲁棒列表，用于管理鲁棒互斥锁
    pub robust_list: RobustList,
    /// 堆底
    pub heap_bottom: usize,
    /// 堆页表
    pub heap_pt: usize,
    /// 进程组ID
    pub pgid: usize,
    /// 资源使用情况
    pub rusage: Rusage,
    /// 任务的时钟信息
    pub clock: ProcClock,
    /// 定时器
    pub timer: [ITimerVal; 3],
}
```

其方法：

```rust
impl TaskControlBlockInner {
    /// 获取陷阱上下文
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// 获取任务状态
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    /// 判断是否为僵尸态
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
    /// 添加信号
    pub fn add_signal(&mut self, signal: Signals) {
        self.sigpending.insert(signal);
    }
    /// 在进入陷阱时更新进程时间
    pub fn update_process_times_enter_trap(&mut self) {
        // 获取当前时间
        let now = TimeVal::now();
        // 更新上次进入内核态的时间
        self.clock.last_enter_s_mode = now;
        // 计算时间差
        let diff = now - self.clock.last_enter_u_mode;
        // 更新用户CPU时间
        self.rusage.ru_utime = self.rusage.ru_utime + diff;
        // 更新虚拟定时器
        self.update_itimer_virtual_if_exists(diff);
        // 更新性能分析定时器
        self.update_itimer_prof_if_exists(diff);
    }
    /// 在离开陷阱时更新进程时间
    #[cfg(feature = "loongarch64")]
    pub fn update_process_times_leave_trap(&mut self, trap_cause: TrapImpl) {
        let now = TimeVal::now();
        self.update_itimer_real_if_exists(now - self.clock.last_enter_u_mode);
        if trap_cause.is_timer() {
            let diff = now - self.clock.last_enter_s_mode;
            self.rusage.ru_stime = self.rusage.ru_stime + diff;
            self.update_itimer_prof_if_exists(diff);
        }
        self.clock.last_enter_u_mode = now;
    }
    #[cfg(feature = "riscv")]
    pub fn update_process_times_leave_trap(&mut self, scause: Trap) {
        let now = TimeVal::now();
        self.update_itimer_real_if_exists(now - self.clock.last_enter_u_mode);
        if scause != Trap::Interrupt(Interrupt::SupervisorTimer) {
            let diff = now - self.clock.last_enter_s_mode;
            self.rusage.ru_stime = self.rusage.ru_stime + diff;
            self.update_itimer_prof_if_exists(diff);
        }
        self.clock.last_enter_u_mode = now;
    }
    /// 更新实时定时器
    pub fn update_itimer_real_if_exists(&mut self, diff: TimeVal) {
        // 如果当前定时器不为0
        if !self.timer[0].it_value.is_zero() {
            // 更新定时器
            self.timer[0].it_value = self.timer[0].it_value - diff;
            // 如果定时器为0
            if self.timer[0].it_value.is_zero() {
                // 添加信号
                self.add_signal(Signals::SIGALRM);
                // 重置定时器
                self.timer[0].it_value = self.timer[0].it_interval;
            }
        }
    }
    /// 更新虚拟定时器
    /// 与上面的更新实时定时器类似
    /// 但是发送的信号是SIGVTALRM
    pub fn update_itimer_virtual_if_exists(&mut self, diff: TimeVal) {
        if !self.timer[1].it_value.is_zero() {
            self.timer[1].it_value = self.timer[1].it_value - diff;
            if self.timer[1].it_value.is_zero() {
                self.add_signal(Signals::SIGVTALRM);
                self.timer[1].it_value = self.timer[1].it_interval;
            }
        }
    }
    /// 更新性能分析定时器
    /// 与上面的更新实时定时器类似
    /// 但是发送的信号是SIGPROF
    pub fn update_itimer_prof_if_exists(&mut self, diff: TimeVal) {
        if !self.timer[2].it_value.is_zero() {
            self.timer[2].it_value = self.timer[2].it_value - diff;
            if self.timer[2].it_value.is_zero() {
                self.add_signal(Signals::SIGPROF);
                self.timer[2].it_value = self.timer[2].it_interval;
            }
        }
    }
}
```

### 5.4 RobustList

再接着是鲁棒列表`RobustList`

```rust
#[derive(Clone, Copy, Debug)]
/// 表示任务的鲁棒列表
/// 用于管理鲁棒互斥锁
pub struct RobustList {
    /// 链表头
    pub head: usize,
    /// 链表长度
    pub len: usize,
}

impl RobustList {
    // from strace
    // 默认的链表头大小
    pub const HEAD_SIZE: usize = 24;
}

impl Default for RobustList {
    /// 初始化方法
    fn default() -> Self {
        Self {
            // 链表头
            head: 0,
            // 链表长度
            len: Self::HEAD_SIZE,
        }
    }
}
```

### 5.5 ProcClock

再然后是进程时钟`ProcClock`

```rust
#[repr(C)]
/// 进程时钟
/// 表示任务的时钟信息
pub struct ProcClock {
    /// 上次进入用户态的时间
    last_enter_u_mode: TimeVal,
    /// 上次进入内核态的时间
    last_enter_s_mode: TimeVal,
}

impl ProcClock {
    /// 构造函数
    pub fn new() -> Self {
        // 获取当前时间
        let now = TimeVal::now();
        Self {
            last_enter_u_mode: now,
            last_enter_s_mode: now,
        }
    }
}
```

### 5.6 Rusage

再来就是资源使用情况`Rusage`

```rust
#[allow(unused)]
#[derive(Clone, Copy)]
#[repr(C)]
/// 资源使用情况
pub struct Rusage {
    /// 用户CPU时间
    pub ru_utime: TimeVal, /* user CPU time used */
    /// 系统CPU时间
    pub ru_stime: TimeVal, /* system CPU time used */
    /// 以下字段未实现，用于后续扩展
    ru_maxrss: isize, // NOT IMPLEMENTED /* maximum resident set size */
    ru_ixrss: isize,    // NOT IMPLEMENTED /* integral shared memory size */
    ru_idrss: isize,    // NOT IMPLEMENTED /* integral unshared data size */
    ru_isrss: isize,    // NOT IMPLEMENTED /* integral unshared stack size */
    ru_minflt: isize,   // NOT IMPLEMENTED /* page reclaims (soft page faults) */
    ru_majflt: isize,   // NOT IMPLEMENTED /* page faults (hard page faults) */
    ru_nswap: isize,    // NOT IMPLEMENTED /* swaps */
    ru_inblock: isize,  // NOT IMPLEMENTED /* block input operations */
    ru_oublock: isize,  // NOT IMPLEMENTED /* block output operations */
    ru_msgsnd: isize,   // NOT IMPLEMENTED /* IPC messages sent */
    ru_msgrcv: isize,   // NOT IMPLEMENTED /* IPC messages received */
    ru_nsignals: isize, // NOT IMPLEMENTED /* signals received */
    ru_nvcsw: isize,    // NOT IMPLEMENTED /* voluntary context switches */
    ru_nivcsw: isize,   // NOT IMPLEMENTED /* involuntary context switches */
}
```

其方法：

```rust
impl Rusage {
    /// 构造函数
    pub fn new() -> Self {
        Self {
            // 初始化为0
            ru_utime: TimeVal::new(),
            // 初始化为0
            ru_stime: TimeVal::new(),
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        }
    }
}
```

### 5.7 TaskStatus

最后是内容比较简单的`TaskStatus`

```rust
#[derive(Copy, Clone, PartialEq, Debug)]
/// 任务状态
pub enum TaskStatus {
    /// 就绪态
    Ready,
    /// 运行态
    Running,
    /// 僵尸态
    Zombie,
    /// 可中断态
    Interruptible,
}
```

## 六、signal

| 类型名称         | 作用 | 方法 |
| ---------------- | ---- | ---- |
| Signals          |      |      |
| SigActionFlags   |      |      |
| SigHandler       |      |      |
| SigAction        |      |      |
| SignalStackFlags |      |      |
| SignalStack      |      |      |
| SigMaskHow       |      |      |
| SigInfo          |      |      |

### 6.1 Signals

```rust
bitflags! {
    /// Signal
    pub struct Signals: signal_type!(){
        /// Hangup.
        const	SIGHUP		= 1 << ( 0);
        /// Interactive attention signal.
        const	SIGINT		= 1 << ( 1);
        /// Quit.
        const	SIGQUIT		= 1 << ( 2);
        /// Illegal instruction.
        const	SIGILL		= 1 << ( 3);
        /// Trace/breakpoint trap.
        const	SIGTRAP		= 1 << ( 4);
        /// IOT instruction, abort() on a PDP-11.
        const	SIGABRT		= 1 << ( 5);
        /// Bus error.
        const	SIGBUS		= 1 << ( 6);
        /// Erroneous arithmetic operation.
        const	SIGFPE		= 1 << ( 7);
        /// Killed.
        const	SIGKILL		= 1 << ( 8);
        /// User-defined signal 1.
        const	SIGUSR1		= 1 << ( 9);
        /// Invalid access to storage.
        const	SIGSEGV		= 1 << (10);
        /// User-defined signal 2.
        const	SIGUSR2		= 1 << (11);
        /// Broken pipe.
        const	SIGPIPE		= 1 << (12);
        /// Alarm clock.
        const	SIGALRM		= 1 << (13);
        /// Termination request.
        const	SIGTERM		= 1 << (14);
        const	SIGSTKFLT	= 1 << (15);
        /// Child terminated or stopped.
        const	SIGCHLD		= 1 << (16);
        /// Continue.
        const	SIGCONT		= 1 << (17);
        /// Stop, unblockable.
        const	SIGSTOP		= 1 << (18);
        /// Keyboard stop.
        const	SIGTSTP		= 1 << (19);
        /// Background read from control terminal.
        const	SIGTTIN		= 1 << (20);
        /// Background write to control terminal.
        const	SIGTTOU		= 1 << (21);
        /// Urgent data is available at a socket.
        const	SIGURG		= 1 << (22);
        /// CPU time limit exceeded.
        const	SIGXCPU		= 1 << (23);
        /// File size limit exceeded.
        const	SIGXFSZ		= 1 << (24);
        /// Virtual timer expired.
        const	SIGVTALRM	= 1 << (25);
        /// Profiling timer expired.
        const	SIGPROF		= 1 << (26);
        /// Window size change (4.3 BSD, Sun).
        const	SIGWINCH	= 1 << (27);
        /// I/O now possible (4.2 BSD).
        const	SIGIO		= 1 << (28);
        const   SIGPWR      = 1 << (29);
        /// Bad system call.
        const   SIGSYS      = 1 << (30);
        /* --- realtime signals for pthread --- */
        const   SIGTIMER    = 1 << (31);
        const   SIGCANCEL   = 1 << (32);
        const   SIGSYNCCALL = 1 << (33);
        /* --- other realtime signals --- */
        const   SIGRT_3     = 1 << (34);
        const   SIGRT_4     = 1 << (35);
        const   SIGRT_5     = 1 << (36);
        const   SIGRT_6     = 1 << (37);
        const   SIGRT_7     = 1 << (38);
        const   SIGRT_8     = 1 << (39);
        const   SIGRT_9     = 1 << (40);
        const   SIGRT_10    = 1 << (41);
        const   SIGRT_11    = 1 << (42);
        const   SIGRT_12    = 1 << (43);
        const   SIGRT_13    = 1 << (44);
        const   SIGRT_14    = 1 << (45);
        const   SIGRT_15    = 1 << (46);
        const   SIGRT_16    = 1 << (47);
        const   SIGRT_17    = 1 << (48);
        const   SIGRT_18    = 1 << (49);
        const   SIGRT_19    = 1 << (50);
        const   SIGRT_20    = 1 << (51);
        const   SIGRT_21    = 1 << (52);
        const   SIGRT_22    = 1 << (53);
        const   SIGRT_23    = 1 << (54);
        const   SIGRT_24    = 1 << (55);
        const   SIGRT_25    = 1 << (56);
        const   SIGRT_26    = 1 << (57);
        const   SIGRT_27    = 1 << (58);
        const   SIGRT_28    = 1 << (59);
        const   SIGRT_29    = 1 << (60);
        const   SIGRT_30    = 1 << (61);
        const   SIGRT_31    = 1 << (62);
        const   SIGRTMAX    = 1 << (63);
    }
}
```

### 6.2 SigActionFlags

### 6.3 SigHandler

### 6.4 SignalStackFlags

### 6.5 SignalStack

### 6.6 SigMaskHow

### 6.7 SigInfo

## 七、processor

不同点：

+ RISCV有一个unused的current_kstack_top()，而LA版本没有，不管这个函数
+ LA有一个is_vacant函数，实现很简单，判断Option是否为空，无需特殊处理

### 7.1 Processor

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

## 八、pid

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

## 九、mod

## 十、导图
