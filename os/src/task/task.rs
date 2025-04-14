use super::manager::TASK_MANAGER;
use super::pid::RecycleAllocator;
use super::signal::*;
use super::threads::Futex;
use super::TaskContext;
use super::{pid_alloc, PidHandle};
use crate::config::MMAP_BASE;
use crate::fs::file_descriptor::FdTable;
use crate::fs::{FileDescriptor, OpenFlags, ROOT_FD};
use crate::hal::trap_cx_bottom_from_tid;
use crate::hal::ustack_bottom_from_tid;
use crate::hal::TrapImpl;
use crate::hal::{kstack_alloc, KernelStack};
use crate::hal::{trap_handler, TrapContext};
use crate::mm::PageTableImpl;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::net::SocketTable;
use crate::syscall::CloneFlags;
use crate::timer::{ITimerVal, TimeVal};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use log::trace;
use spin::{Mutex, MutexGuard};

#[derive(Clone)]
/// 任务的文件系统状态
pub struct FsStatus {
    /// 当前工作目录的文件描述符
    pub working_inode: Arc<FileDescriptor>,
}

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
    pub vm: Arc<Mutex<MemorySet<PageTableImpl>>>,
    /// 信号处理函数表
    pub sighand: Arc<Mutex<Vec<Option<Box<SigAction>>>>>,
    /// 快速用户空间互斥锁
    pub futex: Arc<Mutex<Futex>>,
}

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

impl Debug for Rusage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "(ru_utime:{:?}, ru_stime:{:?})",
            self.ru_utime, self.ru_stime
        ))
    }
}

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
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap();
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
        inner.trap_cx_ppn = (&memory_set)
            .translate(VirtAddr::from(self.trap_cx_user_va()).into())
            .unwrap();
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
        let trap_cx_ppn = memory_set
            .lock()
            .translate(VirtAddr::from(trap_cx_bottom_from_tid(tid)).into())
            .unwrap();

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

impl Drop for TaskControlBlock {
    /// 当任务控制块被销毁时，释放线程ID
    fn drop(&mut self) {
        self.tid_allocator.lock().dealloc(self.tid);
    }
}

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
