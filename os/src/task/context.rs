/*
    此文件用于定义任务上下文结构体 TaskContext
    TaskContext 结构体包含了任务的上下文信息，包括返回地址 ra、栈指针 sp 和通用寄存器 s
    内容与RISCV版本相同，无需修改
*/
use crate::hal::trap_return;

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

impl TaskContext {
    // 空初始化
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    // 从指定栈指针和返回地址初始化
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
