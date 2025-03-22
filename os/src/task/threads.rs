/*
    此文件内容用于
    内容与RISCV版本相同，无需修改
*/
use crate::{syscall::errno::*, task::current_task, timer::TimeSpec};
use alloc::{collections::BTreeMap, sync::Arc};
use log::*;
use num_enum::FromPrimitive;

use super::{
    block_current_and_run_next,
    manager::{wait_with_timeout, WaitQueue},
};

#[allow(unused)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
/// 定义了Futex支持的操作类型
pub enum FutexCmd {
    /// This  operation  tests  that  the value at the futex
    /// word pointed to by the address uaddr still  contains
    /// the expected value val, and if so, then sleeps wait‐
    /// ing for a FUTEX_WAKE operation on  the  futex  word.
    /// The load of the value of the futex word is an atomic
    /// memory access (i.e., using atomic  machine  instruc‐
    /// tions  of  the respective architecture).  This load,
    /// the comparison with the expected value, and starting
    /// to  sleep  are  performed atomically and totally or‐
    /// dered with respect to other futex operations on  the
    /// same  futex word.  If the thread starts to sleep, it
    /// is considered a waiter on this futex word.   If  the
    /// futex  value does not match val, then the call fails
    /// immediately with the error EAGAIN.
    Wait = 0,
    /// This operation wakes at most val of the waiters that
    /// are waiting (e.g., inside FUTEX_WAIT) on  the  futex
    /// word  at  the  address uaddr.  Most commonly, val is
    /// specified as either 1 (wake up a single  waiter)  or
    /// INT_MAX (wake up all waiters).  No guarantee is pro‐
    /// vided about which waiters are awoken (e.g., a waiter
    /// with  a higher scheduling priority is not guaranteed
    /// to be awoken in preference to a waiter with a  lower
    /// priority).
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

/// 原作者注释：目前 `rt_clk` 被忽略
/// ---
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

// Futex的方法实现
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
